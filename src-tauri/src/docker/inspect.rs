//! Narrow inspect projection; no raw JSON, command, health output or environment enters diagnostics.
use crate::{
    domain::*,
    policy::registry::{self, ReadOperation},
    ssh::{
        multiplex::Client,
        runner::{Captured, Limits, RunError},
    },
};
use serde::Deserialize;
use std::{collections::BTreeMap, time::Duration};
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    id: ContainerId,
    name: Option<String>,
    created: Option<String>,
    image: Option<ImageId>,
    restart_count: Option<u32>,
    config: Option<Config>,
    state: Option<State>,
    host_config: Option<Host>,
    mounts: Option<Vec<Mount>>,
    network_settings: Option<Networks>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Config {
    image: Option<String>,
    env: Option<Vec<String>>,
    labels: Option<BTreeMap<String, String>>,
    healthcheck: Option<Healthcheck>,
    exposed_ports: Option<BTreeMap<String, Option<Empty>>>,
}
#[derive(Deserialize)]
struct Empty {}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Healthcheck {
    test: Option<Vec<String>>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct State {
    status: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    exit_code: Option<i32>,
    health: Option<Health>,
    #[serde(rename = "OOMKilled")]
    oom_killed: Option<bool>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Health {
    status: Option<String>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Host {
    restart_policy: Option<Restart>,
    memory: Option<i64>,
    memory_swap: Option<i64>,
    nano_cpus: Option<i64>,
    cpu_shares: Option<i64>,
    cpu_period: Option<i64>,
    cpu_quota: Option<i64>,
    cpuset_cpus: Option<String>,
    pids_limit: Option<i64>,
    privileged: Option<bool>,
    readonly_rootfs: Option<bool>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Restart {
    name: Option<String>,
    maximum_retry_count: Option<u32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Mount {
    #[serde(rename = "Type")]
    kind: Option<String>,
    name: Option<String>,
    source: Option<String>,
    destination: Option<String>,
    #[serde(rename = "RW")]
    rw: Option<bool>,
    propagation: Option<String>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Networks {
    networks: Option<BTreeMap<String, Option<Network>>>,
    ports: Option<BTreeMap<String, Option<Vec<Port>>>>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Network {
    #[serde(rename = "NetworkID")]
    network_id: Option<String>,
    #[serde(rename = "IPAddress")]
    ip_address: Option<String>,
    #[serde(rename = "GlobalIPv6Address")]
    global_ipv6_address: Option<String>,
    gateway: Option<String>,
    mac_address: Option<String>,
    aliases: Option<Vec<String>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Port {
    host_ip: Option<String>,
    host_port: String,
}
fn err(scope: &SessionScope, code: ErrorCode) -> AppError {
    AppError::new(code).in_scope(scope)
}
fn small(scope: &SessionScope, value: Option<String>) -> Result<Option<String>, AppError> {
    if value.as_ref().is_some_and(|s| s.len() > 4096) {
        return Err(err(scope, ErrorCode::ResourceLimit));
    }
    Ok(value.filter(|s| !s.is_empty()))
}
fn count(scope: &SessionScope, actual: usize, max: usize) -> Result<(), AppError> {
    if actual > max {
        Err(err(scope, ErrorCode::ResourceLimit))
    } else {
        Ok(())
    }
}
fn detail_value(
    scope: &SessionScope,
    name: String,
    value: String,
    reveal: bool,
) -> Result<DetailValue, AppError> {
    count(scope, name.len(), 4096)?;
    count(scope, value.len(), 64 * 1024)?;
    Ok(DetailValue {
        name,
        value: reveal.then_some(value),
        masked: !reveal,
    })
}
/// All label values are masked conservatively: arbitrary label names cannot prove a value public.
pub fn parse(
    scope: &SessionScope,
    id: &ContainerId,
    reveal: bool,
    bytes: &[u8],
) -> Result<ContainerDetail, AppError> {
    scope.validate()?;
    id.validate()?;
    count(scope, bytes.len(), MAX_BYTES)?;
    let mut records: Vec<Record> =
        serde_json::from_slice(bytes).map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
    if records.is_empty() {
        return Err(err(scope, ErrorCode::ContainerNotFound));
    }
    if records.len() != 1 {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    let r = records.pop().unwrap();
    if r.id != *id {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    if r.image.as_ref().is_some_and(|id| id.validate().is_err()) {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    let healthcheck_configured = r
        .config
        .as_ref()
        .and_then(|config| match &config.healthcheck {
            None => Some(false),
            Some(check) => {
                check
                    .test
                    .as_ref()
                    .and_then(|test| match test.first().map(String::as_str) {
                        Some("NONE") => Some(false),
                        Some("CMD" | "CMD-SHELL") => Some(true),
                        _ => None,
                    })
            }
        });
    let config = r.config.unwrap_or_default();
    let state = r.state.unwrap_or_default();
    let host = r.host_config.unwrap_or_default();
    let restart = host.restart_policy.unwrap_or_default();
    let env = config.env.unwrap_or_default();
    let labels = config.labels.unwrap_or_default();
    count(scope, env.len(), 1024)?;
    count(scope, labels.len(), 1024)?;
    let mut environment = Vec::new();
    for item in env {
        let (name, value) = item
            .split_once('=')
            .ok_or_else(|| err(scope, ErrorCode::InvalidResponse))?;
        if name.is_empty() {
            return Err(err(scope, ErrorCode::InvalidResponse));
        }
        environment.push(detail_value(scope, name.into(), value.into(), reveal)?);
    }
    let compose = super::compose::group_labels(&labels);
    let labels = labels
        .into_iter()
        .map(|(k, v)| detail_value(scope, k, v, reveal))
        .collect::<Result<Vec<_>, _>>()?;
    let raw_mounts = r.mounts.unwrap_or_default();
    count(scope, raw_mounts.len(), 128)?;
    let mounts = raw_mounts
        .into_iter()
        .map(|m| {
            Ok(DetailMount {
                kind: small(scope, m.kind)?,
                name: small(scope, m.name)?,
                source: small(scope, m.source)?,
                destination: small(scope, m.destination)?,
                read_write: m.rw,
                propagation: small(scope, m.propagation)?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let raw_network = r.network_settings.unwrap_or_default();
    let network_map = raw_network.networks.unwrap_or_default();
    count(scope, network_map.len(), 128)?;
    let mut networks = Vec::new();
    for (name, n) in network_map {
        let n = n.unwrap_or_default();
        let aliases = n.aliases.unwrap_or_default();
        count(scope, aliases.len(), 128)?;
        for alias in &aliases {
            count(scope, alias.len(), 4096)?;
        }
        networks.push(DetailNetwork {
            name: small(scope, Some(name))?.unwrap_or_default(),
            network_id: small(scope, n.network_id)?,
            ipv4: small(scope, n.ip_address)?,
            ipv6: small(scope, n.global_ipv6_address)?,
            gateway: small(scope, n.gateway)?,
            mac_address: small(scope, n.mac_address)?,
            aliases,
        });
    }
    let exposed = config.exposed_ports.unwrap_or_default();
    count(scope, exposed.len(), 128)?;
    let exposed_ports = exposed
        .keys()
        .map(|key| {
            let (private_port, protocol) = port_key(scope, key)?;
            Ok(ExposedPort {
                private_port,
                protocol: protocol.into(),
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let mut ports = Vec::new();
    let port_map = raw_network.ports.unwrap_or_default();
    count(scope, port_map.len(), 128)?;
    for (key, bindings) in port_map {
        let (private_port, protocol) = port_key(scope, &key)?;
        let bindings = bindings.unwrap_or_default();
        count(scope, bindings.len(), 128)?;
        if bindings.is_empty() {
            ports.push(ContainerPort {
                private_port,
                public_port: None,
                host_ip: None,
                protocol: protocol.into(),
            });
        }
        for b in bindings {
            let public_port = b
                .host_port
                .parse::<u16>()
                .ok()
                .filter(|n| *n > 0)
                .ok_or_else(|| err(scope, ErrorCode::InvalidResponse))?;
            if b.host_ip.as_ref().is_some_and(|s| s.len() > 64) {
                return Err(err(scope, ErrorCode::ResourceLimit));
            }
            ports.push(ContainerPort {
                private_port,
                public_port: Some(public_port),
                host_ip: b.host_ip,
                protocol: protocol.into(),
            });
        }
        count(scope, ports.len(), 128)?;
    }
    let status = small(scope, state.status)?.unwrap_or_else(|| "unknown".into());
    Ok(ContainerDetail {
        healthcheck_configured,
        oom_killed: state.oom_killed,
        exposed_ports,
        summary: ContainerSummary {
            scope: scope.clone(),
            id: id.clone(),
            name: small(scope, r.name)?
                .map(|s| s.trim_start_matches('/').into())
                .unwrap_or_else(|| id.0.clone()),
            image: small(scope, config.image)?.unwrap_or_else(|| "Unknown".into()),
            state: status.clone(),
            status,
            health: small(scope, state.health.and_then(|h| h.status))?,
            ports,
            compose,
            cli: None,
        },
        environment_names: environment.iter().map(|e| e.name.clone()).collect(),
        environment_values_masked: !reveal,
        environment,
        labels,
        created_at: small(scope, r.created)?,
        started_at: small(scope, state.started_at)?.filter(|s| !s.starts_with("0001-01-01")),
        finished_at: small(scope, state.finished_at)?.filter(|s| !s.starts_with("0001-01-01")),
        exit_code: state.exit_code,
        restart_count: r.restart_count,
        restart_policy: small(scope, restart.name)?,
        restart_maximum_retry_count: restart.maximum_retry_count,
        image_id: r.image,
        mounts,
        networks,
        resources: ResourceConfiguration {
            memory_bytes: host.memory.map(|v| v.to_string()),
            memory_swap_bytes: host.memory_swap.map(|v| v.to_string()),
            nano_cpus: host.nano_cpus.map(|v| v.to_string()),
            cpu_shares: host.cpu_shares.map(|v| v.to_string()),
            cpu_period: host.cpu_period.map(|v| v.to_string()),
            cpu_quota: host.cpu_quota.map(|v| v.to_string()),
            cpuset_cpus: small(scope, host.cpuset_cpus)?,
            pids_limit: host.pids_limit.map(|v| v.to_string()),
            privileged: host.privileged,
            read_only_rootfs: host.readonly_rootfs,
        },
    })
}
fn port_key<'a>(scope: &SessionScope, key: &'a str) -> Result<(u16, &'a str), AppError> {
    let (number, protocol) = key
        .split_once('/')
        .ok_or_else(|| err(scope, ErrorCode::InvalidResponse))?;
    let port = number
        .parse::<u16>()
        .ok()
        .filter(|p| *p > 0)
        .ok_or_else(|| err(scope, ErrorCode::InvalidResponse))?;
    if !["tcp", "udp", "sctp"].contains(&protocol) {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    Ok((port, protocol))
}
fn decode(
    scope: &SessionScope,
    id: &ContainerId,
    reveal: bool,
    output: Captured,
) -> Result<ContainerDetail, AppError> {
    if !output.status.success() {
        let expected = [
            format!("Error: No such container: {}", id.0),
            format!("Error response from daemon: No such container: {}", id.0),
        ];
        let missing = output.status.code() == Some(1)
            && std::str::from_utf8(&output.stderr)
                .is_ok_and(|s| expected.iter().any(|message| s.trim() == message));
        return Err(err(
            scope,
            if output.status.code() == Some(255) {
                ErrorCode::Disconnected
            } else if missing {
                ErrorCode::ContainerNotFound
            } else {
                ErrorCode::TransportUnavailable
            },
        ));
    }
    parse(scope, id, reveal, &output.stdout)
}
pub(crate) async fn read(
    connection: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: &InspectContainerRequest,
) -> Result<ContainerDetail, AppError> {
    let scope = &request.scope;
    scope.validate()?;
    request.container_id.validate()?;
    tokio::time::timeout(Duration::from_secs(30), async {
        let operation = ReadOperation::InspectContainer {
            container_id: request.container_id.clone(),
        };
        let (fresh, _) = super::probe::run(connection, options).await;
        super::probe::transport_ready(&fresh)?;
        if fresh.daemon_id.as_deref() != Some(scope.daemon_id.as_str()) {
            return Err(err(scope, ErrorCode::StaleSession));
        }
        let command = binding
            .prepare(registry::read(&operation)?, &fresh)
            .map_err(|e| e.in_scope(scope))?;
        let output = connection
            .start_fixed(
                command.encoded().into(),
                Limits {
                    deadline: Duration::from_secs(30),
                    stdout_bytes: MAX_BYTES,
                    stderr_bytes: 16 * 1024,
                },
            )
            .map_err(|e| e.in_scope(scope))?
            .wait()
            .await
            .map_err(|e| {
                err(
                    scope,
                    match e {
                        RunError::TimedOut => ErrorCode::OperationTimedOut,
                        RunError::OutputLimit(_) => ErrorCode::ResourceLimit,
                        RunError::Cancelled => ErrorCode::OperationCancelled,
                        _ => ErrorCode::TransportUnavailable,
                    },
                )
            })?;
        let parsed = decode(
            scope,
            &request.container_id,
            request.reveal_sensitive,
            output,
        )?;
        let (after, _) = super::probe::run(connection, options).await;
        binding
            .prepare(registry::read(&operation)?, &after)
            .map_err(|e| e.in_scope(scope))?;
        Ok(parsed)
    })
    .await
    .map_err(|_| err(scope, ErrorCode::OperationTimedOut))?
}
#[cfg(test)]
mod tests;
