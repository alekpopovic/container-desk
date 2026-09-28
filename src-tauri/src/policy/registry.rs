//! Allowlisted argv plans only. docker::prepare validates configuration and encodes for the remote shell.
use crate::domain::*;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationCategory {
    Read,
    Mutation,
    Terminal,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseKind {
    ContainerList,
    ContainerDetail,
    ImageDetail,
    LogSnapshot,
    Mutation,
    TerminalSession,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadOperation {
    ListContainers,
    InspectImage {
        image_id: ImageId,
    },
    InspectContainer {
        container_id: ContainerId,
    },
    ContainerLogs {
        container_id: ContainerId,
        tail: i32,
        timeout_seconds: i32,
    },
}
/// Cannot be constructed by an IPC caller or from arbitrary shell text.
#[derive(Clone, Debug)]
pub struct CommandPlan {
    args: Vec<String>,
    category: OperationCategory,
    response: ResponseKind,
    timeout_seconds: u32,
}
impl CommandPlan {
    pub fn args(&self) -> &[String] {
        &self.args
    }
    pub fn category(&self) -> &OperationCategory {
        &self.category
    }
    pub fn response(&self) -> &ResponseKind {
        &self.response
    }
    pub fn timeout_seconds(&self) -> u32 {
        self.timeout_seconds
    }
}
fn plan(
    args: &[&str],
    category: OperationCategory,
    response: ResponseKind,
    timeout_seconds: u32,
) -> CommandPlan {
    CommandPlan {
        args: args.iter().map(|s| s.to_string()).collect(),
        category,
        response,
        timeout_seconds,
    }
}
fn limits(value: i32, min: i32, max: i32) -> Result<u32, AppError> {
    if (min..=max).contains(&value) {
        Ok(value as u32)
    } else {
        Err(AppError::new(ErrorCode::InvalidLimits))
    }
}
pub fn read(operation: &ReadOperation) -> Result<CommandPlan, AppError> {
    Ok(match operation {
        ReadOperation::ListContainers => plan(
            &[
                "docker",
                "ps",
                "--all",
                "--no-trunc",
                "--format",
                "{{json .}}",
            ],
            OperationCategory::Read,
            ResponseKind::ContainerList,
            30,
        ),
        ReadOperation::InspectImage { image_id } => {
            image_id.validate()?;
            plan(
                &["docker", "image", "inspect", "--", &image_id.0],
                OperationCategory::Read,
                ResponseKind::ImageDetail,
                30,
            )
        }
        ReadOperation::InspectContainer { container_id } => {
            container_id.validate()?;
            plan(
                &[
                    "docker",
                    "inspect",
                    "--type",
                    "container",
                    "--",
                    &container_id.0,
                ],
                OperationCategory::Read,
                ResponseKind::ContainerDetail,
                30,
            )
        }
        ReadOperation::ContainerLogs {
            container_id,
            tail,
            timeout_seconds,
        } => {
            container_id.validate()?;
            limits(*tail, 0, 20000)?;
            let timeout = limits(*timeout_seconds, 1, 30)?;
            plan(
                &[
                    "docker",
                    "logs",
                    "--timestamps",
                    "--tail",
                    &tail.to_string(),
                    "--",
                    &container_id.0,
                ],
                OperationCategory::Read,
                ResponseKind::LogSnapshot,
                timeout,
            )
        }
    })
}
pub fn confirmation(operation: &ConfirmationOperation) -> Result<CommandPlan, AppError> {
    Ok(match operation {
        ConfirmationOperation::Mutation(spec) => {
            if spec.container_ids.is_empty() || spec.container_ids.len() > 20 {
                return Err(AppError::new(ErrorCode::InvalidLimits));
            }
            let mut unique = HashSet::new();
            for id in &spec.container_ids {
                id.validate()?;
                if !unique.insert(id) {
                    return Err(AppError::new(ErrorCode::InvalidId));
                }
            }
            let timeout = limits(spec.timeout_seconds, 1, 120)?;
            let mut command = match spec.operation {
                MutationOperation::Start => plan(
                    &["docker", "start"],
                    OperationCategory::Mutation,
                    ResponseKind::Mutation,
                    30,
                ),
                MutationOperation::Stop => plan(
                    &["docker", "stop", "-t", &timeout.to_string()],
                    OperationCategory::Mutation,
                    ResponseKind::Mutation,
                    timeout + 10,
                ),
                MutationOperation::Restart => plan(
                    &["docker", "restart", "-t", &timeout.to_string()],
                    OperationCategory::Mutation,
                    ResponseKind::Mutation,
                    timeout + 10,
                ),
            };
            command.args.push("--".into());
            command
                .args
                .extend(spec.container_ids.iter().map(|id| id.0.clone()));
            command
        }
        ConfirmationOperation::Terminal(spec) => {
            spec.container_id.validate()?;
            limits(spec.columns, 20, 500)?;
            limits(spec.rows, 5, 300)?;
            let shell = match spec.shell {
                TerminalShell::Sh => "/bin/sh",
                TerminalShell::Bash => "/bin/bash",
            };
            plan(
                &[
                    "docker",
                    "exec",
                    "--interactive",
                    "--tty",
                    "--",
                    &spec.container_id.0,
                    shell,
                ],
                OperationCategory::Terminal,
                ResponseKind::TerminalSession,
                10,
            )
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_limits_ids_and_supported_variants_are_checked_before_building() {
        let id = ContainerId("a".repeat(64));
        for tail in [-1, 20001, i32::MAX] {
            assert_eq!(
                read(&ReadOperation::ContainerLogs {
                    container_id: id.clone(),
                    tail,
                    timeout_seconds: 30
                })
                .unwrap_err()
                .code,
                ErrorCode::InvalidLimits
            );
        }
        for timeout_seconds in [-1, 0, 31] {
            assert_eq!(
                read(&ReadOperation::ContainerLogs {
                    container_id: id.clone(),
                    tail: 0,
                    timeout_seconds
                })
                .unwrap_err()
                .code,
                ErrorCode::InvalidLimits
            );
        }
        assert_eq!(
            read(&ReadOperation::InspectContainer {
                container_id: ContainerId("--all;touch x".into())
            })
            .unwrap_err()
            .code,
            ErrorCode::InvalidId
        );
        assert!(serde_json::from_str::<MutationOperation>("\"prune\"").is_err());
        let extra = serde_json::json!({"category":"mutation", "spec":{"operation":"stop", "containerIds":["a".repeat(64)], "timeoutSeconds":10}, "command":"unwanted"});
        assert!(serde_json::from_value::<ConfirmationOperation>(extra).is_err());
        assert!(serde_json::from_str::<TerminalShell>("\"sh -c anything\"").is_err());
        let command = read(&ReadOperation::ContainerLogs {
            container_id: id,
            tail: 20000,
            timeout_seconds: 30,
        })
        .unwrap();
        assert_eq!(command.category(), &OperationCategory::Read);
        assert_eq!(command.response(), &ResponseKind::LogSnapshot);
        assert_eq!(command.args()[4], "20000");
    }
}
