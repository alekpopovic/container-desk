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
    ComposeList,
    ComposeIds,
    ComposeLabels,
    ComposeConfiguration,
    ContainerStats,
    StatsState,
    ContainerDetail,
    ImageDetail,
    ImageList,
    NetworkList,
    NetworkDetail,
    VolumeList,
    VolumeDetail,
    VolumeReferences,
    ImageReferences,
    LogSnapshot,
    LogStream,
    EventStream,
    Mutation,
    TerminalSession,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadOperation {
    ListContainers,
    ListNetworks,
    InspectNetwork {
        network_id: NetworkId,
    },
    ListVolumes,
    InspectVolume {
        name: VolumeName,
    },
    VolumeContainerIds {
        name: VolumeName,
    },
    InspectVolumeMounts {
        container_ids: Vec<ContainerId>,
    },
    ListImages {
        dangling_only: bool,
    },
    ImageContainerIds {
        image_id: ImageId,
    },
    InspectImageReferences {
        container_ids: Vec<ContainerId>,
    },
    ComposeValidate {
        configuration: ComposeConfiguration,
    },
    ComposeHashes {
        configuration: ComposeConfiguration,
    },
    ComposeExistingIds {
        configuration: ComposeConfiguration,
    },
    ComposeExistingLabels {
        container_ids: Vec<ContainerId>,
    },
    ListCompose,
    ComposeContainerIds,
    InspectComposeLabels {
        container_ids: Vec<ContainerId>,
    },
    FollowEvents {
        since: Option<String>,
    },
    ContainerStats {
        container_id: ContainerId,
    },
    RemovalState {
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
    compose_configuration: Option<ComposeConfiguration>,
}
impl CommandPlan {
    pub fn compose_configuration(&self) -> Option<&ComposeConfiguration> {
        self.compose_configuration.as_ref()
    }
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
        compose_configuration: None,
    }
}
fn limits(value: i32, min: i32, max: i32) -> Result<u32, AppError> {
    if (min..=max).contains(&value) {
        Ok(value as u32)
    } else {
        Err(AppError::new(ErrorCode::InvalidLimits))
    }
}
pub(crate) fn validate_compose(configuration: &ComposeConfiguration) -> Result<(), AppError> {
    let name = &configuration.project_name;
    if name.is_empty()
        || name.len() > 128
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
    {
        return Err(AppError::new(ErrorCode::InvalidRemoteArgument));
    }
    crate::docker::validate_absolute_path(&configuration.working_directory)?;
    if configuration.config_files.is_empty() || configuration.config_files.len() > 8 {
        return Err(AppError::new(ErrorCode::InvalidLimits));
    }
    let mut seen = HashSet::new();
    for path in &configuration.config_files {
        crate::docker::validate_absolute_path(path)?;
        if !seen.insert(path) {
            return Err(AppError::new(ErrorCode::InvalidRemoteArgument));
        }
    }
    Ok(())
}
pub(crate) fn validate_service(name: &str) -> Result<(), AppError> {
    if name.is_empty()
        || name.len() > 128
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
    {
        Err(AppError::new(ErrorCode::InvalidRemoteArgument))
    } else {
        Ok(())
    }
}
fn compose_plan(
    configuration: &ComposeConfiguration,
    tail: &[&str],
    category: OperationCategory,
    timeout: u32,
) -> Result<CommandPlan, AppError> {
    validate_compose(configuration)?;
    let mut command = plan(
        &[
            "docker",
            "compose",
            "--ansi",
            "never",
            "--progress",
            "quiet",
            "--parallel",
            "1",
            "--profile",
            "*",
            "--project-directory",
            &configuration.working_directory,
            "--project-name",
            &configuration.project_name,
        ],
        category,
        ResponseKind::ComposeConfiguration,
        timeout,
    );
    for file in &configuration.config_files {
        command.args.extend(["--file".into(), file.clone()]);
    }
    command.args.extend(tail.iter().map(|v| (*v).into()));
    command.compose_configuration = Some(configuration.clone());
    Ok(command)
}
pub fn read(operation: &ReadOperation) -> Result<CommandPlan, AppError> {
    Ok(match operation {
        ReadOperation::ComposeValidate { configuration } => compose_plan(
            configuration,
            &["config", "--quiet"],
            OperationCategory::Read,
            30,
        )?,
        ReadOperation::ComposeHashes { configuration } => compose_plan(
            configuration,
            &["config", "--hash", "*"],
            OperationCategory::Read,
            30,
        )?,
        ReadOperation::ComposeExistingIds { configuration } => compose_plan(
            configuration,
            &["ps", "--all", "--quiet", "--orphans=false"],
            OperationCategory::Read,
            30,
        )?,
        ReadOperation::ComposeExistingLabels { container_ids } => {
            if container_ids.is_empty() || container_ids.len() > 20 {
                return Err(AppError::new(ErrorCode::InvalidLimits));
            }
            let mut seen = HashSet::new();
            for id in container_ids {
                id.validate()?;
                if !seen.insert(id) {
                    return Err(AppError::new(ErrorCode::InvalidId));
                }
            }
            let mut command = plan(
                &[
                    "docker",
                    "inspect",
                    "--type",
                    "container",
                    "--format",
                    crate::docker::compose_actions::INSTANCE_TEMPLATE,
                    "--",
                ],
                OperationCategory::Read,
                ResponseKind::ComposeLabels,
                30,
            );
            command
                .args
                .extend(container_ids.iter().map(|id| id.0.clone()));
            command
        }
        ReadOperation::ListCompose => plan(
            &["docker", "compose", "ls", "--all", "--format", "json"],
            OperationCategory::Read,
            ResponseKind::ComposeList,
            30,
        ),
        ReadOperation::ComposeContainerIds => plan(
            &[
                "docker",
                "ps",
                "--all",
                "--quiet",
                "--no-trunc",
                "--filter",
                "label=com.docker.compose.project",
            ],
            OperationCategory::Read,
            ResponseKind::ComposeIds,
            30,
        ),
        ReadOperation::InspectComposeLabels { container_ids } => {
            if container_ids.is_empty() || container_ids.len() > 64 {
                return Err(AppError::new(ErrorCode::InvalidLimits));
            }
            let mut seen = HashSet::new();
            for id in container_ids {
                id.validate()?;
                if !seen.insert(id) {
                    return Err(AppError::new(ErrorCode::InvalidId));
                }
            }
            let mut command = plan(
                &[
                    "docker",
                    "inspect",
                    "--type",
                    "container",
                    "--format",
                    crate::docker::compose::LABEL_TEMPLATE,
                    "--",
                ],
                OperationCategory::Read,
                ResponseKind::ComposeLabels,
                30,
            );
            command
                .args
                .extend(container_ids.iter().map(|id| id.0.clone()));
            command
        }
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
        ReadOperation::RemovalState { container_id } => {
            container_id.validate()?;
            plan(
                &[
                    "docker",
                    "inspect",
                    "--type",
                    "container",
                    "--format",
                    r#"{"state":{{json .State.Status}},"running":{{json .State.Running}}}"#,
                    "--",
                    &container_id.0,
                ],
                OperationCategory::Read,
                ResponseKind::StatsState,
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
        ReadOperation::ListNetworks => plan(
            &[
                "docker",
                "network",
                "ls",
                "--no-trunc",
                "--format",
                crate::docker::networks::LIST_TEMPLATE,
            ],
            OperationCategory::Read,
            ResponseKind::NetworkList,
            30,
        ),
        ReadOperation::InspectNetwork { network_id } => {
            network_id.validate()?;
            plan(
                &["docker", "network", "inspect", "--", &network_id.0],
                OperationCategory::Read,
                ResponseKind::NetworkDetail,
                30,
            )
        }
        ReadOperation::ListVolumes => plan(
            &[
                "docker",
                "volume",
                "ls",
                "--format",
                crate::docker::volumes::LIST_TEMPLATE,
            ],
            OperationCategory::Read,
            ResponseKind::VolumeList,
            30,
        ),
        ReadOperation::InspectVolume { name } => {
            name.validate()?;
            plan(
                &["docker", "volume", "inspect", "--", &name.0],
                OperationCategory::Read,
                ResponseKind::VolumeDetail,
                30,
            )
        }
        ReadOperation::VolumeContainerIds { name } => {
            name.validate()?;
            plan(
                &[
                    "docker",
                    "ps",
                    "--all",
                    "--quiet",
                    "--no-trunc",
                    "--filter",
                    &format!("volume={}", name.0),
                ],
                OperationCategory::Read,
                ResponseKind::VolumeReferences,
                30,
            )
        }
        ReadOperation::InspectVolumeMounts { container_ids } => {
            if container_ids.is_empty() || container_ids.len() > 64 {
                return Err(AppError::new(ErrorCode::InvalidLimits));
            }
            let mut seen = HashSet::new();
            for id in container_ids {
                id.validate()?;
                if !seen.insert(id) {
                    return Err(AppError::new(ErrorCode::InvalidId));
                }
            }
            let mut command = plan(
                &[
                    "docker",
                    "inspect",
                    "--type",
                    "container",
                    "--format",
                    crate::docker::volumes::MOUNTS_TEMPLATE,
                    "--",
                ],
                OperationCategory::Read,
                ResponseKind::VolumeReferences,
                30,
            );
            command
                .args
                .extend(container_ids.iter().map(|id| id.0.clone()));
            command
        }
        ReadOperation::ListImages { dangling_only } => {
            let mut command = plan(
                &[
                    "docker",
                    "image",
                    "ls",
                    "--all",
                    "--no-trunc",
                    "--digests",
                    "--format",
                    "{{json .}}",
                ],
                OperationCategory::Read,
                ResponseKind::ImageList,
                30,
            );
            if *dangling_only {
                command
                    .args
                    .extend(["--filter".into(), "dangling=true".into()]);
            }
            command
        }
        ReadOperation::ImageContainerIds { image_id } => {
            image_id.validate()?;
            plan(
                &[
                    "docker",
                    "ps",
                    "--all",
                    "--quiet",
                    "--no-trunc",
                    "--filter",
                    &format!("ancestor={}", image_id.0),
                ],
                OperationCategory::Read,
                ResponseKind::ImageReferences,
                30,
            )
        }
        ReadOperation::InspectImageReferences { container_ids } => {
            if container_ids.is_empty() || container_ids.len() > 64 {
                return Err(AppError::new(ErrorCode::InvalidLimits));
            }
            let mut seen = HashSet::new();
            for id in container_ids {
                id.validate()?;
                if !seen.insert(id) {
                    return Err(AppError::new(ErrorCode::InvalidId));
                }
            }
            let mut command = plan(
                &[
                    "docker",
                    "inspect",
                    "--type",
                    "container",
                    "--format",
                    crate::docker::images::REFERENCE_TEMPLATE,
                    "--",
                ],
                OperationCategory::Read,
                ResponseKind::ImageReferences,
                30,
            );
            command
                .args
                .extend(container_ids.iter().map(|id| id.0.clone()));
            command
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
                compose_configuration: None,
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
                MutationOperation::Remove => plan(
                    &["docker", "rm"],
                    OperationCategory::Mutation,
                    ResponseKind::Mutation,
                    30,
                ),
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
        ConfirmationOperation::Compose(spec) => {
            spec.verification_id.validate()?;
            if spec.services.is_empty()
                || spec.services.len() > 20
                || spec.container_ids.is_empty()
                || spec.container_ids.len() > 20
            {
                return Err(AppError::new(ErrorCode::InvalidLimits));
            }
            let mut services = HashSet::new();
            for service in &spec.services {
                validate_service(service)?;
                if !services.insert(service) {
                    return Err(AppError::new(ErrorCode::InvalidRemoteArgument));
                }
            }
            let mut ids = HashSet::new();
            for id in &spec.container_ids {
                id.validate()?;
                if !ids.insert(id) {
                    return Err(AppError::new(ErrorCode::InvalidId));
                }
            }
            let timeout = limits(spec.timeout_seconds, 1, 120)?;
            let seconds = timeout.to_string();
            let tail = match spec.operation {
                ComposeActionOperation::Start => vec!["start"],
                ComposeActionOperation::Stop => vec!["stop", "--timeout", &seconds],
                ComposeActionOperation::Restart => {
                    vec!["restart", "--no-deps", "--timeout", &seconds]
                }
            };
            let mut command = compose_plan(
                &spec.configuration,
                &tail,
                OperationCategory::Mutation,
                timeout + 10,
            )?;
            command.response = ResponseKind::Mutation;
            command.args.push("--".into());
            command.args.extend(spec.services.iter().cloned());
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
                    "--user",
                    "1000:1000",
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
    fn lifecycle_plans_validate_timeouts_and_keep_exact_targets_after_separator() {
        for operation in [
            MutationOperation::Remove,
            MutationOperation::Start,
            MutationOperation::Stop,
            MutationOperation::Restart,
        ] {
            for timeout_seconds in [-1, 0, 121, i32::MAX] {
                assert_eq!(
                    confirmation(&ConfirmationOperation::Mutation(MutationSpec {
                        operation: operation.clone(),
                        container_ids: vec![ContainerId("a".repeat(64))],
                        timeout_seconds
                    }))
                    .unwrap_err()
                    .code,
                    ErrorCode::InvalidLimits
                );
            }
            let plan = confirmation(&ConfirmationOperation::Mutation(MutationSpec {
                operation: operation.clone(),
                container_ids: vec![ContainerId("a".repeat(64))],
                timeout_seconds: 120,
            }))
            .unwrap();
            assert_eq!(
                &plan.args()[plan.args().len() - 2..],
                &["--".to_string(), "a".repeat(64)]
            );
            assert!(plan.timeout_seconds() <= 130);
            assert!(
                !plan
                    .args()
                    .iter()
                    .any(|arg| ["-f", "--force", "-v", "--volumes"].contains(&arg.as_str()))
            );
            assert!(
                !plan
                    .args()
                    .iter()
                    .any(|arg| ["--force", "--attach", "--interactive"].contains(&arg.as_str()))
            );
        }
    }
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
