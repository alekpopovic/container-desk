//! One acknowledged Tauri batch in flight, bounded oldest-drop retention, owned cancellation.
use super::logs::{MAX_RECORDS, MAX_RETAINED_BYTES, channel, size};
use crate::{
    domain::*,
    ssh::runner::{Job, RunError, Stream, streaming::LineSink},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::Duration,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, watch};
pub(crate) type BatchSink = Arc<dyn Fn(LogBatch) -> Result<(), ()> + Send + Sync>;
type ScopeCheck = Arc<dyn Fn() -> Result<(), AppError> + Send + Sync>;
const BATCH_BYTES: usize = 256 * 1024 + 30;
const BATCH_LINES: usize = 128;
const ACK_TIMEOUT: Duration = Duration::from_secs(30);
#[derive(Default)]
pub(crate) struct LogQueue {
    records: VecDeque<LogRecord>,
    bytes: usize,
    dropped: u32,
}
impl LogQueue {
    fn push(&mut self, record: LogRecord) {
        self.bytes += size(&record);
        self.records.push_back(record);
        while self.bytes > MAX_RETAINED_BYTES || self.records.len() > MAX_RECORDS {
            if let Some(row) = self.records.pop_front() {
                self.bytes -= size(&row);
                self.dropped = self.dropped.saturating_add(1);
            }
        }
    }
    fn batch(&mut self) -> (Vec<LogRecord>, u32) {
        let mut rows = Vec::new();
        let mut bytes = 0;
        while let Some(front) = self.records.front() {
            if rows.len() == BATCH_LINES || bytes + size(front) > BATCH_BYTES {
                break;
            }
            let row = self.records.pop_front().expect("front exists");
            bytes += size(&row);
            rows.push(row);
        }
        self.bytes -= bytes;
        (rows, std::mem::take(&mut self.dropped))
    }
}
pub(crate) fn line_sink(queue: Arc<Mutex<LogQueue>>) -> LineSink {
    Arc::new(move |stream, mut bytes, truncated| {
        bytes.push(b'\n');
        let origin = match stream {
            Stream::Stdout => LogChannel::Stdout,
            Stream::Stderr => LogChannel::StderrAmbiguous,
        };
        let (mut rows, _) = channel(&bytes, origin);
        if let Some(mut row) = rows.pop_front() {
            row.truncated |= truncated;
            if let Ok(mut queue) = queue.lock() {
                queue.push(row);
            }
        }
    })
}
struct Entry {
    scope: SessionScope,
    cancel: watch::Sender<bool>,
    done: watch::Receiver<bool>,
    ack: mpsc::Sender<u32>,
    expected: AtomicU32,
    waiting: AtomicBool,
}
#[derive(Clone)]
pub(crate) struct Subscriptions {
    entries: Arc<Mutex<HashMap<SubscriptionId, Arc<Entry>>>>,
    slots: Arc<Semaphore>,
}
impl Default for Subscriptions {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            slots: Arc::new(Semaphore::new(2)),
        }
    }
}
impl Subscriptions {
    pub fn reserve(&self) -> Result<OwnedSemaphorePermit, AppError> {
        self.slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))
    }
    fn entry(&self, scope: &SessionScope, id: &SubscriptionId) -> Result<Arc<Entry>, AppError> {
        scope.validate()?;
        id.validate()?;
        self.entries
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .get(id)
            .filter(|e| e.scope == *scope)
            .cloned()
            .ok_or_else(|| AppError::new(ErrorCode::SubscriptionNotFound).in_scope(scope))
    }
    pub fn ack(&self, request: &AckLogsRequest) -> Result<(), AppError> {
        let entry = self.entry(&request.scope, &request.subscription_id)?;
        if entry.expected.load(Ordering::SeqCst) != request.sequence
            || entry
                .waiting
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
        {
            return Err(AppError::new(ErrorCode::InvalidResponse).in_scope(&request.scope));
        }
        entry
            .ack
            .try_send(request.sequence)
            .map_err(|_| AppError::new(ErrorCode::SubscriptionNotFound).in_scope(&request.scope))
    }
    pub async fn cancel(&self, request: &CancelSubscriptionRequest) -> Result<(), AppError> {
        let entry = self.entry(&request.scope, &request.subscription_id)?;
        let mut done = entry.done.clone();
        entry.cancel.send_replace(true);
        while !*done.borrow() {
            if done.changed().await.is_err() {
                break;
            }
        }
        Ok(())
    }
    pub async fn shutdown(&self) {
        let entries: Vec<_> = self
            .entries
            .lock()
            .map(|e| e.values().cloned().collect())
            .unwrap_or_default();
        for e in &entries {
            e.cancel.send_replace(true);
        }
        for e in entries {
            let mut done = e.done.clone();
            while !*done.borrow() {
                if done.changed().await.is_err() {
                    break;
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        &self,
        request: FollowLogsRequest,
        mut job: Job,
        queue: Arc<Mutex<LogQueue>>,
        sink: BatchSink,
        current: ScopeCheck,
        permits: (OwnedSemaphorePermit, OwnedSemaphorePermit),
    ) -> Result<CancelSubscriptionResponse, AppError> {
        let mut random = [0; 16];
        getrandom::fill(&mut random).map_err(|_| AppError::new(ErrorCode::Internal))?;
        let id = SubscriptionId(format!(
            "sub_{}",
            random
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ));
        let (cancel, mut stopped) = watch::channel(false);
        let (done, finished) = watch::channel(false);
        let (ack, mut receiver) = mpsc::channel(1);
        let entry = Arc::new(Entry {
            scope: request.scope.clone(),
            cancel,
            done: finished,
            ack,
            expected: AtomicU32::new(0),
            waiting: AtomicBool::new(true),
        });
        self.entries
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .insert(id.clone(), entry.clone());
        let response = CancelSubscriptionResponse {
            scope: request.scope.clone(),
            subscription_id: id.clone(),
        };
        let entries = self.entries.clone();
        tokio::spawn(async move {
            let _permits = permits;
            let mut tick = tokio::time::interval(Duration::from_millis(100));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut sent_at = tokio::time::Instant::now();
            let mut credit = false;
            let mut sequence = 0u32;
            let mut outcome: Option<Option<ErrorCode>> = None;
            let mut initial_gap = request.since.is_some();
            loop {
                tokio::select! {
                    biased;
                    _ = stopped.changed() => break,
                    result = job.finished(), if outcome.is_none() => {
                        outcome = Some(match result {
                            Ok(output) if output.status.success() => None,
                            Ok(_) => Some(ErrorCode::TransportUnavailable),
                            Err(RunError::Cancelled) => Some(ErrorCode::Disconnected),
                            Err(_) => Some(ErrorCode::TransportUnavailable),
                        });
                    }
                    ack = receiver.recv() => { if ack.is_none() { break; } credit = true; }
                    _ = tick.tick() => {
                        if *stopped.borrow() || current().is_err() { break; }
                        if !credit {
                            if sent_at.elapsed() >= ACK_TIMEOUT { break; }
                            continue;
                        }
                        let Ok(mut retained) = queue.lock() else { break; };
                        if retained.records.is_empty() && outcome.is_none() && sent_at.elapsed() < Duration::from_secs(1) { continue; }
                        let (records, dropped_records) = retained.batch();
                        let ended = outcome.is_some() && retained.records.is_empty();
                        drop(retained);
                        let Some(next) = sequence.checked_add(1) else { break; };
                        sequence = next;
                        entry.expected.store(sequence, Ordering::SeqCst);
                        entry.waiting.store(true, Ordering::SeqCst);
                        let batch = LogBatch { scope: request.scope.clone(), container_id: request.container_id.clone(), subscription_id: id.clone(), sequence, records, dropped_records, gap: std::mem::take(&mut initial_gap) || dropped_records > 0 || outcome.as_ref().is_some_and(Option::is_some), ended, error: outcome.clone().flatten() };
                        credit = false;
                        sent_at = tokio::time::Instant::now();
                        if sink(batch).is_err() || ended { break; }
                    }
                }
            }
            if outcome.is_none() {
                job.cancel();
                let _ = job.finished().await;
            }
            // Permit release and direct-child reaping precede cancellation acknowledgment.
            drop(_permits);
            if let Ok(mut entries) = entries.lock() {
                entries.remove(&id);
            }
            done.send_replace(true);
        });
        Ok(response)
    }
}

#[cfg(test)]
mod tests;
