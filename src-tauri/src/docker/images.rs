//! Read-only, bounded image identity and metadata. Label values never enter IPC.
use crate::{
    domain::*,
    policy::registry::{self, ReadOperation},
    ssh::{
        multiplex::Client,
        runner::{Limits, RunError},
    },
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    time::Duration,
};
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_IMAGES: usize = 5000;
const MAX_ROWS: usize = 20000;
const MAX_REFERENCES: usize = 5000;
pub(crate) const REFERENCE_TEMPLATE: &str = r#"{"id":{{json .Id}},"image":{{json .Image}},"name":{{json .Name}},"state":{{json .State.Status}}}"#;
fn err(scope: &SessionScope, code: ErrorCode) -> AppError {
    AppError::new(code).in_scope(scope)
}
fn bounded(scope: &SessionScope, n: usize, limit: usize) -> Result<(), AppError> {
    if n > limit {
        Err(err(scope, ErrorCode::ResourceLimit))
    } else {
        Ok(())
    }
}
fn field(scope: &SessionScope, value: Option<String>) -> Result<Option<String>, AppError> {
    if let Some(value) = &value {
        bounded(scope, value.len(), 4096)?;
        if value.chars().any(char::is_control) {
            return Err(err(scope, ErrorCode::InvalidResponse));
        }
    }
    Ok(value.filter(|v| !v.is_empty() && v != "<none>"))
}
fn image_id(scope: &SessionScope, raw: &str) -> Result<ImageId, AppError> {
    let id = ImageId(raw.into());
    id.validate()
        .map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
    Ok(ImageId(format!(
        "sha256:{}",
        raw.strip_prefix("sha256:").unwrap_or(raw)
    )))
}
fn strings(scope: &SessionScope, values: Option<Vec<String>>) -> Result<Vec<String>, AppError> {
    let values = values.unwrap_or_default();
    bounded(scope, values.len(), 128)?;
    let mut result = BTreeSet::new();
    for value in values {
        if let Some(value) = field(scope, Some(value))? {
            result.insert(value);
        }
    }
    Ok(result.into_iter().collect())
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListRow {
    #[serde(rename = "ID")]
    id: String,
    repository: Option<String>,
    tag: Option<String>,
    digest: Option<String>,
    size: Option<String>,
    created_at: Option<String>,
}
pub(crate) fn parse_list(
    request: &ListImagesRequest,
    bytes: &[u8],
) -> Result<ListImagesResponse, AppError> {
    let scope = &request.scope;
    scope.validate()?;
    bounded(scope, bytes.len(), MAX_BYTES)?;
    let mut images: BTreeMap<String, ImageSummary> = BTreeMap::new();
    if !bytes.is_empty() {
        for (index, line) in bytes
            .strip_suffix(b"\n")
            .unwrap_or(bytes)
            .split(|c| *c == b'\n')
            .enumerate()
        {
            bounded(scope, index + 1, MAX_ROWS)?;
            bounded(scope, line.len(), 32768)?;
            let row: ListRow =
                serde_json::from_slice(line).map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
            let id = image_id(scope, &row.id)?;
            let repository = field(scope, row.repository)?;
            let tag = field(scope, row.tag)?;
            let digest = field(scope, row.digest)?;
            let size = field(scope, row.size)?;
            let created = field(scope, row.created_at)?;
            let image = images.entry(id.0.clone()).or_insert_with(|| ImageSummary {
                scope: scope.clone(),
                id,
                tags: vec![],
                digests: vec![],
                size_reported: size.clone(),
                created_at_reported: created.clone(),
            });
            if image.size_reported != size || image.created_at_reported != created {
                return Err(err(scope, ErrorCode::InvalidResponse));
            }
            if let (Some(repository), Some(tag)) = (&repository, tag) {
                let tag = format!("{repository}:{tag}");
                bounded(scope, tag.len(), 4096)?;
                if !image.tags.contains(&tag) {
                    image.tags.push(tag);
                }
            }
            if let (Some(repository), Some(digest)) = (&repository, digest) {
                image_id(scope, &digest)?;
                let digest = format!("{repository}@{digest}");
                bounded(scope, digest.len(), 4096)?;
                if !image.digests.contains(&digest) {
                    image.digests.push(digest);
                }
            }
            bounded(scope, image.tags.len(), 128)?;
            bounded(scope, image.digests.len(), 128)?;
            bounded(scope, images.len(), MAX_IMAGES)?;
        }
    }
    for image in images.values_mut() {
        image.tags.sort();
        image.digests.sort();
    }
    Ok(ListImagesResponse {
        scope: scope.clone(),
        dangling_only: request.dangling_only,
        images: images.into_values().collect(),
    })
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InspectRow {
    id: String,
    repo_tags: Option<Vec<String>>,
    repo_digests: Option<Vec<String>>,
    size: Option<u64>,
    created: Option<String>,
    os: Option<String>,
    architecture: Option<String>,
    variant: Option<String>,
    config: Option<Config>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Config {
    labels: Option<BTreeMap<String, String>>,
}
pub(crate) fn parse_detail(
    request: &InspectImageRequest,
    bytes: &[u8],
) -> Result<ImageDetail, AppError> {
    let scope = &request.scope;
    scope.validate()?;
    request.image_id.validate()?;
    bounded(scope, bytes.len(), 2 * 1024 * 1024)?;
    let mut rows: Vec<InspectRow> =
        serde_json::from_slice(bytes).map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
    if rows.is_empty() {
        return Err(err(scope, ErrorCode::ImageNotFound));
    }
    if rows.len() != 1 {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    let row = rows.remove(0);
    let id = image_id(scope, &row.id)?;
    if id != image_id(scope, &request.image_id.0)?
        || row.size.is_some_and(|v| v > 9_007_199_254_740_991)
    {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    let raw_labels = row.config.unwrap_or_default().labels.unwrap_or_default();
    bounded(scope, raw_labels.len(), 256)?;
    let mut labels = vec![];
    for (name, value) in raw_labels {
        bounded(scope, name.len(), 4096)?;
        bounded(scope, value.len(), 64 * 1024)?;
        labels.push(DetailValue {
            name,
            value: None,
            masked: true,
        });
    }
    Ok(ImageDetail {
        scope: scope.clone(),
        id,
        tags: strings(scope, row.repo_tags)?,
        digests: strings(scope, row.repo_digests)?,
        size_bytes: row.size,
        created_at: field(scope, row.created)?,
        os: field(scope, row.os)?,
        architecture: field(scope, row.architecture)?,
        variant: field(scope, row.variant)?,
        labels,
        containers: vec![],
    })
}
#[derive(Deserialize)]
struct Reference {
    id: ContainerId,
    image: String,
    name: String,
    state: String,
}
fn references(
    scope: &SessionScope,
    id: &ImageId,
    expected: &[ContainerId],
    bytes: &[u8],
) -> Result<Vec<ImageContainerReference>, AppError> {
    bounded(scope, bytes.len(), 2 * 1024 * 1024)?;
    let mut seen = HashSet::new();
    let mut references = vec![];
    for line in bytes
        .strip_suffix(b"\n")
        .unwrap_or(bytes)
        .split(|c| *c == b'\n')
    {
        bounded(scope, line.len(), 32768)?;
        let row: Reference =
            serde_json::from_slice(line).map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
        if !expected.contains(&row.id) || !seen.insert(row.id.clone()) {
            return Err(err(scope, ErrorCode::InvalidResponse));
        }
        // Docker's ancestor filter also returns descendants. Only exact immutable image references qualify.
        if image_id(scope, &row.image)? == *id {
            references.push(ImageContainerReference {
                container_id: row.id,
                name: field(scope, Some(row.name))?
                    .unwrap_or_else(|| "Unnamed container".into())
                    .trim_start_matches('/')
                    .into(),
                state: field(scope, Some(row.state))?.unwrap_or_else(|| "unknown".into()),
            });
        }
    }
    if seen.len() != expected.len() {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    Ok(references)
}
async fn execute(
    client: &Client,
    binding: &super::probe::VerifiedDocker,
    report: &DockerProbeReport,
    scope: &SessionScope,
    operation: ReadOperation,
    max_bytes: usize,
) -> Result<Vec<u8>, AppError> {
    let image_inspect = matches!(operation, ReadOperation::InspectImage { .. });
    let command = binding
        .prepare(registry::read(&operation)?, report)
        .map_err(|e| e.in_scope(scope))?;
    let output = client
        .start_fixed(
            command.encoded().into(),
            Limits {
                deadline: Duration::from_secs(30),
                stdout_bytes: max_bytes,
                stderr_bytes: 64 * 1024,
            },
        )?
        .wait()
        .await
        .map_err(|error| {
            err(
                scope,
                match error {
                    RunError::TimedOut => ErrorCode::OperationTimedOut,
                    RunError::Cancelled => ErrorCode::Disconnected,
                    RunError::OutputLimit(_) => ErrorCode::ResourceLimit,
                    _ => ErrorCode::TransportUnavailable,
                },
            )
        })?;
    if !output.status.success() {
        let diagnostic = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        return Err(err(
            scope,
            if output.status.code() == Some(255) {
                ErrorCode::Disconnected
            } else if diagnostic.contains("permission denied") {
                ErrorCode::PermissionDenied
            } else if image_inspect
                && (diagnostic.contains("no such image") || diagnostic.contains("no such object"))
            {
                ErrorCode::ImageNotFound
            } else {
                ErrorCode::TransportUnavailable
            },
        ));
    }
    Ok(output.stdout)
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
    request: &ListImagesRequest,
) -> Result<ListImagesResponse, AppError> {
    tokio::time::timeout(Duration::from_secs(30), async {
        let report = fresh(client, options, &request.scope).await?;
        let bytes = execute(
            client,
            binding,
            &report,
            &request.scope,
            ReadOperation::ListImages {
                dangling_only: request.dangling_only,
            },
            MAX_BYTES,
        )
        .await?;
        parse_list(request, &bytes)
    })
    .await
    .map_err(|_| err(&request.scope, ErrorCode::OperationTimedOut))?
}
pub(crate) async fn inspect(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: &InspectImageRequest,
) -> Result<ImageDetail, AppError> {
    tokio::time::timeout(Duration::from_secs(30), async {
        let scope = &request.scope;
        let report = fresh(client, options, scope).await?;
        let bytes = execute(
            client,
            binding,
            &report,
            scope,
            ReadOperation::InspectImage {
                image_id: request.image_id.clone(),
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
            ReadOperation::ImageContainerIds {
                image_id: detail.id.clone(),
            },
            MAX_REFERENCES * 65,
        )
        .await?;
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
            bounded(scope, ids.len(), MAX_REFERENCES)?;
        }
        for batch in ids.chunks(64) {
            let raw = execute(
                client,
                binding,
                &report,
                scope,
                ReadOperation::InspectImageReferences {
                    container_ids: batch.to_vec(),
                },
                2 * 1024 * 1024,
            )
            .await?;
            total += raw.len();
            bounded(scope, total, MAX_BYTES)?;
            detail
                .containers
                .extend(references(scope, &detail.id, batch, &raw)?);
        }
        detail
            .containers
            .sort_by(|a, b| a.container_id.0.cmp(&b.container_id.0));
        Ok(detail)
    })
    .await
    .map_err(|_| err(&request.scope, ErrorCode::OperationTimedOut))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_tests::scope;
    fn row(id: &str, tag: &str) -> serde_json::Value {
        serde_json::json!({"ID":format!("sha256:{}",id.repeat(64)),"Repository":if tag=="<none>" {"<none>"} else {"same/repository"},"Tag":tag,"Digest":"<none>","Size":"12.3MB","CreatedAt":"2026-09-29 00:00:00 +0000 UTC"})
    }
    #[test]
    fn deduplicates_full_identity_preserves_tags_digests_and_missing_tags_per_host() {
        let request = ListImagesRequest {
            scope: scope(),
            dangling_only: false,
        };
        let mut digest = row("a", "latest");
        digest["Digest"] = format!("sha256:{}", "d".repeat(64)).into();
        let bytes = [
            row("a", "stable"),
            row("a", "latest"),
            digest,
            row("b", "<none>"),
        ]
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join("\n");
        let parsed = parse_list(&request, bytes.as_bytes()).unwrap();
        assert_eq!(parsed.images.len(), 2);
        assert_eq!(
            parsed.images[0].tags,
            vec!["same/repository:latest", "same/repository:stable"]
        );
        assert_eq!(
            parsed.images[0].digests,
            vec![format!("same/repository@sha256:{}", "d".repeat(64))]
        );
        assert!(parsed.images[1].tags.is_empty());
        let mut other = request.clone();
        other.scope.selection.host_id = HostId(format!("h_{}", "f".repeat(32)));
        other.scope.daemon_id = "other-engine".into();
        let foreign = parse_list(&other, row("c", "latest").to_string().as_bytes()).unwrap();
        assert_eq!(foreign.images[0].tags[0], parsed.images[0].tags[0]);
        assert_ne!(foreign.images[0].scope, parsed.images[0].scope);
        assert_ne!(foreign.images[0].id, parsed.images[0].id);
        assert!(parse_list(&request, b"").unwrap().images.is_empty());
    }
    #[test]
    fn metadata_masks_all_labels_omits_environment_and_validates_identity_and_bounds() {
        let request = InspectImageRequest {
            scope: scope(),
            image_id: ImageId(format!("sha256:{}", "a".repeat(64))),
        };
        let raw = serde_json::json!([{"Id":request.image_id,"RepoTags":null,"RepoDigests":[],"Size":0,"Created":"2026-09-29T00:00:00Z","Os":"linux","Architecture":"amd64","Config":{"Env":["TOKEN=private-image-environment"],"Labels":{"innocent":"private-image-label","token":"private-token-label"},"Cmd":["private-command"]}}]);
        let detail = parse_detail(&request, &serde_json::to_vec(&raw).unwrap()).unwrap();
        assert_eq!(detail.labels.len(), 2);
        assert!(detail.labels.iter().all(|v| v.masked && v.value.is_none()));
        let encoded = serde_json::to_string(&detail).unwrap();
        assert!(!encoded.contains("private-"));
        let mut wrong = raw.clone();
        wrong[0]["Id"] = format!("sha256:{}", "b".repeat(64)).into();
        assert!(parse_detail(&request, &serde_json::to_vec(&wrong).unwrap()).is_err());
        let mut oversized = raw;
        oversized[0]["Size"] = serde_json::json!(9_007_199_254_740_992u64);
        assert!(parse_detail(&request, &serde_json::to_vec(&oversized).unwrap()).is_err());
        assert_eq!(
            parse_detail(&request, b"[]").unwrap_err().code,
            ErrorCode::ImageNotFound
        );
        let mut invalid = row("a", "latest");
        invalid["ID"] = "sha256:short".into();
        assert!(
            parse_list(
                &ListImagesRequest {
                    scope: scope(),
                    dangling_only: false
                },
                invalid.to_string().as_bytes()
            )
            .is_err()
        );
    }
    #[test]
    fn ancestor_candidates_are_rechecked_by_exact_image_and_missing_or_foreign_ids_fail() {
        let selected = ImageId(format!("sha256:{}", "a".repeat(64)));
        let ids = vec![ContainerId("b".repeat(64)), ContainerId("c".repeat(64))];
        let exact =
            serde_json::json!({"id":ids[0],"image":selected,"name":"/exact","state":"created"});
        let derived = serde_json::json!({"id":ids[1],"image":format!("sha256:{}","d".repeat(64)),"name":"/derived","state":"running"});
        let bytes = format!("{exact}\n{derived}\n");
        let result = references(&scope(), &selected, &ids, bytes.as_bytes()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_id, ids[0]);
        assert!(references(&scope(), &selected, &ids, exact.to_string().as_bytes()).is_err());
        assert!(references(&scope(), &selected, &ids[..1], bytes.as_bytes()).is_err());
    }
    #[test]
    fn fixed_image_filters_and_reference_inspection_reject_hostile_ids() {
        for dangling_only in [false, true] {
            let command = registry::read(&ReadOperation::ListImages { dangling_only }).unwrap();
            assert_eq!(
                command.args().contains(&"dangling=true".into()),
                dangling_only
            );
            assert_eq!(command.category(), &registry::OperationCategory::Read);
        }
        for raw in ["--all", "latest", "sha256:bad;touch /tmp/not-allowed"] {
            assert!(
                registry::read(&ReadOperation::ImageContainerIds {
                    image_id: ImageId(raw.into())
                })
                .is_err()
            );
            assert!(
                registry::read(&ReadOperation::InspectImage {
                    image_id: ImageId(raw.into())
                })
                .is_err()
            );
        }
        assert!(
            registry::read(&ReadOperation::InspectImageReferences {
                container_ids: vec![ContainerId("a".repeat(64)); 65]
            })
            .is_err()
        );
    }
}
