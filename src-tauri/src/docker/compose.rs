//! Discovery only: reported paths never become local filesystem access or executable configuration.
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
    collections::{BTreeMap, HashSet},
    time::Duration,
};
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_INSTANCES: usize = 5000;
const MAX_PROJECTS: usize = 1000;
pub(crate) const LABEL_TEMPLATE: &str = r#"{"id":{{json .Id}},"name":{{json .Name}},"state":{{json .State.Status}},"project":{{json (index .Config.Labels "com.docker.compose.project")}},"service":{{json (index .Config.Labels "com.docker.compose.service")}},"configFiles":{{json (index .Config.Labels "com.docker.compose.project.config_files")}},"workingDir":{{json (index .Config.Labels "com.docker.compose.project.working_dir")}}}"#;
fn err(scope: &SessionScope, code: ErrorCode) -> AppError {
    AppError::new(code).in_scope(scope)
}
fn field(value: Option<String>) -> Result<Option<String>, ()> {
    if value
        .as_ref()
        .is_some_and(|s| s.len() > 4096 || s.chars().any(char::is_control))
    {
        return Err(());
    }
    Ok(value.filter(|s| !s.is_empty()))
}
pub(crate) fn group_labels(labels: &BTreeMap<String, String>) -> Option<ComposeLabels> {
    let project = field(labels.get("com.docker.compose.project").cloned()).ok()??;
    let service = field(labels.get("com.docker.compose.service").cloned()).ok()?;
    Some(ComposeLabels { project, service })
}
fn project(name: String) -> ComposeProject {
    ComposeProject {
        name,
        status: None,
        from_plugin: false,
        from_labels: false,
        config_files_reported: vec![],
        working_directories_reported: vec![],
        configuration: ComposeConfigurationStatus::Unverified,
        instances: vec![],
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ProjectRecord {
    name: String,
    status: Option<String>,
    config_files: Option<String>,
}
fn plugin_projects(bytes: &[u8]) -> Result<BTreeMap<String, ComposeProject>, ()> {
    if bytes.len() > MAX_BYTES {
        return Err(());
    }
    let rows: Vec<ProjectRecord> = serde_json::from_slice(bytes).map_err(|_| ())?;
    if rows.len() > MAX_PROJECTS {
        return Err(());
    }
    let mut result = BTreeMap::new();
    for row in rows {
        let name = field(Some(row.name))?.ok_or(())?;
        let mut p = project(name.clone());
        p.from_plugin = true;
        p.status = field(row.status)?;
        if let Some(files) = field(row.config_files)? {
            p.config_files_reported.push(files);
        }
        if result.insert(name, p).is_some() {
            return Err(());
        }
    }
    Ok(result)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LabelRecord {
    id: ContainerId,
    name: Option<String>,
    state: Option<String>,
    project: Option<String>,
    service: Option<String>,
    config_files: Option<String>,
    working_dir: Option<String>,
}
fn merge_labels(
    bytes: &[u8],
    expected: &[ContainerId],
    projects: &mut BTreeMap<String, ComposeProject>,
) -> Result<(), ()> {
    if bytes.len() > MAX_BYTES {
        return Err(());
    }
    let mut seen = HashSet::new();
    for line in bytes
        .strip_suffix(b"\n")
        .unwrap_or(bytes)
        .split(|c| *c == b'\n')
    {
        if line.len() > 32768 {
            return Err(());
        }
        let r: LabelRecord = serde_json::from_slice(line).map_err(|_| ())?;
        r.id.validate().map_err(|_| ())?;
        if !expected.contains(&r.id) || !seen.insert(r.id.clone()) {
            return Err(());
        }
        let Some(name) = field(r.project)? else {
            continue;
        };
        let p = projects
            .entry(name.clone())
            .or_insert_with(|| project(name));
        p.from_labels = true;
        for (value, target) in [
            (field(r.config_files)?, &mut p.config_files_reported),
            (field(r.working_dir)?, &mut p.working_directories_reported),
        ] {
            if let Some(value) = value
                && !target.contains(&value)
            {
                if target.len() >= 128 {
                    return Err(());
                }
                target.push(value);
            }
        }
        p.instances.push(ComposeInstance {
            container_id: r.id,
            name: field(r.name)?
                .unwrap_or_else(|| "Unnamed container".into())
                .trim_start_matches('/')
                .into(),
            service: field(r.service)?,
            state: field(r.state)?.unwrap_or_else(|| "unknown".into()),
        });
    }
    if seen.len() != expected.len() || projects.len() > MAX_PROJECTS {
        return Err(());
    }
    Ok(())
}
async fn execute(
    client: &Client,
    binding: &super::probe::VerifiedDocker,
    report: &DockerProbeReport,
    scope: &SessionScope,
    operation: ReadOperation,
) -> Result<Captured, AppError> {
    let command = binding
        .prepare(registry::read(&operation)?, report)
        .map_err(|e| e.in_scope(scope))?;
    client
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
        })
}
fn success(scope: &SessionScope, result: Captured) -> Result<Vec<u8>, AppError> {
    if result.status.success() {
        Ok(result.stdout)
    } else {
        Err(err(
            scope,
            if result.status.code() == Some(255) {
                ErrorCode::Disconnected
            } else {
                ErrorCode::TransportUnavailable
            },
        ))
    }
}
pub(crate) async fn read(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    scope: &SessionScope,
) -> Result<ListComposeResponse, AppError> {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (fresh, _) = super::probe::run(client, options).await;
        if fresh.daemon_id.as_deref() != Some(scope.daemon_id.as_str()) {
            return Err(err(scope, ErrorCode::StaleSession));
        }
        let mut projects = BTreeMap::new();
        let mut listing_error = None;
        let mut total_bytes = 0usize;
        if fresh.compose == ComposeAvailability::Available {
            let read = execute(client, binding, &fresh, scope, ReadOperation::ListCompose).await;
            match read.and_then(|v| success(scope, v)).and_then(|v| {
                total_bytes += v.len();
                plugin_projects(&v).map_err(|_| err(scope, ErrorCode::InvalidResponse))
            }) {
                Ok(p) => projects = p,
                Err(error)
                    if matches!(
                        error.code,
                        ErrorCode::TransportUnavailable | ErrorCode::InvalidResponse
                    ) =>
                {
                    listing_error = Some(error.code)
                }
                Err(error) => return Err(error),
            }
        }
        let ids = success(
            scope,
            execute(
                client,
                binding,
                &fresh,
                scope,
                ReadOperation::ComposeContainerIds,
            )
            .await?,
        )?;
        let raw = std::str::from_utf8(&ids).map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
        let mut unique = HashSet::new();
        let mut ids = vec![];
        for line in raw.lines() {
            let id = ContainerId(line.into());
            id.validate()
                .map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
            if !unique.insert(id.clone()) {
                return Err(err(scope, ErrorCode::InvalidResponse));
            }
            ids.push(id);
            if ids.len() > MAX_INSTANCES {
                return Err(err(scope, ErrorCode::ResourceLimit));
            }
        }
        for batch in ids.chunks(64) {
            let rows = success(
                scope,
                execute(
                    client,
                    binding,
                    &fresh,
                    scope,
                    ReadOperation::InspectComposeLabels {
                        container_ids: batch.to_vec(),
                    },
                )
                .await?,
            )?;
            total_bytes += rows.len();
            if total_bytes > 16 * 1024 * 1024 {
                return Err(err(scope, ErrorCode::ResourceLimit));
            }
            merge_labels(&rows, batch, &mut projects)
                .map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
        }
        let (after, _) = super::probe::run(client, options).await;
        binding
            .prepare(registry::read(&ReadOperation::ListCompose)?, &after)
            .map_err(|e| e.in_scope(scope))?;
        Ok(ListComposeResponse {
            scope: scope.clone(),
            plugin: fresh.compose,
            listing_error,
            projects: projects.into_values().collect(),
        })
    })
    .await
    .map_err(|_| err(scope, ErrorCode::OperationTimedOut))?
}
#[cfg(test)]
mod tests;
