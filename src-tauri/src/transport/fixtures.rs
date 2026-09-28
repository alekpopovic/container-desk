//! Synthetic normalized records, deliberately not a claim to parse Docker CLI output yet.
use super::{ReadTransport, SnapshotFuture};
use crate::{
    domain::*,
    policy::registry::{CommandPlan, ResponseKind},
};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const MAX_RECORD_BYTES: usize = 256 * 1024;
const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
const STANDARD: &str = include_str!("../../fixtures/demo/containers.jsonl");
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    id: ContainerId,
    name: String,
    image: String,
    state: String,
    status: String,
    health: Option<String>,
    ports: Vec<ContainerPort>,
    labels: BTreeMap<String, String>,
}
fn invalid(scope: &SessionScope) -> AppError {
    AppError::new(ErrorCode::InvalidResponse).in_scope(scope)
}
/// Parses only the documented demo record schema. The native Docker adapter is a later prompt.
pub fn parse(scope: &SessionScope, bytes: &[u8]) -> Result<ListContainersResponse, AppError> {
    scope.validate()?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(AppError::new(ErrorCode::ResourceLimit).in_scope(scope));
    }
    let mut containers = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for line in bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
    {
        if line.len() > MAX_RECORD_BYTES || containers.len() >= 50000 {
            return Err(AppError::new(ErrorCode::ResourceLimit).in_scope(scope));
        }
        let record: Record = serde_json::from_slice(line).map_err(|_| invalid(scope))?;
        record.id.validate().map_err(|_| invalid(scope))?;
        if !ids.insert(record.id.clone())
            || [&record.name, &record.image, &record.state, &record.status]
                .iter()
                .any(|s| s.len() > 4096)
            || record.health.as_ref().is_some_and(|s| s.len() > 256)
            || record.ports.len() > 128
            || record.labels.len() > 64
        {
            return Err(invalid(scope));
        }
        for port in &record.ports {
            if port.private_port == 0
                || port.public_port == Some(0)
                || !["tcp", "udp", "sctp"].contains(&port.protocol.as_str())
                || port
                    .host_ip
                    .as_ref()
                    .is_some_and(|ip| ip.parse::<std::net::IpAddr>().is_err())
            {
                return Err(invalid(scope));
            }
        }
        let compose = record
            .labels
            .get("com.docker.compose.project")
            .map(|project| ComposeLabels {
                project: project.clone(),
                service: record.labels.get("com.docker.compose.service").cloned(),
            });
        if compose.as_ref().is_some_and(|c| {
            c.project.len() > 256 || c.service.as_ref().is_some_and(|s| s.len() > 256)
        }) {
            return Err(invalid(scope));
        }
        containers.push(ContainerSummary {
            scope: scope.clone(),
            id: record.id,
            name: record.name,
            image: record.image,
            state: record.state,
            status: record.status,
            health: record.health,
            ports: record.ports,
            compose,
        });
    }
    Ok(ListContainersResponse {
        scope: scope.clone(),
        containers,
    })
}

pub struct FixtureTransport {
    scenario: DemoScenario,
}
impl FixtureTransport {
    pub fn new(scenario: DemoScenario) -> Self {
        Self { scenario }
    }
}
impl ReadTransport for FixtureTransport {
    fn list_containers(&self, scope: SessionScope, command: CommandPlan) -> SnapshotFuture<'_> {
        Box::pin(async move {
            if command.response() != &ResponseKind::ContainerList {
                return Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&scope));
            }
            match self.scenario {
                DemoScenario::Standard => parse(&scope, STANDARD.as_bytes()),
                DemoScenario::Empty => parse(&scope, b"\n \n"),
                DemoScenario::InvalidJson => parse(&scope, b"{broken fixture"),
                DemoScenario::HugeRecord => parse(&scope, &vec![b'x'; MAX_RECORD_BYTES + 1]),
                DemoScenario::PermissionFailure => {
                    Err(AppError::new(ErrorCode::PermissionDenied).in_scope(&scope))
                }
                DemoScenario::Disconnect => {
                    Err(AppError::new(ErrorCode::Disconnected).in_scope(&scope))
                }
                DemoScenario::Timeout => {
                    Err(AppError::new(ErrorCode::OperationTimedOut).in_scope(&scope))
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        contract_tests::scope,
        policy::registry::{self, ReadOperation},
    };
    #[test]
    fn normalized_records_retain_states_ipv6_and_only_compose_labels() {
        let response = parse(&scope(), STANDARD.as_bytes()).unwrap();
        assert_eq!(response.containers.len(), 4);
        assert_eq!(response.containers[0].health.as_deref(), Some("healthy"));
        assert_eq!(response.containers[1].state, "exited");
        assert_eq!(response.containers[2].state, "restarting");
        assert_eq!(response.containers[3].health.as_deref(), Some("unhealthy"));
        assert_eq!(
            response.containers[0].ports[0].host_ip.as_deref(),
            Some("::1")
        );
        assert_eq!(
            response.containers[0].compose.as_ref().unwrap().project,
            "demo-stack"
        );
        assert!(
            !serde_json::to_string(&response)
                .unwrap()
                .contains("SYNTHETIC_VALUE_NOT_FOR_IPC")
        );
        assert!(response.containers.iter().all(|row| row.scope == scope()));
    }
    #[tokio::test]
    async fn fixture_failures_and_empty_data_are_deterministic_and_distinct() {
        let command = || registry::read(&ReadOperation::ListContainers).unwrap();
        assert!(
            FixtureTransport::new(DemoScenario::Empty)
                .list_containers(scope(), command())
                .await
                .unwrap()
                .containers
                .is_empty()
        );
        for (scenario, code) in [
            (DemoScenario::PermissionFailure, ErrorCode::PermissionDenied),
            (DemoScenario::InvalidJson, ErrorCode::InvalidResponse),
            (DemoScenario::HugeRecord, ErrorCode::ResourceLimit),
            (DemoScenario::Disconnect, ErrorCode::Disconnected),
            (DemoScenario::Timeout, ErrorCode::OperationTimedOut),
        ] {
            assert_eq!(
                FixtureTransport::new(scenario)
                    .list_containers(scope(), command())
                    .await
                    .unwrap_err()
                    .code,
                code
            );
        }
    }
    #[test]
    fn malformed_ports_duplicate_ids_and_oversized_records_are_rejected() {
        let first = STANDARD.lines().next().unwrap();
        assert_eq!(
            parse(&scope(), format!("{first}\n{first}").as_bytes())
                .unwrap_err()
                .code,
            ErrorCode::InvalidResponse
        );
        let mut record: serde_json::Value = serde_json::from_str(first).unwrap();
        record["ports"][0]["publicPort"] = (-1).into();
        assert_eq!(
            parse(&scope(), &serde_json::to_vec(&record).unwrap())
                .unwrap_err()
                .code,
            ErrorCode::InvalidResponse
        );
        assert_eq!(
            parse(&scope(), &vec![b'x'; MAX_RECORD_BYTES + 1])
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
    }
}
