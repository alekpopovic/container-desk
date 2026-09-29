//! Volume metadata only. Returned paths are display text, never filesystem commands.
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
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_ITEMS: usize = 5000;
pub(crate) const LIST_TEMPLATE: &str =
    r#"{"name":{{json .Name}},"driver":{{json .Driver}},"scope":{{json .Scope}}}"#;
pub(crate) const MOUNTS_TEMPLATE: &str = r#"{"id":{{json .Id}},"name":{{json .Name}},"state":{{json .State.Status}},"mounts":{{json .Mounts}}}"#;
fn err(scope: &SessionScope, code: ErrorCode) -> AppError {
    AppError::new(code).in_scope(scope)
}
fn bound(scope: &SessionScope, n: usize, max: usize) -> Result<(), AppError> {
    if n > max {
        Err(err(scope, ErrorCode::ResourceLimit))
    } else {
        Ok(())
    }
}
fn field(scope: &SessionScope, value: Option<String>) -> Result<Option<String>, AppError> {
    if let Some(value) = &value {
        bound(scope, value.len(), 4096)?;
        if value.chars().any(char::is_control) {
            return Err(err(scope, ErrorCode::InvalidResponse));
        }
    }
    Ok(value.filter(|value| !value.is_empty()))
}
fn masked(
    scope: &SessionScope,
    values: Option<BTreeMap<String, String>>,
) -> Result<Vec<DetailValue>, AppError> {
    let values = values.unwrap_or_default();
    bound(scope, values.len(), 256)?;
    values
        .into_iter()
        .map(|(name, value)| {
            bound(scope, name.len(), 4096)?;
            bound(scope, value.len(), 64 * 1024)?;
            Ok(DetailValue {
                name,
                value: None,
                masked: true,
            })
        })
        .collect()
}
#[derive(Deserialize)]
struct ListRow {
    name: VolumeName,
    driver: Option<String>,
    scope: Option<String>,
}
pub(crate) fn parse_list(
    scope: &SessionScope,
    bytes: &[u8],
) -> Result<ListVolumesResponse, AppError> {
    scope.validate()?;
    bound(scope, bytes.len(), MAX_BYTES)?;
    let mut rows = BTreeMap::new();
    if !bytes.is_empty() {
        for line in bytes
            .strip_suffix(b"\n")
            .unwrap_or(bytes)
            .split(|b| *b == b'\n')
        {
            bound(scope, line.len(), 32768)?;
            let row: ListRow =
                serde_json::from_slice(line).map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
            row.name
                .validate()
                .map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
            let name = row.name.0.clone();
            let summary = VolumeSummary {
                scope: scope.clone(),
                name: row.name,
                driver: field(scope, row.driver)?,
                volume_scope: field(scope, row.scope)?,
            };
            if rows.insert(name, summary).is_some() {
                return Err(err(scope, ErrorCode::InvalidResponse));
            }
            bound(scope, rows.len(), MAX_ITEMS)?;
        }
    }
    Ok(ListVolumesResponse {
        scope: scope.clone(),
        volumes: rows.into_values().collect(),
    })
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InspectRow {
    name: VolumeName,
    driver: Option<String>,
    scope: Option<String>,
    created_at: Option<String>,
    mountpoint: Option<String>,
    labels: Option<BTreeMap<String, String>>,
    options: Option<BTreeMap<String, String>>,
}
pub(crate) fn parse_detail(
    request: &InspectVolumeRequest,
    bytes: &[u8],
) -> Result<VolumeDetail, AppError> {
    let scope = &request.scope;
    scope.validate()?;
    request.name.validate()?;
    bound(scope, bytes.len(), 2 * 1024 * 1024)?;
    let mut rows: Vec<InspectRow> =
        serde_json::from_slice(bytes).map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
    if rows.is_empty() {
        return Err(err(scope, ErrorCode::VolumeNotFound));
    }
    if rows.len() != 1 {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    let row = rows.remove(0);
    if row.name != request.name {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    Ok(VolumeDetail {
        summary: VolumeSummary {
            scope: scope.clone(),
            name: row.name,
            driver: field(scope, row.driver)?,
            volume_scope: field(scope, row.scope)?,
        },
        created_at: field(scope, row.created_at)?,
        mountpoint_reported: field(scope, row.mountpoint)?,
        labels: masked(scope, row.labels)?,
        options: masked(scope, row.options)?,
        references: vec![],
        reference_observation: ReferenceObservation::Incomplete,
        unresolved_container_ids: vec![],
    })
}
#[derive(Deserialize)]
struct Container {
    id: ContainerId,
    name: Option<String>,
    state: Option<String>,
    mounts: Option<Vec<Mount>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Mount {
    #[serde(rename = "Type")]
    kind: Option<String>,
    name: Option<String>,
    destination: Option<String>,
    #[serde(rename = "RW")]
    rw: Option<bool>,
}
fn mount_references(
    scope: &SessionScope,
    name: &VolumeName,
    expected: &[ContainerId],
    bytes: &[u8],
    allow_missing: bool,
) -> Result<(Vec<VolumeMountReference>, Vec<ContainerId>), AppError> {
    bound(scope, bytes.len(), 2 * 1024 * 1024)?;
    let mut seen = HashSet::new();
    let mut references = vec![];
    if !bytes.is_empty() {
        for line in bytes
            .strip_suffix(b"\n")
            .unwrap_or(bytes)
            .split(|b| *b == b'\n')
        {
            bound(scope, line.len(), 256 * 1024)?;
            let row: Container =
                serde_json::from_slice(line).map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
            if !expected.contains(&row.id) || !seen.insert(row.id.clone()) {
                return Err(err(scope, ErrorCode::InvalidResponse));
            }
            let display = field(scope, row.name)?
                .unwrap_or_else(|| "Unnamed container".into())
                .trim_start_matches('/')
                .to_owned();
            let state = field(scope, row.state)?.unwrap_or_else(|| "unknown".into());
            let mounts = row.mounts.unwrap_or_default();
            bound(scope, mounts.len(), 128)?;
            let mut destinations = HashSet::new();
            for mount in mounts {
                if mount.kind.as_deref() != Some("volume")
                    || mount.name.as_deref() != Some(name.0.as_str())
                {
                    continue;
                }
                let destination = field(scope, mount.destination)?;
                if !destinations.insert(destination.clone()) {
                    return Err(err(scope, ErrorCode::InvalidResponse));
                }
                references.push(VolumeMountReference {
                    container_id: row.id.clone(),
                    name: display.clone(),
                    state: state.clone(),
                    destination,
                    read_only: mount.rw.map(|rw| !rw),
                });
                bound(scope, references.len(), MAX_ITEMS)?;
            }
        }
    }
    let unresolved: Vec<_> = expected
        .iter()
        .filter(|id| !seen.contains(*id))
        .cloned()
        .collect();
    if !allow_missing && !unresolved.is_empty() {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    Ok((references, unresolved))
}
async fn capture(
    client: &Client,
    binding: &super::probe::VerifiedDocker,
    report: &DockerProbeReport,
    scope: &SessionScope,
    operation: ReadOperation,
    bytes: usize,
) -> Result<Captured, AppError> {
    let command = binding
        .prepare(registry::read(&operation)?, report)
        .map_err(|e| e.in_scope(scope))?;
    client
        .start_fixed(
            command.encoded().into(),
            Limits {
                deadline: Duration::from_secs(30),
                stdout_bytes: bytes,
                stderr_bytes: 64 * 1024,
            },
        )?
        .wait()
        .await
        .map_err(|error| {
            err(
                scope,
                match error {
                    RunError::Cancelled => ErrorCode::Disconnected,
                    RunError::TimedOut => ErrorCode::OperationTimedOut,
                    RunError::OutputLimit(_) => ErrorCode::ResourceLimit,
                    _ => ErrorCode::TransportUnavailable,
                },
            )
        })
}
fn failure(output: &Captured) -> Option<ErrorCode> {
    if output.status.success() {
        return None;
    }
    let diagnostic = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    Some(if output.status.code() == Some(255) {
        ErrorCode::Disconnected
    } else if diagnostic.contains("permission denied") || diagnostic.contains("access denied") {
        ErrorCode::PermissionDenied
    } else if diagnostic.contains("no such volume") {
        ErrorCode::VolumeNotFound
    } else if diagnostic.contains("no such container") || diagnostic.contains("no such object") {
        ErrorCode::ContainerNotFound
    } else {
        ErrorCode::TransportUnavailable
    })
}
async fn execute(
    client: &Client,
    binding: &super::probe::VerifiedDocker,
    report: &DockerProbeReport,
    scope: &SessionScope,
    operation: ReadOperation,
    bytes: usize,
) -> Result<Vec<u8>, AppError> {
    let result = capture(client, binding, report, scope, operation, bytes).await?;
    if let Some(error) = failure(&result) {
        Err(err(scope, error))
    } else {
        Ok(result.stdout)
    }
}
async fn fresh(
    client: &Client,
    options: &DockerOptions,
    scope: &SessionScope,
) -> Result<DockerProbeReport, AppError> {
    let (report, _) = super::probe::run(client, options).await;
    super::probe::transport_ready(&report)?;
    if report.daemon_id.as_deref() != Some(scope.daemon_id.as_str()) {
        return Err(err(scope, ErrorCode::StaleSession));
    }
    Ok(report)
}
pub(crate) async fn list(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    scope: &SessionScope,
) -> Result<ListVolumesResponse, AppError> {
    tokio::time::timeout(Duration::from_secs(30), async {
        let report = fresh(client, options, scope).await?;
        let bytes = execute(
            client,
            binding,
            &report,
            scope,
            ReadOperation::ListVolumes,
            MAX_BYTES,
        )
        .await?;
        parse_list(scope, &bytes)
    })
    .await
    .map_err(|_| err(scope, ErrorCode::OperationTimedOut))?
}
pub(crate) async fn inspect(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: &InspectVolumeRequest,
) -> Result<VolumeDetail, AppError> {
    tokio::time::timeout(Duration::from_secs(30), async {
        let scope = &request.scope;
        let report = fresh(client, options, scope).await?;
        let bytes = execute(
            client,
            binding,
            &report,
            scope,
            ReadOperation::InspectVolume {
                name: request.name.clone(),
            },
            2 * 1024 * 1024,
        )
        .await?;
        let mut total = bytes.len();
        let mut detail = parse_detail(request, &bytes)?;
        let raw = execute(
            client,
            binding,
            &report,
            scope,
            ReadOperation::VolumeContainerIds {
                name: request.name.clone(),
            },
            MAX_ITEMS * 65,
        )
        .await?;
        total += raw.len();
        bound(scope, total, MAX_BYTES)?;
        let mut seen = HashSet::new();
        let mut ids = vec![];
        for line in std::str::from_utf8(&raw)
            .map_err(|_| err(scope, ErrorCode::InvalidResponse))?
            .lines()
        {
            let id = ContainerId(line.into());
            id.validate()
                .map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
            if !seen.insert(id.clone()) {
                return Err(err(scope, ErrorCode::InvalidResponse));
            }
            ids.push(id);
            bound(scope, ids.len(), MAX_ITEMS)?;
        }
        let mut incomplete = false;
        for batch in ids.chunks(64) {
            let output = capture(
                client,
                binding,
                &report,
                scope,
                ReadOperation::InspectVolumeMounts {
                    container_ids: batch.to_vec(),
                },
                2 * 1024 * 1024,
            )
            .await?;
            let missing = match failure(&output) {
                None => false,
                Some(ErrorCode::ContainerNotFound) => true,
                Some(error) => return Err(err(scope, error)),
            };
            total += output.stdout.len();
            bound(scope, total, MAX_BYTES)?;
            let (references, unresolved) =
                mount_references(scope, &request.name, batch, &output.stdout, missing)?;
            incomplete |= missing;
            detail.references.extend(references);
            detail.unresolved_container_ids.extend(unresolved);
            bound(scope, detail.references.len(), MAX_ITEMS)?;
        }
        detail.references.sort_by(|a, b| {
            (&a.container_id.0, &a.destination).cmp(&(&b.container_id.0, &b.destination))
        });
        detail.reference_observation = if incomplete {
            ReferenceObservation::Incomplete
        } else if detail.references.is_empty() {
            ReferenceObservation::Unreferenced
        } else {
            ReferenceObservation::Referenced
        };
        Ok(detail)
    })
    .await
    .map_err(|_| err(&request.scope, ErrorCode::OperationTimedOut))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_tests::scope;
    #[test]
    fn named_anonymous_and_external_driver_metadata_are_safe_and_nullable() {
        let scope = scope();
        let anonymous = "a".repeat(64);
        let rows = format!(
            "{{\"name\":\"named-data\",\"driver\":\"local\",\"scope\":\"local\"}}\n{{\"name\":\"{anonymous}\",\"driver\":\"vendor/plugin:latest\",\"scope\":\"global\"}}\n"
        );
        let list = parse_list(&scope, rows.as_bytes()).unwrap();
        assert_eq!(list.volumes.len(), 2);
        assert_eq!(
            list.volumes[0].driver.as_deref(),
            Some("vendor/plugin:latest")
        );
        let request = InspectVolumeRequest {
            scope,
            name: VolumeName(anonymous),
        };
        let raw = serde_json::json!([{"Name":request.name,"Driver":"vendor/plugin:latest","Scope":"global","Labels":{"token":"private-label"},"Options":{"password":"private-option"},"Status":{"secret":"private-status"}}]);
        let detail = parse_detail(&request, &serde_json::to_vec(&raw).unwrap()).unwrap();
        assert!(detail.mountpoint_reported.is_none());
        assert!(
            detail
                .labels
                .iter()
                .chain(&detail.options)
                .all(|v| v.masked && v.value.is_none())
        );
        assert!(!serde_json::to_string(&detail).unwrap().contains("private-"));
        assert_eq!(
            detail.reference_observation,
            ReferenceObservation::Incomplete
        );
        assert_eq!(
            parse_detail(&request, b"[]").unwrap_err().code,
            ErrorCode::VolumeNotFound
        );
        assert!(parse_list(&request.scope, b"").unwrap().volumes.is_empty());
        assert!(parse_list(&request.scope, format!("{rows}{rows}").as_bytes()).is_err());
    }
    #[test]
    fn disappearing_container_preserves_partial_exact_mounts_without_false_unreferenced_claim() {
        let scope = scope();
        let volume = VolumeName("data".into());
        let ids = vec![ContainerId("a".repeat(64)), ContainerId("b".repeat(64))];
        let raw = serde_json::json!({"id":ids[0],"name":"/existing","state":"created","mounts":[
            {"Type":"volume","Name":"data","Destination":"/data","RW":false,"Source":"/private/source"},
            {"Type":"bind","Name":"data","Destination":"/ignored","RW":true},
            {"Type":"volume","Name":"different","Destination":"/other"}
        ]}).to_string();
        let (refs, missing) =
            mount_references(&scope, &volume, &ids, raw.as_bytes(), true).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].destination.as_deref(), Some("/data"));
        assert_eq!(refs[0].read_only, Some(true));
        assert_eq!(missing, vec![ids[1].clone()]);
        assert!(
            !serde_json::to_string(&refs)
                .unwrap()
                .contains("/private/source")
        );
        assert!(mount_references(&scope, &volume, &ids, raw.as_bytes(), false).is_err());
        assert!(mount_references(&scope, &volume, &ids[1..], raw.as_bytes(), true).is_err());
        assert!(
            mount_references(
                &scope,
                &volume,
                &ids,
                format!("{raw}\n{raw}").as_bytes(),
                true
            )
            .is_err()
        );
        assert_eq!(
            mount_references(&scope, &volume, &ids, b"", true)
                .unwrap()
                .1,
            ids
        );
    }
    #[test]
    fn only_valid_names_and_bounded_metadata_builders_are_admitted() {
        for raw in [
            "",
            "--all",
            "../data",
            "data;touch /tmp/x",
            "$(cat /secret)",
            "a\nb",
        ] {
            let name = VolumeName(raw.into());
            assert!(registry::read(&ReadOperation::InspectVolume { name: name.clone() }).is_err());
            assert!(registry::read(&ReadOperation::VolumeContainerIds { name }).is_err());
        }
        let name = VolumeName("named-data.1".into());
        for op in [
            ReadOperation::ListVolumes,
            ReadOperation::InspectVolume { name: name.clone() },
            ReadOperation::VolumeContainerIds { name },
        ] {
            let plan = registry::read(&op).unwrap();
            assert_eq!(plan.category(), &registry::OperationCategory::Read);
            assert!(!plan.args().iter().any(|arg| arg.contains("_data")
                || ["cat", "find", "du", "rm", "prune"].contains(&arg.as_str())));
        }
        assert!(
            registry::read(&ReadOperation::InspectVolumeMounts {
                container_ids: vec![ContainerId("a".repeat(64)); 65]
            })
            .is_err()
        );
        assert_eq!(
            parse_list(&scope(), &vec![b'x'; MAX_BYTES + 1])
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
    }
}
