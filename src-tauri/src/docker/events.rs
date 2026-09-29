//! Event hints only. Never forward Actor.Attributes, labels, raw output or diagnostics.
use crate::{
    domain::*,
    ssh::{
        runner::{Stream, streaming::LineSink},
        subscriptions::{Delivery, Queue, Request},
    },
};
use serde::Deserialize;
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Mutex},
};
pub(crate) type BatchSink = Arc<dyn Fn(EventBatch) -> Result<(), ()> + Send + Sync>;
const MAX_EVENTS: usize = 512;
const MAX_RECENT: usize = 1024;
const BATCH_EVENTS: usize = 64;
#[derive(Deserialize)]
struct Actor {
    #[serde(rename = "ID")]
    id: ContainerId,
}
#[derive(Deserialize)]
struct Record {
    #[serde(rename = "Type")]
    kind: String,
    #[serde(rename = "Action")]
    action: String,
    #[serde(rename = "Actor")]
    actor: Actor,
    time: Option<u64>,
    #[serde(rename = "timeNano")]
    nanos: Option<u64>,
}
pub(crate) fn parse(bytes: &[u8]) -> Result<Option<ContainerEvent>, ()> {
    if bytes.len() > 16 * 1024 {
        return Err(());
    }
    let r: Record = serde_json::from_slice(bytes).map_err(|_| ())?;
    if r.kind != "container" {
        return Ok(None);
    }
    r.actor.id.validate().map_err(|_| ())?;
    let timestamp = r
        .nanos
        .or_else(|| r.time.and_then(|n| n.checked_mul(1_000_000_000)))
        .ok_or(())?;
    if r.time.is_some_and(|s| s != timestamp / 1_000_000_000) {
        return Err(());
    }
    use ContainerEventAction::*;
    let action = match r.action.as_str() {
        "create" => Create,
        "start" => Start,
        "stop" => Stop,
        "die" => Die,
        "destroy" => Destroy,
        "restart" => Restart,
        "pause" => Pause,
        "unpause" => Unpause,
        "rename" => Rename,
        "health_status: healthy" | "health_status: unhealthy" | "health_status: starting" => {
            HealthStatus
        }
        _ => return Ok(None),
    };
    Ok(Some(ContainerEvent {
        actor_id: r.actor.id,
        action,
        timestamp_unix_nanos: timestamp.to_string(),
    }))
}
#[derive(Default)]
pub(crate) struct EventQueue {
    events: VecDeque<ContainerEvent>,
    recent: VecDeque<ContainerEvent>,
    seen: HashSet<ContainerEvent>,
    dropped: u32,
}
impl EventQueue {
    fn push(&mut self, event: ContainerEvent) {
        if !self.seen.insert(event.clone()) {
            return;
        }
        self.recent.push_back(event.clone());
        while self.recent.len() > MAX_RECENT {
            if let Some(old) = self.recent.pop_front() {
                self.seen.remove(&old);
            }
        }
        self.events.push_back(event);
        while self.events.len() > MAX_EVENTS {
            self.events.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
    }
}
impl Queue for EventQueue {
    type Record = ContainerEvent;
    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
    fn batch(&mut self) -> (Vec<ContainerEvent>, u32) {
        (
            self.events
                .drain(..self.events.len().min(BATCH_EVENTS))
                .collect(),
            std::mem::take(&mut self.dropped),
        )
    }
}
pub(crate) fn line_sink(queue: Arc<Mutex<EventQueue>>) -> LineSink {
    Arc::new(move |stream, bytes, truncated| {
        let parsed = if stream == Stream::Stdout && !truncated {
            parse(&bytes)
        } else {
            Err(())
        };
        if let Ok(mut queue) = queue.lock() {
            match parsed {
                Ok(Some(event)) => queue.push(event),
                Ok(None) => {}
                Err(()) => queue.dropped = queue.dropped.saturating_add(1),
            }
        }
    })
}
impl Request for FollowEventsRequest {
    type Record = ContainerEvent;
    type Batch = EventBatch;
    fn scope(&self) -> &SessionScope {
        &self.scope
    }
    fn initial_gap(&self) -> bool {
        true
    }
    fn make_batch(&self, d: Delivery<ContainerEvent>) -> EventBatch {
        EventBatch {
            scope: self.scope.clone(),
            subscription_id: d.subscription_id,
            sequence: d.sequence,
            events: d.records,
            dropped_records: d.dropped_records,
            gap: d.gap,
            ended: d.ended,
            error: d.error,
        }
    }
}
#[cfg(test)]
mod tests;
