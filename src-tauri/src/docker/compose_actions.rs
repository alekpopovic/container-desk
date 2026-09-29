//! Explicitly verified project configuration; never execute a path discovered only from labels.
use crate::{
    domain::*,
    policy::registry::{self, ReadOperation},
    ssh::{
        multiplex::Client,
        runner::{Captured, Limits, RunError},
    },
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};
pub(crate) const INSTANCE_TEMPLATE: &str = r#"{"id":{{json .Id}},"project":{{json (index .Config.Labels "com.docker.compose.project")}},"service":{{json (index .Config.Labels "com.docker.compose.service")}},"hash":{{json (index .Config.Labels "com.docker.compose.config-hash")}},"oneoff":{{json (index .Config.Labels "com.docker.compose.oneoff")}}}"#;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerifiedRead {
    pub services: Vec<String>,
    pub container_ids: Vec<ContainerId>,
    // These hashes never leave Rust or enter persistent diagnostics/activity.
    pub hashes: BTreeMap<String, String>,
}
fn error(scope: &SessionScope, code: ErrorCode) -> AppError {
    AppError::new(code).in_scope(scope)
}
fn read_error(scope: &SessionScope, result: Captured) -> Result<Vec<u8>, AppError> {
    let diagnostic = String::from_utf8_lossy(&result.stderr).to_ascii_lowercase();
    if !result.status.success()
        || diagnostic.contains("variable is not set")
        || diagnostic.contains("defaulting to a blank string")
    {
        return Err(error(
            scope,
            if result.status.code() == Some(255) {
                ErrorCode::Disconnected
            } else {
                ErrorCode::ComposeConfigurationUnavailable
            },
        ));
    }
    Ok(result.stdout)
}
async fn read(
    client: &Client,
    binding: &super::probe::VerifiedDocker,
    report: &DockerProbeReport,
    scope: &SessionScope,
    operation: ReadOperation,
    cap: usize,
) -> Result<Vec<u8>, AppError> {
    let command = binding.prepare(registry::read(&operation)?, report)?;
    let output = client
        .start_fixed(
            command.encoded().into(),
            Limits {
                deadline: Duration::from_secs(30),
                stdout_bytes: cap,
                stderr_bytes: 64 * 1024,
            },
        )?
        .wait()
        .await
        .map_err(|cause| {
            error(
                scope,
                match cause {
                    RunError::TimedOut => ErrorCode::OperationTimedOut,
                    RunError::OutputLimit(_) => ErrorCode::ResourceLimit,
                    RunError::Cancelled => ErrorCode::Disconnected,
                    _ => ErrorCode::TransportUnavailable,
                },
            )
        })?;
    read_error(scope, output)
}
fn hashes(bytes: &[u8]) -> Result<BTreeMap<String, String>, AppError> {
    if bytes.len() > 32768 {
        return Err(AppError::new(ErrorCode::ResourceLimit));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| AppError::new(ErrorCode::InvalidResponse))?;
    let mut hashes = BTreeMap::new();
    for line in text.lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        if words.len() != 2
            || words[1].len() != 64
            || !words[1]
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(AppError::new(ErrorCode::InvalidResponse));
        }
        registry::validate_service(words[0])?;
        if hashes.insert(words[0].into(), words[1].into()).is_some() {
            return Err(AppError::new(ErrorCode::InvalidResponse));
        }
        if hashes.len() > 128 {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
    }
    if hashes.is_empty() {
        return Err(AppError::new(ErrorCode::ComposeProjectMismatch));
    }
    Ok(hashes)
}
fn ids(bytes: &[u8]) -> Result<Vec<ContainerId>, AppError> {
    if bytes.len() > 20 * 65 {
        return Err(AppError::new(ErrorCode::ResourceLimit));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| AppError::new(ErrorCode::InvalidResponse))?;
    let mut ids = BTreeSet::new();
    for line in text.lines() {
        let id = ContainerId(line.into());
        id.validate()?;
        if !ids.insert(id.0) {
            return Err(AppError::new(ErrorCode::InvalidResponse));
        }
    }
    if ids.is_empty() {
        return Err(AppError::new(ErrorCode::ComposeProjectMismatch));
    }
    Ok(ids.into_iter().map(ContainerId).collect())
}
#[derive(Deserialize)]
struct Instance {
    id: ContainerId,
    project: String,
    service: String,
    hash: String,
    oneoff: Option<String>,
}
fn verify_instances(
    configuration: &ComposeConfiguration,
    expected: Vec<ContainerId>,
    hashes: BTreeMap<String, String>,
    bytes: &[u8],
) -> Result<VerifiedRead, AppError> {
    if bytes.len() > 32768 {
        return Err(AppError::new(ErrorCode::ResourceLimit));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| AppError::new(ErrorCode::InvalidResponse))?;
    let mut seen = BTreeSet::new();
    let mut services = BTreeSet::new();
    for line in text.lines() {
        let row: Instance =
            serde_json::from_str(line).map_err(|_| AppError::new(ErrorCode::InvalidResponse))?;
        if !expected.contains(&row.id)
            || !seen.insert(row.id.0)
            || row.project != configuration.project_name
            || hashes.get(&row.service) != Some(&row.hash)
            || !row
                .oneoff
                .as_ref()
                .is_some_and(|v| v.eq_ignore_ascii_case("false"))
        {
            return Err(AppError::new(ErrorCode::ComposeProjectMismatch));
        }
        services.insert(row.service);
    }
    if seen.len() != expected.len() {
        return Err(AppError::new(ErrorCode::ComposeProjectMismatch));
    }
    Ok(VerifiedRead {
        services: services.into_iter().collect(),
        container_ids: expected,
        hashes,
    })
}
pub(crate) async fn verify(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: &VerifyComposeRequest,
) -> Result<VerifiedRead, AppError> {
    let scope = &request.scope;
    scope.validate()?;
    registry::validate_compose(&request.configuration)?;
    if !request.acknowledged {
        return Err(error(scope, ErrorCode::PermissionDenied));
    }
    tokio::time::timeout(Duration::from_secs(30), async {
        let (fresh, _) = super::probe::run(client, options).await;
        super::probe::transport_ready(&fresh)?;
        if fresh.daemon_id.as_deref() != Some(scope.daemon_id.as_str()) {
            return Err(error(scope, ErrorCode::StaleSession));
        }
        if fresh.compose != ComposeAvailability::Available {
            return Err(error(scope, ErrorCode::ComposeConfigurationUnavailable));
        }
        let configuration = &request.configuration;
        read(
            client,
            binding,
            &fresh,
            scope,
            ReadOperation::ComposeValidate {
                configuration: configuration.clone(),
            },
            4096,
        )
        .await?;
        let hashes = hashes(
            &read(
                client,
                binding,
                &fresh,
                scope,
                ReadOperation::ComposeHashes {
                    configuration: configuration.clone(),
                },
                32768,
            )
            .await?,
        )?;
        let ids = ids(&read(
            client,
            binding,
            &fresh,
            scope,
            ReadOperation::ComposeExistingIds {
                configuration: configuration.clone(),
            },
            20 * 65,
        )
        .await?)?;
        let labels = read(
            client,
            binding,
            &fresh,
            scope,
            ReadOperation::ComposeExistingLabels {
                container_ids: ids.clone(),
            },
            32768,
        )
        .await?;
        verify_instances(configuration, ids, hashes, &labels).map_err(|e| e.in_scope(scope))
    })
    .await
    .map_err(|_| error(scope, ErrorCode::OperationTimedOut))?
}

