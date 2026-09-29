//! Exactly one bounded dispatch; failed transport is never a retry instruction.
use crate::{
    activity::Operation,
    domain::*,
    policy::registry,
    ssh::{
        multiplex::Client,
        runner::{Captured, Limits, RunError},
    },
};
use std::{sync::Arc, time::Duration};
pub(crate) type Current = Arc<dyn Fn() -> Result<(), AppError> + Send + Sync>;
fn outcome(result: &Result<Captured, RunError>) -> MutationOutcome {
    match result {
        Ok(output) if output.status.success() => MutationOutcome::Succeeded,
        Ok(output) if output.status.code().is_some_and(|code| code != 255) => {
            MutationOutcome::Failed
        }
        _ => MutationOutcome::Unknown,
    }
}
fn diagnostic(output: &Captured) -> Option<ErrorCode> {
    let text = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    if text.contains("no such container") || text.contains("no such object") {
        Some(ErrorCode::ContainerNotFound)
    } else if text.contains("permission denied") || text.contains("access denied") {
        Some(ErrorCode::PermissionDenied)
    } else {
        None
    }
}
async fn preflight(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: &MutationRequest,
    id: &ContainerId,
    current: &Current,
) -> Result<DockerProbeReport, AppError> {
    current()?;
    let (fresh, _) = super::probe::run(client, options).await;
    if fresh.daemon_id.as_deref() != Some(request.scope.daemon_id.as_str()) {
        return Err(AppError::new(ErrorCode::StaleSession));
    }
    let state = binding.prepare(
        registry::read(&registry::ReadOperation::RemovalState {
            container_id: id.clone(),
        })?,
        &fresh,
    )?;
    let output = client
        .start_fixed(
            state.encoded().to_string(),
            Limits {
                deadline: Duration::from_secs(10),
                stdout_bytes: 4096,
                stderr_bytes: 8192,
            },
        )?
        .wait()
        .await
        .map_err(|_| AppError::new(ErrorCode::TransportUnavailable))?;
    if !output.status.success() {
        return Err(AppError::new(
            diagnostic(&output).unwrap_or(ErrorCode::TransportUnavailable),
        ));
    }
    #[derive(serde::Deserialize)]
    struct State {
        state: String,
        running: bool,
    }
    let state: State = serde_json::from_slice(&output.stdout)
        .map_err(|_| AppError::new(ErrorCode::InvalidResponse))?;
    if state.state.len() > 32 || state.state.chars().any(char::is_control) {
        return Err(AppError::new(ErrorCode::InvalidResponse));
    }
    if request.spec.operation == MutationOperation::Remove
        && (state.running || !["exited", "created"].contains(&state.state.as_str()))
    {
        return Err(AppError::new(ErrorCode::ContainerNotStopped));
    }
    current()?;
    Ok(fresh)
}
pub(crate) async fn run(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: MutationRequest,
    mut owner: Operation,
    current: Current,
) -> Result<MutationResponse, AppError> {
    let mut halted = None;
    for id in &request.spec.container_ids {
        if owner.cancelled() || halted.is_some() {
            owner.complete_target(MutationTargetResult {
                container_id: id.clone(),
                outcome: MutationTargetOutcome::Cancelled,
                dispatched: false,
                error: halted.clone().or(Some(ErrorCode::OperationCancelled)),
            })?;
            continue;
        }
        let fresh = tokio::time::timeout(
            Duration::from_secs(30),
            preflight(client, options, binding, &request, id, &current),
        )
        .await
        .map_err(|_| AppError::new(ErrorCode::OperationTimedOut))
        .and_then(|result| result);
        let fresh = match fresh {
            Ok(fresh) => fresh,
            Err(error) => {
                if matches!(
                    error.code,
                    ErrorCode::StaleSession
                        | ErrorCode::Disconnected
                        | ErrorCode::SessionNotFound
                        | ErrorCode::TransportUnavailable
                        | ErrorCode::OperationTimedOut
                ) {
                    halted = Some(error.code.clone());
                }
                owner.complete_target(MutationTargetResult {
                    container_id: id.clone(),
                    outcome: MutationTargetOutcome::Failed,
                    dispatched: false,
                    error: Some(error.code),
                })?;
                continue;
            }
        };
        // Pending cancellation never interrupts the child for a target already dispatched.
        if owner.cancelled() {
            owner.complete_target(MutationTargetResult {
                container_id: id.clone(),
                outcome: MutationTargetOutcome::Cancelled,
                dispatched: false,
                error: Some(ErrorCode::OperationCancelled),
            })?;
            continue;
        }
        let plan = match owner.dispatch_target(id) {
            Ok(plan) => plan,
            Err(error) if error.code == ErrorCode::OperationCancelled => {
                owner.complete_target(MutationTargetResult {
                    container_id: id.clone(),
                    outcome: MutationTargetOutcome::Cancelled,
                    dispatched: false,
                    error: Some(error.code),
                })?;
                continue;
            }
            Err(error) => return Err(error),
        };
        let command = binding.prepare(plan, &fresh)?;
        let result = match client.start_fixed(
            command.encoded().to_owned(),
            Limits {
                deadline: Duration::from_secs(command.timeout_seconds().into()),
                stdout_bytes: 64 * 1024,
                stderr_bytes: 64 * 1024,
            },
        ) {
            Ok(job) => job.wait().await,
            Err(_) => Err(RunError::Unavailable),
        };
        let observed = outcome(&result);
        let error = result.as_ref().ok().and_then(diagnostic);
        let target_outcome = match observed {
            MutationOutcome::Succeeded => MutationTargetOutcome::Succeeded,
            MutationOutcome::Failed => MutationTargetOutcome::Failed,
            _ => {
                halted = Some(ErrorCode::TransportUnavailable);
                MutationTargetOutcome::Unknown
            }
        };
        owner.complete_target(MutationTargetResult {
            container_id: id.clone(),
            outcome: target_outcome,
            dispatched: true,
            error: error.or_else(|| halted.clone()),
        })?;
    }
    let results = owner.results()?;
    Ok(MutationResponse {
        scope: request.scope,
        spec: request.spec,
        outcome: crate::activity::response_outcome(&results),
        results,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    #[test]
    fn interruption_and_ssh_exit_are_unknown_without_a_retry_branch() {
        for error in [
            RunError::Cancelled,
            RunError::TimedOut,
            RunError::Io,
            RunError::Unavailable,
            RunError::OutputLimit(crate::ssh::runner::Stream::Stdout),
        ] {
            assert_eq!(outcome(&Err(error)), MutationOutcome::Unknown);
        }
        for (code, expected) in [
            (0, MutationOutcome::Succeeded),
            (1, MutationOutcome::Failed),
            (255, MutationOutcome::Unknown),
        ] {
            assert_eq!(
                outcome(&Ok(Captured {
                    status: std::process::ExitStatus::from_raw(code << 8),
                    stdout: vec![],
                    stderr: vec![]
                })),
                expected
            );
        }
    }
}
