//! Docker CLI JSONL adapter. Display strings are not inspect data or authorization metadata.
use crate::{
    domain::*,
    policy::registry::{self, ReadOperation},
    ssh::{
        multiplex::Connection,
        runner::{Limits, RunError},
    },
};
use serde::Deserialize;
use std::{collections::HashSet, time::Duration};
pub const MAX_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_RECORD_BYTES: usize = 256 * 1024;
pub const MAX_RECORDS: usize = 50_000;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    #[serde(rename = "ID")]
    id: ContainerId,
    names: Option<String>,
    image: Option<String>,
    state: Option<String>,
    status: Option<String>,
    ports: Option<String>,
    labels: Option<String>,
    created_at: Option<String>,
    running_for: Option<String>,
}
fn error(scope: &SessionScope, code: ErrorCode) -> AppError {
    AppError::new(code).in_scope(scope)
}
fn bounded(value: &Option<String>, limit: usize) -> bool {
    value.as_ref().is_none_or(|s| s.len() <= limit)
}
/// No non-JSON line is silently discarded. Only the final JSONL newline is optional.
/// Unknown fields are ignored by serde; invalid types in known fields remain errors.
pub fn parse(scope: &SessionScope, bytes: &[u8]) -> Result<ListContainersResponse, AppError> {
    scope.validate()?;
    if bytes.len() > MAX_BYTES {
        return Err(error(scope, ErrorCode::ResourceLimit));
    }
    let mut containers = Vec::new();
    let mut ids = HashSet::new();
    let contents = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if !bytes.is_empty() {
        for line in contents.split(|byte| *byte == b'\n') {
            if line.len() > MAX_RECORD_BYTES || containers.len() >= MAX_RECORDS {
                return Err(error(scope, ErrorCode::ResourceLimit));
            }
            let record: Record = serde_json::from_slice(line)
                .map_err(|_| error(scope, ErrorCode::InvalidResponse))?;
            record
                .id
                .validate()
                .map_err(|_| error(scope, ErrorCode::InvalidResponse))?;
            if !ids.insert(record.id.clone()) {
                return Err(error(scope, ErrorCode::InvalidResponse));
            }
            if [
                &record.names,
                &record.image,
                &record.state,
                &record.status,
                &record.created_at,
                &record.running_for,
            ]
            .iter()
            .any(|value| !bounded(value, 4096))
                || !bounded(&record.ports, 16 * 1024)
                || !bounded(&record.labels, 64 * 1024)
            {
                return Err(error(scope, ErrorCode::ResourceLimit));
            }
            let names: Vec<String> = record
                .names
                .as_deref()
                .unwrap_or("")
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            if names.len() > 128 {
                return Err(error(scope, ErrorCode::ResourceLimit));
            }
            let name = names
                .first()
                .cloned()
                .unwrap_or_else(|| record.id.0.clone());
            let present = record.labels.as_ref().map(|s| !s.is_empty());
            containers.push(ContainerSummary {
                scope: scope.clone(),
                id: record.id,
                name,
                image: record
                    .image
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "Unknown".into()),
                state: record
                    .state
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "unknown".into()),
                status: record.status.unwrap_or_default(),
                // No inference of exact health/port bindings or verified Compose metadata from ps text.
                health: None,
                ports: vec![],
                compose: None,
                cli: Some(ContainerListDisplay {
                    names,
                    ports: record.ports,
                    created_at: record.created_at,
                    running_for: record.running_for,
                    // CLI Labels is ambiguous comma-separated key=value text, including unescaped values.
                    // Default IPC exposes only presence. Inspect supplies exact, redacted metadata later.
                    labels_present: present,
                }),
            });
        }
    }
    Ok(ListContainersResponse {
        scope: scope.clone(),
        containers,
    })
}

fn decode(
    scope: &SessionScope,
    output: crate::ssh::runner::Captured,
) -> Result<ListContainersResponse, AppError> {
    // Exit status is checked before parsing: failed empty output is never an empty inventory.
    if !output.status.success() {
        return Err(error(
            scope,
            if output.status.code() == Some(255) {
                ErrorCode::Disconnected
            } else {
                ErrorCode::TransportUnavailable
            },
        ));
    }
    parse(scope, &output.stdout)
}

/// Called with a backend-owned current connection/binding. IPC registration is integrated in 020.
/// The caller must fence its session before/after this future; closing the connection cancels jobs.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Live inventory dispatcher follows in prompt 020")
)]
pub(crate) async fn read(
    connection: &Connection,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    scope: &SessionScope,
) -> Result<ListContainersResponse, AppError> {
    scope.validate()?;
    tokio::time::timeout(Duration::from_secs(30), async {
        let (fresh, _) = super::probe::run(connection, options).await;
        if fresh.daemon_id.as_deref() != Some(scope.daemon_id.as_str()) {
            return Err(error(scope, ErrorCode::StaleSession));
        }
        let plan = registry::read(&ReadOperation::ListContainers)?;
        let command = binding
            .prepare(plan, &fresh)
            .map_err(|e| e.in_scope(scope))?;
        let result = connection
            .start_fixed(
                command.encoded().to_string(),
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
                error(
                    scope,
                    match e {
                        RunError::TimedOut => ErrorCode::OperationTimedOut,
                        RunError::OutputLimit(_) => ErrorCode::ResourceLimit,
                        RunError::Cancelled => ErrorCode::OperationCancelled,
                        _ => ErrorCode::TransportUnavailable,
                    },
                )
            })?;
        let parsed = decode(scope, result)?;
        // Do not publish a successful read under an identity that changed while it ran.
        let (after, _) = super::probe::run(connection, options).await;
        binding
            .prepare(registry::read(&ReadOperation::ListContainers)?, &after)
            .map_err(|e| e.in_scope(scope))?;
        Ok(parsed)
    })
    .await
    .map_err(|_| error(scope, ErrorCode::OperationTimedOut))?
}

#[cfg(test)]
mod tests;