pub(crate) async fn run(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: ComposeMutationRequest,
    expected: VerifiedRead,
    mut owner: crate::activity::Operation,
    current: super::mutations::Current,
) -> Result<ComposeMutationResponse, AppError> {
    let preflight = async {
        current()?;
        let observed = verify(
            client,
            options,
            binding,
            &VerifyComposeRequest {
                scope: request.scope.clone(),
                configuration: request.spec.configuration.clone(),
                acknowledged: true,
            },
        )
        .await?;
        if observed != expected {
            return Err(error(&request.scope, ErrorCode::ComposeProjectMismatch));
        }
        current()?;
        let (fresh, _) = super::probe::run(client, options).await;
        super::probe::transport_ready(&fresh)?;
        if fresh.daemon_id.as_deref() != Some(request.scope.daemon_id.as_str()) {
            return Err(error(&request.scope, ErrorCode::StaleSession));
        }
        current()?;
        Ok(fresh)
    };
    let preflight = tokio::time::timeout(Duration::from_secs(30), preflight)
        .await
        .map_err(|_| error(&request.scope, ErrorCode::OperationTimedOut))
        .and_then(|r| r);
    match preflight {
        Err(e) => owner.complete_compose(MutationTargetOutcome::Failed, false, Some(e.code))?,
        Ok(_) if owner.cancelled() => owner.complete_compose(
            MutationTargetOutcome::Cancelled,
            false,
            Some(ErrorCode::OperationCancelled),
        )?,
        Ok(fresh) => {
            let plan = match owner.dispatch_compose() {
                Ok(plan) => Some(plan),
                Err(e) if e.code == ErrorCode::OperationCancelled => {
                    owner.complete_compose(
                        MutationTargetOutcome::Cancelled,
                        false,
                        Some(e.code),
                    )?;
                    None
                }
                Err(e) => return Err(e),
            };
            if let Some(plan) = plan {
                // Persist Unknown for every target before a single Compose dispatch; never retry.
                let result = match binding.prepare(plan, &fresh) {
                    Ok(command) => match client.start_fixed(
                        command.encoded().into(),
                        Limits {
                            deadline: Duration::from_secs(command.timeout_seconds().into()),
                            stdout_bytes: 64 * 1024,
                            stderr_bytes: 64 * 1024,
                        },
                    ) {
                        Ok(job) => job.wait().await,
                        Err(_) => Err(RunError::Unavailable),
                    },
                    Err(_) => Err(RunError::Unavailable),
                };
                let success = result.as_ref().is_ok_and(|output| output.status.success());
                // A nonzero multi-service Compose exit can follow partial work. Per-target attribution is unknown.
                owner.complete_compose(
                    if success {
                        MutationTargetOutcome::Succeeded
                    } else {
                        MutationTargetOutcome::Unknown
                    },
                    true,
                    (!success).then_some(ErrorCode::TransportUnavailable),
                )?;
            }
        }
    }
    let results = owner.results()?;
    Ok(ComposeMutationResponse {
        scope: request.scope,
        spec: request.spec,
        outcome: crate::activity::response_outcome(&results),
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn configuration() -> ComposeConfiguration {
        ComposeConfiguration {
            project_name: "existing-project".into(),
            working_directory: "/srv/project's directory".into(),
            config_files: vec![
                "/srv/project's directory/base file.yml".into(),
                "/srv/project's directory/override.yml".into(),
            ],
        }
    }
    #[test]
    fn exact_hashes_names_and_existing_service_ids_must_agree() {
        let hash = "c".repeat(64);
        let cfg = configuration();
        let expected = vec![ContainerId("a".repeat(64))];
        let values = hashes(format!("web {hash}\n").as_bytes()).unwrap();
        let mut row = serde_json::json!({"id":expected[0],"project":cfg.project_name,"service":"web","hash":hash,"oneoff":"False"});
        let result = verify_instances(
            &cfg,
            expected.clone(),
            values.clone(),
            row.to_string().as_bytes(),
        )
        .unwrap();
        assert_eq!(result.services, vec!["web"]);
        for (field, value) in [
            ("project", "wrong-project"),
            ("service", "not-configured"),
            ("hash", "changed"),
            ("oneoff", "True"),
        ] {
            let mut invalid = row.clone();
            invalid[field] = value.into();
            assert_eq!(
                verify_instances(
                    &cfg,
                    expected.clone(),
                    values.clone(),
                    invalid.to_string().as_bytes()
                )
                .unwrap_err()
                .code,
                ErrorCode::ComposeProjectMismatch
            );
        }
        row["id"] = "b".repeat(64).into();
        assert!(verify_instances(&cfg, expected, values, row.to_string().as_bytes()).is_err());
        assert_eq!(
            ids(b"").unwrap_err().code,
            ErrorCode::ComposeProjectMismatch
        );
        assert!(hashes(b"web private-not-a-hash\n").is_err());
        assert!(hashes(format!("web {hash}\nweb {hash}\n").as_bytes()).is_err());
    }
    #[test]
    fn fixed_ordered_configuration_never_adds_deployment_or_unselected_dependency_actions() {
        let cfg = configuration();
        let plan = registry::read(&ReadOperation::ComposeValidate {
            configuration: cfg.clone(),
        })
        .unwrap();
        let file_args: Vec<_> = plan
            .args()
            .windows(2)
            .filter(|row| row[0] == "--file")
            .map(|row| row[1].clone())
            .collect();
        assert_eq!(file_args, cfg.config_files);
        let spec = ComposeActionSpec {
            verification_id: ComposeVerificationId(format!("v_{}", "a".repeat(32))),
            configuration: cfg.clone(),
            services: vec!["web".into()],
            container_ids: vec![ContainerId("b".repeat(64))],
            operation: ComposeActionOperation::Restart,
            timeout_seconds: 10,
        };
        let command =
            registry::confirmation(&ConfirmationOperation::Compose(spec.clone())).unwrap();
        assert!(
            command
                .args()
                .windows(2)
                .any(|w| w == ["restart", "--no-deps"])
        );
        assert_eq!(command.args().last().unwrap(), "web");
        assert!(
            !command
                .args()
                .iter()
                .any(|word| ["up", "down", "pull", "build", "rm"].contains(&word.as_str()))
        );
        for project in ["--bad", "UpperCase", "project; rm", "../project"] {
            let mut invalid = cfg.clone();
            invalid.project_name = project.into();
            assert!(registry::validate_compose(&invalid).is_err());
        }
        for path in [
            "relative.yml",
            "-",
            "oci://registry/config",
            "/srv/../etc/config",
            "/srv/config\n.yml",
        ] {
            let mut invalid = cfg.clone();
            invalid.config_files[0] = path.into();
            assert!(registry::validate_compose(&invalid).is_err());
        }
        let mut duplicate = spec;
        duplicate.services.push("web".into());
        assert!(registry::confirmation(&ConfirmationOperation::Compose(duplicate)).is_err());
    }
}
