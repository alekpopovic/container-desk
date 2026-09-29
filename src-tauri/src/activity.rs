//! Bounded local activity, not a tamper-proof audit log or a replay queue.
use crate::{
    domain::*,
    policy::{
        PolicyEngine,
        registry::{self, CommandPlan},
    },
    storage::{FileStorage, Storage},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
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
    Partial,
    Cancelled,
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
    #[serde(default)]
    pub results: Vec<MutationTargetResult>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct History {
    schema_version: u32,
    records: Vec<ActivityRecord>,
}
struct Active {
    scope: SessionScope,
    id: IntentId,
    cancelled: Arc<AtomicBool>,
}
struct State {
    adapter: Box<dyn Storage>,
    history: History,
    active: HashMap<HostId, Active>,
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
    spec: MutationSpec,
    compose: Option<ComposeActionSpec>,
    project_dispatched: bool,
    cursor: usize,
    inflight: Option<ContainerId>,
    cancelled: Arc<AtomicBool>,
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
pub(crate) fn aggregate(results: &[MutationTargetResult]) -> ActivityOutcome {
    if results
        .iter()
        .any(|r| r.outcome == MutationTargetOutcome::Unknown)
    {
        ActivityOutcome::Unknown
    } else if results
        .iter()
        .all(|r| r.outcome == MutationTargetOutcome::Succeeded)
    {
        ActivityOutcome::Succeeded
    } else if results
        .iter()
        .all(|r| r.outcome == MutationTargetOutcome::Failed)
    {
        ActivityOutcome::Failed
    } else if results
        .iter()
        .all(|r| r.outcome == MutationTargetOutcome::Cancelled)
    {
        ActivityOutcome::Cancelled
    } else if results
        .iter()
        .all(|r| r.outcome == MutationTargetOutcome::NotDispatched)
    {
        ActivityOutcome::NotDispatched
    } else {
        ActivityOutcome::Partial
    }
}
pub(crate) fn response_outcome(results: &[MutationTargetResult]) -> MutationOutcome {
    match aggregate(results) {
        ActivityOutcome::Unknown => MutationOutcome::Unknown,
        ActivityOutcome::Succeeded => MutationOutcome::Succeeded,
        ActivityOutcome::Failed => MutationOutcome::Failed,
        ActivityOutcome::Partial => MutationOutcome::Partial,
        ActivityOutcome::Cancelled | ActivityOutcome::NotDispatched => MutationOutcome::Cancelled,
    }
}
impl State {
    fn persist(&mut self, mut next: History) -> Result<(), AppError> {
        let bytes = loop {
            let bytes = serde_json::to_vec(&next).map_err(|_| storage_error())?;
            if bytes.len() <= MAX_BYTES && next.records.len() <= MAX_RECORDS {
                break bytes;
            }
            let index = next
                .records
                .iter()
                .enumerate()
                .position(|(i, r)| {
                    i + 1 < next.records.len()
                        && !self.active.values().any(|active| active.id == r.id)
                })
                .ok_or_else(|| AppError::new(ErrorCode::ResourceLimit))?;
            next.records.remove(index);
        };
        if self.adapter.commit(&bytes, None).is_err() {
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
        let mut history: History = match bytes {
            Some(bytes) if bytes.len() <= MAX_BYTES => {
                serde_json::from_slice(&bytes).map_err(|_| storage_error())?
            }
            Some(_) => return Err(storage_error()),
            None => History {
                schema_version: 2,
                records: vec![],
            },
        };
        if ![1, 2].contains(&history.schema_version) || history.records.len() > MAX_RECORDS {
            return Err(storage_error());
        }
        let mut seen = HashSet::new();
        for record in &mut history.records {
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
            if history.schema_version == 1 {
                record.results = record
                    .targets
                    .iter()
                    .map(|id| MutationTargetResult {
                        container_id: id.clone(),
                        outcome: match record.outcome {
                            ActivityOutcome::NotDispatched => MutationTargetOutcome::NotDispatched,
                            ActivityOutcome::Succeeded if record.targets.len() == 1 => {
                                MutationTargetOutcome::Succeeded
                            }
                            ActivityOutcome::Failed if record.targets.len() == 1 => {
                                MutationTargetOutcome::Failed
                            }
                            _ => MutationTargetOutcome::Unknown,
                        },
                        dispatched: record.outcome != ActivityOutcome::NotDispatched,
                        error: None,
                    })
                    .collect();
                record.outcome = aggregate(&record.results);
            }
            if record.results.len() != record.targets.len()
                || record.outcome != aggregate(&record.results)
                || record.results.iter().zip(&record.targets).any(|(r, id)| {
                    &r.container_id != id
                        || match r.outcome {
                            MutationTargetOutcome::NotDispatched
                            | MutationTargetOutcome::Cancelled => r.dispatched,
                            MutationTargetOutcome::Succeeded | MutationTargetOutcome::Unknown => {
                                !r.dispatched
                            }
                            MutationTargetOutcome::Failed => false,
                        }
                })
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
        history.schema_version = 2;
        // Unknown records are displayed as stored, never submitted or automatically reconciled.
        Ok(Self {
            state: Arc::new(Mutex::new(State {
                adapter,
                history,
                active: HashMap::new(),
                persistence_failed: false,
            })),
        })
    }
    pub fn clear(&self) -> Result<(), AppError> {
        let mut state = self.state.lock().map_err(|_| storage_error())?;
        if !state.active.is_empty() {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
        state.persist(History {
            schema_version: 2,
            records: vec![],
        })?;
        state.persistence_failed = false;
        Ok(())
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
        self.begin_operation(
            policy,
            request.clone(),
            ConfirmationOperation::Mutation(request.spec.clone()),
            None,
        )
    }
    pub(crate) fn begin_compose(
        &self,
        policy: &mut PolicyEngine,
        request: ComposeMutationRequest,
    ) -> Result<Operation, AppError> {
        let mutation = MutationRequest {
            scope: request.scope,
            intent_id: request.intent_id,
            spec: MutationSpec {
                operation: request.spec.operation.mutation(),
                container_ids: request.spec.container_ids.clone(),
                timeout_seconds: request.spec.timeout_seconds,
            },
        };
        self.begin_operation(
            policy,
            mutation,
            ConfirmationOperation::Compose(request.spec.clone()),
            Some(request.spec),
        )
    }
    fn begin_operation(
        &self,
        policy: &mut PolicyEngine,
        request: MutationRequest,
        confirmation: ConfirmationOperation,
        compose: Option<ComposeActionSpec>,
    ) -> Result<Operation, AppError> {
        let _authorized = policy.consume(&request.scope, &request.intent_id, &confirmation)?;
        let mut state = self.state.lock().map_err(|_| storage_error())?;
        if state.persistence_failed {
            return Err(storage_error());
        }
        let host = request.scope.selection.host_id.clone();
        if state.active.contains_key(&host) || state.active.len() >= MAX_ACTIVE {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
        if state
            .history
            .records
            .iter()
            .any(|r| r.id == request.intent_id)
        {
            return Err(AppError::new(ErrorCode::InvalidIntent));
        }
        let now = now_ms()?;
        let mut next = state.history.clone();
        let results = request
            .spec
            .container_ids
            .iter()
            .map(|id| MutationTargetResult {
                container_id: id.clone(),
                outcome: MutationTargetOutcome::NotDispatched,
                dispatched: false,
                error: None,
            })
            .collect();
        next.records.push(ActivityRecord {
            id: request.intent_id.clone(),
            host_id: host.clone(),
            action: request.spec.operation.clone(),
            targets: request.spec.container_ids.clone(),
            started_at_ms: now,
            updated_at_ms: now,
            outcome: ActivityOutcome::NotDispatched,
            results,
        });
        state.persist(next)?;
        let cancelled = Arc::new(AtomicBool::new(false));
        state.active.insert(
            host.clone(),
            Active {
                scope: request.scope,
                id: request.intent_id.clone(),
                cancelled: cancelled.clone(),
            },
        );
        Ok(Operation {
            state: self.state.clone(),
            host,
            id: request.intent_id,
            spec: request.spec,
            compose,
            project_dispatched: false,
            cursor: 0,
            inflight: None,
            cancelled,
        })
    }
    pub fn cancel(
        &self,
        request: &CancelMutationRequest,
    ) -> Result<CancelMutationResponse, AppError> {
        request.scope.validate()?;
        request.intent_id.validate()?;
        let state = self.state.lock().map_err(|_| storage_error())?;
        let accepted = state
            .active
            .get(&request.scope.selection.host_id)
            .is_some_and(|active| {
                if active.scope == request.scope && active.id == request.intent_id {
                    active.cancelled.store(true, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            });
        Ok(CancelMutationResponse {
            pending_cancellation_requested: accepted,
        })
    }
}
impl Operation {
    pub(crate) fn dispatch_compose(&mut self) -> Result<CommandPlan, AppError> {
        let spec = self
            .compose
            .as_ref()
            .ok_or_else(|| AppError::new(ErrorCode::InvalidIntent))?;
        if self.project_dispatched || self.cursor != 0 || self.inflight.is_some() {
            return Err(AppError::new(ErrorCode::InvalidIntent));
        }
        if self.cancelled() {
            return Err(AppError::new(ErrorCode::OperationCancelled));
        }
        let plan = registry::confirmation(&ConfirmationOperation::Compose(spec.clone()))?;
        self.update_project(MutationTargetOutcome::Unknown, true, None)?;
        self.project_dispatched = true;
        Ok(plan)
    }
    pub(crate) fn complete_compose(
        &mut self,
        outcome: MutationTargetOutcome,
        dispatched: bool,
        error: Option<ErrorCode>,
    ) -> Result<(), AppError> {
        if self.compose.is_none()
            || self.cursor != 0
            || self.project_dispatched != dispatched
            || (dispatched
                && !matches!(
                    outcome,
                    MutationTargetOutcome::Succeeded | MutationTargetOutcome::Unknown
                ))
            || (!dispatched
                && !matches!(
                    outcome,
                    MutationTargetOutcome::Failed | MutationTargetOutcome::Cancelled
                ))
        {
            return Err(AppError::new(ErrorCode::InvalidIntent));
        }
        self.update_project(outcome, dispatched, error)?;
        self.cursor = self.spec.container_ids.len();
        Ok(())
    }
    fn update_project(
        &self,
        outcome: MutationTargetOutcome,
        dispatched: bool,
        error: Option<ErrorCode>,
    ) -> Result<(), AppError> {
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
        record.results = self
            .spec
            .container_ids
            .iter()
            .map(|id| MutationTargetResult {
                container_id: id.clone(),
                outcome: outcome.clone(),
                dispatched,
                error: error.clone(),
            })
            .collect();
        record.updated_at_ms = now_ms()?.max(record.started_at_ms);
        record.outcome = aggregate(&record.results);
        state.persist(next)
    }
    pub fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    fn require_next(&self, id: &ContainerId) -> Result<(), AppError> {
        if self.inflight.is_some() || self.spec.container_ids.get(self.cursor) != Some(id) {
            return Err(AppError::new(ErrorCode::InvalidIntent));
        }
        Ok(())
    }
    pub fn dispatch_target(&mut self, id: &ContainerId) -> Result<CommandPlan, AppError> {
        if self.compose.is_some() {
            return Err(AppError::new(ErrorCode::InvalidIntent));
        }
        self.require_next(id)?;
        if self.cancelled() {
            return Err(AppError::new(ErrorCode::OperationCancelled));
        }
        let plan = registry::confirmation(&ConfirmationOperation::Mutation(MutationSpec {
            operation: self.spec.operation.clone(),
            container_ids: vec![id.clone()],
            timeout_seconds: self.spec.timeout_seconds,
        }))?;
        self.update_target(MutationTargetResult {
            container_id: id.clone(),
            outcome: MutationTargetOutcome::Unknown,
            dispatched: true,
            error: None,
        })?;
        self.inflight = Some(id.clone());
        Ok(plan)
    }
    pub fn dispatch(&mut self) -> Result<CommandPlan, AppError> {
        if self.spec.container_ids.len() != 1 {
            return Err(AppError::new(ErrorCode::InvalidLimits));
        }
        self.dispatch_target(&self.spec.container_ids[0].clone())
    }
    fn update_target(&self, result: MutationTargetResult) -> Result<(), AppError> {
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
        let target = record
            .results
            .iter_mut()
            .find(|r| r.container_id == result.container_id)
            .ok_or_else(storage_error)?;
        *target = result;
        record.updated_at_ms = now_ms()?.max(record.started_at_ms);
        record.outcome = aggregate(&record.results);
        state.persist(next)
    }
    /// Called only after the dispatched child's result/reaping, never on a renderer timer.
    pub fn complete_target(&mut self, result: MutationTargetResult) -> Result<(), AppError> {
        if result.dispatched {
            if self.inflight.as_ref() != Some(&result.container_id)
                || matches!(
                    result.outcome,
                    MutationTargetOutcome::NotDispatched | MutationTargetOutcome::Cancelled
                )
            {
                return Err(AppError::new(ErrorCode::InvalidIntent));
            }
        } else {
            self.require_next(&result.container_id)?;
            if matches!(
                result.outcome,
                MutationTargetOutcome::Succeeded | MutationTargetOutcome::Unknown
            ) {
                return Err(AppError::new(ErrorCode::InvalidIntent));
            }
        }
        self.update_target(result)?;
        self.inflight = None;
        self.cursor += 1;
        Ok(())
    }
    pub fn results(&self) -> Result<Vec<MutationTargetResult>, AppError> {
        let state = self.state.lock().map_err(|_| storage_error())?;
        state
            .history
            .records
            .iter()
            .find(|r| r.id == self.id)
            .map(|r| r.results.clone())
            .ok_or_else(storage_error)
    }
    pub fn finish(mut self, outcome: MutationOutcome) -> Result<(), AppError> {
        let id = self
            .inflight
            .clone()
            .ok_or_else(|| AppError::new(ErrorCode::InvalidIntent))?;
        let outcome = match outcome {
            MutationOutcome::Succeeded => MutationTargetOutcome::Succeeded,
            MutationOutcome::Failed => MutationTargetOutcome::Failed,
            MutationOutcome::Unknown => MutationTargetOutcome::Unknown,
            _ => return Err(AppError::new(ErrorCode::InvalidIntent)),
        };
        self.complete_target(MutationTargetResult {
            container_id: id,
            outcome,
            dispatched: true,
            error: None,
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
