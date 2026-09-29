//! Log-specific bounded retention; ACK credit and child ownership are shared with events.
use super::logs::{channel, size};
pub(crate) use crate::ssh::subscriptions::Subscriptions;
use crate::{
    domain::*,
    ssh::{
        runner::{Stream, streaming::LineSink},
        subscriptions::{Delivery, Queue, Request},
    },
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
pub(crate) type BatchSink = Arc<dyn Fn(LogBatch) -> Result<(), ()> + Send + Sync>;
const BATCH_BYTES: usize = 256 * 1024 + 30;
const BATCH_LINES: usize = 128;
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
        while self.bytes > crate::resource_limits::current().log_bytes as usize
            || self.records.len() > crate::resource_limits::current().log_lines as usize
        {
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

impl Queue for LogQueue {
    type Record = LogRecord;
    fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    fn batch(&mut self) -> (Vec<LogRecord>, u32) {
        LogQueue::batch(self)
    }
}
impl Request for FollowLogsRequest {
    type Record = LogRecord;
    type Batch = LogBatch;
    fn scope(&self) -> &SessionScope {
        &self.scope
    }
    fn initial_gap(&self) -> bool {
        self.since.is_some()
    }
    fn make_batch(&self, d: Delivery<LogRecord>) -> LogBatch {
        LogBatch {
            scope: self.scope.clone(),
            container_id: self.container_id.clone(),
            subscription_id: d.subscription_id,
            sequence: d.sequence,
            records: d.records,
            dropped_records: d.dropped_records,
            gap: d.gap,
            ended: d.ended,
            error: d.error,
        }
    }
}
#[cfg(test)]
mod tests;
