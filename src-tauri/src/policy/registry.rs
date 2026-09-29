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
    ContainerStats,
    StatsState,
    ContainerDetail,
    ImageDetail,
    LogSnapshot,
    LogStream,
    EventStream,
    Mutation,
    TerminalSession,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadOperation {
    ListContainers,
    FollowEvents {
        since: Option<String>,
    },
    ContainerStats {
        container_id: ContainerId,
    },
    StatsState {
        container_id: ContainerId,
    },
    InspectImage {
        image_id: ImageId,
    },
    InspectContainer {
        container_id: ContainerId,
    },
    FollowLogs {
        container_id: ContainerId,
        tail: i32,
        since: Option<String>,
    },
    ContainerLogs {
        since: Option<String>,
        until: Option<String>,
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
        ReadOperation::FollowEvents { since } => {
            validate_log_range(since.as_deref(), None)?;
            let mut result = plan(
                &[
                    "docker",
                    "events",
                    "--filter",
                    "type=container",
                    "--format",
                    "{{json .}}",
                ],
                OperationCategory::Read,
                ResponseKind::EventStream,
                30,
            );
            if let Some(since) = since {
                result.args.extend(["--since".into(), since.clone()]);
            }
            result
        }
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
        ReadOperation::ContainerStats { container_id } => {
            container_id.validate()?;
            plan(
                &[
                    "docker",
                    "stats",
                    "--no-stream",
                    "--no-trunc",
                    "--format",
                    "{{json .}}",
                    "--",
                    &container_id.0,
                ],
                OperationCategory::Read,
                ResponseKind::ContainerStats,
                30,
            )
        }
        ReadOperation::StatsState { container_id } => {
            container_id.validate()?;
            plan(
                &[
                    "docker",
                    "inspect",
                    "--type",
                    "container",
                    "--format",
                    r#"{"running":{{json .State.Running}},"startedAt":{{json .State.StartedAt}}}"#,
                    "--",
                    &container_id.0,
                ],
                OperationCategory::Read,
                ResponseKind::StatsState,
                30,
            )
        }
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
        ReadOperation::FollowLogs {
            container_id,
            tail,
            since,
        } => {
            let mut plan = read(&ReadOperation::ContainerLogs {
                container_id: container_id.clone(),
                tail: *tail,
                since: since.clone(),
                until: None,
                timeout_seconds: 30,
            })?;
            plan.args.insert(2, "--follow".into());
            plan.response = ResponseKind::LogStream;
            plan
        }
        ReadOperation::ContainerLogs {
            container_id,
            tail,
            timeout_seconds,
            since,
            until,
        } => {
            container_id.validate()?;
            limits(*tail, 1, 20000)?;
            let timeout = limits(*timeout_seconds, 1, 30)?;
            validate_log_range(since.as_deref(), until.as_deref())?;
            let mut args = vec![
                "docker".into(),
                "logs".into(),
                "--timestamps".into(),
                "--tail".into(),
                tail.to_string(),
            ];
            if let Some(value) = since {
                args.extend(["--since".into(), value.clone()]);
            }
            if let Some(value) = until {
                args.extend(["--until".into(), value.clone()]);
            }
            args.extend(["--".into(), container_id.0.clone()]);
            CommandPlan {
                args,
                category: OperationCategory::Read,
                response: ResponseKind::LogSnapshot,
                timeout_seconds: timeout,
            }
        }
    })
}
/// UTC Unix seconds only; no relative dates, local timezone interpretation or shell fragments.
pub(crate) fn validate_log_range(since: Option<&str>, until: Option<&str>) -> Result<(), AppError> {
    fn epoch(s: &str) -> Option<(u64, u32)> {
        if s.len() > 22 {
            return None;
        }
        let (seconds, fraction) = s.split_once('.').map_or((s, ""), |(a, b)| (a, b));
        if seconds.is_empty()
            || seconds.len() > 12
            || !seconds.bytes().all(|b| b.is_ascii_digit())
            || fraction.len() > 9
            || !fraction.bytes().all(|b| b.is_ascii_digit())
            || s.ends_with('.')
        {
            return None;
        }
        let seconds = seconds.parse::<u64>().ok()?;
        if seconds > 253402300799 {
            return None;
        }
        let nanos = if fraction.is_empty() {
            0
        } else {
            fraction.parse::<u32>().ok()? * 10u32.pow(9 - fraction.len() as u32)
        };
        Some((seconds, nanos))
    }
    let invalid = || AppError::new(ErrorCode::InvalidLimits);
    let from = since.map(|s| epoch(s).ok_or_else(invalid)).transpose()?;
    let to = until.map(|s| epoch(s).ok_or_else(invalid)).transpose()?;
    if from.zip(to).is_some_and(|(a, b)| a > b) {
        return Err(invalid());
    }
    Ok(())
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
        for tail in [-1, 0, 20001, i32::MAX] {
            assert_eq!(
                read(&ReadOperation::ContainerLogs {
                    since: None,
                    until: None,
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
                    since: None,
                    until: None,
                    container_id: id.clone(),
                    tail: 1,
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
            since: None,
            until: None,
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
