//! One finite selected-container sample. CLI strings are approximate measurements, never API counters.
pub(crate) use crate::ssh::read_limits::Slots;
use crate::{
    domain::*,
    policy::registry::{self, ReadOperation},
    ssh::{
        multiplex::Client,
        runner::{Captured, Limits, RunError},
    },
};
use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct Record {
    #[serde(rename = "ID")]
    id: ContainerId,
    #[serde(rename = "CPUPerc")]
    cpu: Option<String>,
    #[serde(rename = "MemUsage")]
    memory: Option<String>,
    #[serde(rename = "MemPerc")]
    memory_percent: Option<String>,
    #[serde(rename = "NetIO")]
    network: Option<String>,
    #[serde(rename = "BlockIO")]
    block: Option<String>,
    #[serde(rename = "PIDs")]
    pids: Option<String>,
}
fn numeric(value: &str) -> Option<f64> {
    if value.is_empty()
        || !value.bytes().all(|c| c.is_ascii_digit() || c == b'.')
        || !value.bytes().any(|c| c.is_ascii_digit())
    {
        return None;
    }
    value
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n >= 0.0)
}
fn percent(value: Option<&str>) -> Option<f64> {
    numeric(value?.trim().strip_suffix('%')?).filter(|n| *n <= 1_000_000.0)
}
fn bytes(value: &str) -> Option<f64> {
    let value = value.trim();
    let cut = value.find(|c: char| !c.is_ascii_digit() && c != '.')?;
    let number = numeric(&value[..cut])?;
    let multiplier = match value[cut..].trim() {
        "B" => 1.0,
        "kB" | "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        "PB" => 1e15,
        "EB" => 1e18,
        "KiB" => 1024.0,
        "MiB" => 1024_f64.powi(2),
        "GiB" => 1024_f64.powi(3),
        "TiB" => 1024_f64.powi(4),
        "PiB" => 1024_f64.powi(5),
        "EiB" => 1024_f64.powi(6),
        _ => return None,
    };
    let result = number * multiplier;
    (result.is_finite() && result <= 9_007_199_254_740_991.0).then_some(result)
}
fn pair(value: Option<&str>) -> (Option<f64>, Option<f64>) {
    value
        .and_then(|s| s.split_once('/'))
        .map(|(a, b)| (bytes(a), bytes(b)))
        .unwrap_or_default()
}
fn empty(request: &ContainerStatsRequest, availability: StatsAvailability) -> StatsSample {
    StatsSample {
        scope: request.scope.clone(),
        container_id: request.container_id.clone(),
        captured_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
            .mul_add(1000.0, 0.0)
            .floor(),
        availability,
        values: StatsValues::default(),
        raw: StatsRaw::default(),
    }
}
pub(crate) fn parse(
    request: &ContainerStatsRequest,
    output: &[u8],
) -> Result<StatsSample, AppError> {
    request.scope.validate()?;
    request.container_id.validate()?;
    if output.len() > 16 * 1024 {
        return Err(AppError::new(ErrorCode::ResourceLimit));
    }
    if output.is_empty() {
        return Ok(empty(request, StatsAvailability::Unavailable));
    }
    let record: Record =
        serde_json::from_slice(output).map_err(|_| AppError::new(ErrorCode::InvalidResponse))?;
    if record.id != request.container_id {
        return Err(AppError::new(ErrorCode::InvalidResponse));
    }
    for value in [
        &record.cpu,
        &record.memory,
        &record.memory_percent,
        &record.network,
        &record.block,
        &record.pids,
    ]
    .into_iter()
    .flatten()
    {
        if value.len() > 128 || value.chars().any(char::is_control) {
            return Err(AppError::new(ErrorCode::InvalidResponse));
        }
    }
    let (memory_usage_bytes, memory_limit_bytes) = pair(record.memory.as_deref());
    let known_memory = memory_limit_bytes.is_some_and(|n| n > 0.0);
    let (network_rx_bytes, network_tx_bytes) = pair(record.network.as_deref());
    let (block_read_bytes, block_write_bytes) = pair(record.block.as_deref());
    let values = StatsValues {
        cpu_percent: percent(record.cpu.as_deref()),
        memory_usage_bytes: memory_usage_bytes.filter(|_| known_memory),
        memory_limit_bytes: memory_limit_bytes.filter(|_| known_memory),
        memory_percent: percent(record.memory_percent.as_deref()).filter(|_| known_memory),
        network_rx_bytes,
        network_tx_bytes,
        block_read_bytes,
        block_write_bytes,
        pids: record
            .pids
            .as_deref()
            .and_then(|s| s.trim().parse::<u32>().ok()),
    };
    let mut sample = empty(
        request,
        if values == StatsValues::default() {
            StatsAvailability::Unavailable
        } else {
            StatsAvailability::Available
        },
    );
    sample.values = values;
    sample.raw = StatsRaw {
        cpu: record.cpu,
        memory: record.memory,
        memory_percent: record.memory_percent,
        network: record.network,
        block: record.block,
        pids: record.pids,
    };
    Ok(sample)
}
fn check_exit(id: &ContainerId, output: &Captured) -> Result<(), AppError> {
    if output.status.success() {
        return Ok(());
    }
    let missing = output.status.code() == Some(1)
        && std::str::from_utf8(&output.stderr).is_ok_and(|s| {
            [
                format!("Error: No such container: {}", id.0),
                format!("Error response from daemon: No such container: {}", id.0),
                format!("Error: No such object: {}", id.0),
            ]
            .iter()
            .any(|m| s.trim() == m)
        });
    Err(AppError::new(if missing {
        ErrorCode::ContainerNotFound
    } else if output.status.code() == Some(255) {
        ErrorCode::Disconnected
    } else {
        ErrorCode::TransportUnavailable
    }))
}
#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct State {
    running: bool,
    started_at: String,
}
async fn execute(
    client: &Client,
    command: super::PreparedCommand,
    id: &ContainerId,
) -> Result<Captured, AppError> {
    let output = client
        .start_fixed(
            command.encoded().into(),
            Limits {
                deadline: Duration::from_secs(30),
                stdout_bytes: 16 * 1024,
                stderr_bytes: 16 * 1024,
            },
        )?
        .wait()
        .await
        .map_err(|e| {
            AppError::new(match e {
                RunError::TimedOut => ErrorCode::OperationTimedOut,
                RunError::OutputLimit(_) => ErrorCode::ResourceLimit,
                RunError::Cancelled => ErrorCode::OperationCancelled,
                _ => ErrorCode::TransportUnavailable,
            })
        })?;
    check_exit(id, &output)?;
    Ok(output)
}
fn state(output: &Captured) -> Result<State, AppError> {
    let state: State = serde_json::from_slice(&output.stdout)
        .map_err(|_| AppError::new(ErrorCode::InvalidResponse))?;
    if state.started_at.len() > 128 || state.started_at.chars().any(char::is_control) {
        return Err(AppError::new(ErrorCode::InvalidResponse));
    }
    Ok(state)
}
pub(crate) async fn read(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: &ContainerStatsRequest,
) -> Result<StatsSample, AppError> {
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        let (fresh, _) = super::probe::run(client, options).await;
        super::probe::transport_ready(&fresh)?;
        if fresh.daemon_id.as_deref() != Some(request.scope.daemon_id.as_str()) {
            return Err(AppError::new(ErrorCode::StaleSession));
        }
        let state_op = ReadOperation::StatsState {
            container_id: request.container_id.clone(),
        };
        let stats_op = ReadOperation::ContainerStats {
            container_id: request.container_id.clone(),
        };
        let before = execute(
            client,
            binding.prepare(registry::read(&state_op)?, &fresh)?,
            &request.container_id,
        )
        .await;
        let mut sample = match before {
            Err(e) if e.code == ErrorCode::ContainerNotFound => {
                empty(request, StatsAvailability::Missing)
            }
            Err(e) => return Err(e),
            Ok(output) => {
                let before = state(&output)?;
                if !before.running {
                    empty(request, StatsAvailability::Stopped)
                } else {
                    let output = execute(
                        client,
                        binding.prepare(registry::read(&stats_op)?, &fresh)?,
                        &request.container_id,
                    )
                    .await;
                    match output {
                        Err(e) if e.code == ErrorCode::ContainerNotFound => {
                            empty(request, StatsAvailability::Missing)
                        }
                        Err(e) => return Err(e),
                        Ok(output) => {
                            let parsed = parse(request, &output.stdout)?;
                            match execute(
                                client,
                                binding.prepare(registry::read(&state_op)?, &fresh)?,
                                &request.container_id,
                            )
                            .await
                            {
                                Err(e) if e.code == ErrorCode::ContainerNotFound => {
                                    empty(request, StatsAvailability::Missing)
                                }
                                Err(e) => return Err(e),
                                Ok(output) => {
                                    let after = state(&output)?;
                                    if !after.running {
                                        empty(request, StatsAvailability::Stopped)
                                    } else if before != after {
                                        empty(request, StatsAvailability::Unavailable)
                                    } else {
                                        parsed
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
        let (after, _) = super::probe::run(client, options).await;
        binding.prepare(registry::read(&stats_op)?, &after)?;
        sample.captured_at_ms = empty(request, StatsAvailability::Unavailable).captured_at_ms;
        Ok(sample)
    })
    .await
    .map_err(|_| AppError::new(ErrorCode::OperationTimedOut))?;
    result.map_err(|e| e.in_scope(&request.scope))
}
#[cfg(test)]
mod tests;
