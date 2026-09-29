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
pub(crate) async fn run(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: MutationRequest,
    mut owner: Operation,
    current: Current,
) -> Result<MutationResponse, AppError> {
    let fresh = tokio::time::timeout(Duration::from_secs(30), async {
        current()?;
        let inventory = super::listing::read(client, options, binding, &request.scope).await?;
        if !request
            .spec
            .container_ids
            .iter()
            .all(|id| inventory.containers.iter().any(|row| &row.id == id))
        {
            return Err(AppError::new(ErrorCode::ContainerNotFound).in_scope(&request.scope));
        }
        let (fresh, _) = super::probe::run(client, options).await;
        if fresh.daemon_id.as_deref() != Some(request.scope.daemon_id.as_str()) {
            return Err(AppError::new(ErrorCode::StaleSession));
        }
        // Reject changed endpoint/context before recording dispatch intent.
        binding.prepare(
            registry::confirmation(&ConfirmationOperation::Mutation(request.spec.clone()))?,
            &fresh,
        )?;
        current()?;
        Ok(fresh)
    })
    .await
    .map_err(|_| AppError::new(ErrorCode::OperationTimedOut))??;
    let command = binding.prepare(owner.dispatch()?, &fresh)?;
    let result = match client.start_fixed(
        command.encoded().to_owned(),
        Limits {
            deadline: Duration::from_secs(command.timeout_seconds().into()),
            stdout_bytes: 64 * 1024,
            stderr_bytes: 64 * 1024,
        },
    ) {
        Ok(job) => job.wait().await,
        // Dispatch ownership was committed; remain conservative even if local startup failed.
        Err(_) => {
            owner.finish(MutationOutcome::Unknown)?;
            return Ok(MutationResponse {
                scope: request.scope,
                spec: request.spec,
                outcome: MutationOutcome::Unknown,
            });
        }
    };
    let observed = outcome(&result);
    // The job's owner has already reaped its process; discard command output without logging it.
    owner.finish(observed.clone())?;
    Ok(MutationResponse {
        scope: request.scope,
        spec: request.spec,
        outcome: observed,
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
