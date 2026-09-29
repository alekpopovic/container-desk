//! Bounded local activity, not a tamper-proof audit log or a replay queue.
use crate::{
    domain::*,
    policy::{PolicyEngine, registry::CommandPlan},
    storage::{FileStorage, Storage},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_RECORDS: usize = 200;
const MAX_ACTIVE: usize = 3;
const MAX_BYTES: usize = 512 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ActivityOutcome {
    NotDispatched,
    Unknown,
    Succeeded,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ActivityRecord {
    pub id: IntentId,
    pub host_id: HostId,
    pub action: MutationOperation,
    pub targets: Vec<ContainerId>,
    #[cfg_attr(test, ts(type = "number"))]
    pub started_at_ms: u64,
    #[cfg_attr(test, ts(type = "number"))]
    pub updated_at_ms: u64,
    pub outcome: ActivityOutcome,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct History {
    schema_version: u32,
    records: Vec<ActivityRecord>,
}
struct State {
    adapter: Box<dyn Storage>,
    history: History,
    active: HashSet<HostId>,
    persistence_failed: bool,
}
#[derive(Clone)]
pub struct Activities {
    state: Arc<Mutex<State>>,
}
/// Owns a host's mutation slot through transport completion/reaping, even across reconnect.
/// Dropping it releases only the lock: the durable pre-dispatch outcome remains Unknown.
pub struct Operation {
    state: Arc<Mutex<State>>,
    host: HostId,
    id: IntentId,
    plan: Option<CommandPlan>,
}
fn storage_error() -> AppError {
    AppError::new(ErrorCode::StorageUnavailable)
}
fn now_ms() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| d.as_millis().try_into().ok())
        .ok_or_else(|| AppError::new(ErrorCode::Internal))
}
impl State {
    fn persist(&mut self, next: History) -> Result<(), AppError> {
        let bytes = serde_json::to_vec(&next).map_err(|_| storage_error())?;
        if bytes.len() > MAX_BYTES || self.adapter.commit(&bytes, None).is_err() {
            // A rename may already have succeeded before a directory fsync error.
            // Never authorize another dispatch after an uncertain persistence failure.
            self.persistence_failed = true;
            return Err(storage_error());
        }
        self.history = next;
        Ok(())
    }
}
impl Activities {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Self::load(Box::new(
            FileStorage::open_activity(path).map_err(|_| storage_error())?,
        ))
    }
    pub(crate) fn load(adapter: Box<dyn Storage>) -> Result<Self, AppError> {
        let bytes = adapter.read(false).map_err(|_| storage_error())?;
        let history: History = match bytes {
            Some(bytes) if bytes.len() <= MAX_BYTES => {
                serde_json::from_slice(&bytes).map_err(|_| storage_error())?
            }
            Some(_) => return Err(storage_error()),
            None => History {
                schema_version: 1,
                records: vec![],
            },
        };
        if history.schema_version != 1 || history.records.len() > MAX_RECORDS {
            return Err(storage_error());
        }
        let mut seen = HashSet::new();
        for record in &history.records {
            record.id.validate().map_err(|_| storage_error())?;
            record.host_id.validate().map_err(|_| storage_error())?;
            if !seen.insert(&record.id)
                || record.targets.is_empty()
                || record.targets.len() > 20
                || record.updated_at_ms > 253_402_300_799_999
                || record.updated_at_ms < record.started_at_ms
            {
                return Err(storage_error());
            }
            let mut targets = HashSet::new();
            for id in &record.targets {
                id.validate().map_err(|_| storage_error())?;
                if !targets.insert(id) {
                    return Err(storage_error());
                }
            }
        }
        // Unknown records are displayed as stored, never submitted or automatically reconciled.
        Ok(Self {
            state: Arc::new(Mutex::new(State {
                adapter,
                history,
                active: HashSet::new(),
                persistence_failed: false,
            })),
        })
    }
    pub fn records(&self) -> Result<Vec<ActivityRecord>, AppError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| storage_error())?
            .history
            .records
            .clone())
    }
    pub fn begin(
        &self,
        policy: &mut PolicyEngine,
        request: MutationRequest,
    ) -> Result<Operation, AppError> {
        let authorized = policy.consume(
            &request.scope,
            &request.intent_id,
            &ConfirmationOperation::Mutation(request.spec.clone()),
        )?;
        let mut state = self.state.lock().map_err(|_| storage_error())?;
        if state.persistence_failed {
            return Err(storage_error());
        }
        let host = request.scope.selection.host_id;
        if state.active.contains(&host) || state.active.len() >= MAX_ACTIVE {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
        let now = now_ms()?;
        let mut next = state.history.clone();
        if next.records.len() == MAX_RECORDS {
            // A running operation's record must survive unrelated completed activity.
            let index = next
                .records
                .iter()
                .position(|r| !state.active.contains(&r.host_id))
                .ok_or_else(|| AppError::new(ErrorCode::ResourceLimit))?;
            next.records.remove(index);
        }
        next.records.push(ActivityRecord {
            id: request.intent_id.clone(),
            host_id: host.clone(),
            action: request.spec.operation,
            targets: request.spec.container_ids,
            started_at_ms: now,
            updated_at_ms: now,
            outcome: ActivityOutcome::NotDispatched,
        });
        state.persist(next)?;
        state.active.insert(host.clone());
        Ok(Operation {
            state: self.state.clone(),
            host,
            id: request.intent_id,
            plan: Some(authorized.into_plan()),
        })
    }
}
impl Operation {
    /// Persist Unknown BEFORE allowing the one-use command to leave this owner.
    /// A crash between this write and spawn is conservatively unknown, never a retry signal.
    pub fn dispatch(&mut self) -> Result<CommandPlan, AppError> {
        if self.plan.is_none() {
            return Err(AppError::new(ErrorCode::InvalidIntent));
        }
        self.update(ActivityOutcome::Unknown)?;
        Ok(self.plan.take().expect("checked one-use plan"))
    }
    fn update(&self, outcome: ActivityOutcome) -> Result<(), AppError> {
        let mut state = self.state.lock().map_err(|_| storage_error())?;
        if state.persistence_failed {
            return Err(storage_error());
        }
        let mut next = state.history.clone();
        let record = next
            .records
            .iter_mut()
            .find(|r| r.id == self.id)
            .ok_or_else(storage_error)?;
        record.updated_at_ms = now_ms()?.max(record.started_at_ms);
        record.outcome = outcome;
        state.persist(next)
    }
    /// Called only after the owning transport has completed and reaped its child.
    /// Transport loss/cancellation/timeout must pass Unknown, never be retried.
    pub fn finish(self, outcome: MutationOutcome) -> Result<(), AppError> {
        if self.plan.is_some() {
            return Err(AppError::new(ErrorCode::InvalidIntent));
        }
        self.update(match outcome {
            MutationOutcome::Succeeded => ActivityOutcome::Succeeded,
            MutationOutcome::Failed => ActivityOutcome::Failed,
            MutationOutcome::Unknown => ActivityOutcome::Unknown,
        })
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.active.remove(&self.host);
        }
    }
}

#[cfg(test)]
mod tests;
