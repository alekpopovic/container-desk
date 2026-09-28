//! Replaceable read transport. Live and synthetic providers never fall back to each other.
pub mod fixtures;
use crate::{domain::*, policy::registry::CommandPlan};
use std::{future::Future, pin::Pin, sync::Arc};

pub type SnapshotFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ListContainersResponse, AppError>> + Send + 'a>>;
pub trait ReadTransport: Send + Sync {
    fn list_containers(&self, scope: SessionScope, command: CommandPlan) -> SnapshotFuture<'_>;
}
pub struct UnavailableLiveTransport;
impl ReadTransport for UnavailableLiveTransport {
    fn list_containers(&self, scope: SessionScope, _command: CommandPlan) -> SnapshotFuture<'_> {
        Box::pin(async move { Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&scope)) })
    }
}
pub struct WorkspaceTransport {
    pub snapshot: WorkspaceModeSnapshot,
    pub generation: u32,
    pub live: Arc<dyn ReadTransport>,
    pub active: Arc<dyn ReadTransport>,
}
impl Default for WorkspaceTransport {
    fn default() -> Self {
        let live: Arc<dyn ReadTransport> = Arc::new(UnavailableLiveTransport);
        Self {
            snapshot: WorkspaceModeSnapshot {
                mode: WorkspaceMode::Live,
                scenario: None,
                scope: None,
                host: None,
            },
            generation: 0,
            active: live.clone(),
            live,
        }
    }
}
pub fn demo_host() -> HostSummary {
    HostSummary {
        id: HostId(format!("h_{:032x}", 0xd00800)),
        alias: "demo-local".into(),
        display_name: "Demo Linux host".into(),
        group: "Demo".into(),
        read_only: true,
        connection_state: ConnectionState::Ready,
    }
}
