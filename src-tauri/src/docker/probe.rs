//! Fixed read-only capability commands; no remote scripts, credentials or raw output cross IPC.
use super::*;
use crate::ssh::{
    multiplex::{Client, Connection},
    runner::{Captured, Limits, RunError},
};
use serde::Deserialize;
use std::{future::Future, pin::Pin, time::Duration};
type ProbeFuture<'a> = Pin<Box<dyn Future<Output = Result<Captured, RunError>> + Send + 'a>>;
pub(crate) trait Executor {
    fn execute(&self, encoded: String) -> ProbeFuture<'_>;
}
impl Executor for Client {
    fn execute(&self, encoded: String) -> ProbeFuture<'_> {
        Box::pin(async move {
            self.start_fixed(
                encoded,
                Limits {
                    deadline: Duration::from_secs(5),
                    stdout_bytes: 16 * 1024,
                    stderr_bytes: 16 * 1024,
                },
            )
            .map_err(|_| RunError::Unavailable)?
            .wait()
            .await
        })
    }
}
impl Executor for Connection {
    fn execute(&self, encoded: String) -> ProbeFuture<'_> {
        Box::pin(async move { self.client().execute(encoded).await })
    }
}
#[derive(Clone, Copy)]
enum Probe {
    Context,
    Version,
    Info,
    ComposePresent,
    Compose,
}
impl Probe {
    fn arguments(self) -> Vec<String> {
        match self {
            Self::Context => vec!["context", "inspect", "--format", r#"{"name":{{json .Name}},"endpoint":{{json .Endpoints.docker.Host}}}"#],
            Self::Version => vec!["version", "--format", r#"{"client":{{json .Client.Version}},"server":{{if .Server}}{{json .Server.Version}}{{else}}null{{end}}}"#],
            Self::Info => vec!["info", "--format", r#"{"id":{{json .ID}},"os":{{json .OSType}},"security":{{json .SecurityOptions}}}"#],
            Self::ComposePresent => vec!["compose", "version"],
            Self::Compose => vec!["compose", "version", "--format", "json"],
        }.into_iter().map(String::from).collect()
    }
}
impl DockerProbeReport {
    fn empty(sudo: bool) -> Self {
        Self {
            status: DockerProbeStatus::CommandFailed,
            context: None,
            endpoint: None,
            endpoint_kind: None,
            client_version: None,
            server_version: None,
            daemon_id: None,
            os: None,
            rootless: None,
            compose: ComposeAvailability::Unknown,
            compose_version: None,
            sudo,
        }
    }
}
/// Backend-only binding. Future dispatchers must refresh identity before using this preparation method.
/// No renderer-supplied report or unverified config may authorize a resource operation.
#[derive(Clone)]
pub(crate) struct VerifiedDocker {
    config: DockerCommandConfig,
    report: DockerProbeReport,
}
impl VerifiedDocker {
    pub fn prepare(
        &self,
        plan: CommandPlan,
        observed: &DockerProbeReport,
    ) -> Result<PreparedCommand, AppError> {
        if observed.status != DockerProbeStatus::Ready
            || observed.daemon_id != self.report.daemon_id
            || observed.endpoint != self.report.endpoint
            || observed.context != self.report.context
            || observed.sudo != self.report.sudo
        {
            return Err(AppError::new(ErrorCode::StaleSession));
        }
        prepare(plan, &self.config)
    }
}
pub(crate) async fn run(
    connection: &impl Executor,
    options: &DockerOptions,
) -> (DockerProbeReport, Option<VerifiedDocker>) {
    let mut report = DockerProbeReport::empty(options.sudo);
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        probe(connection, options, &mut report),
    )
    .await;
    match result {
        Ok(Ok(binding)) => (report, Some(binding)),
        Ok(Err(status)) => {
            report.status = status;
            (report, None)
        }
        Err(_) => {
            report.status = DockerProbeStatus::TimedOut;
            (report, None)
        }
    }
}
#[derive(Deserialize, PartialEq, Eq)]
struct Context {
    name: String,
    endpoint: String,
}
#[derive(Deserialize)]
struct Version {
    client: String,
    server: Option<String>,
}
#[derive(Deserialize)]
struct Info {
    id: String,
    os: String,
    security: Option<Vec<String>>,
}
#[derive(Deserialize)]
struct Compose {
    #[serde(rename = "version")]
    version: String,
}
fn valid_text(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
fn endpoint_kind(value: &str) -> Result<DockerEndpointKind, DockerProbeStatus> {
    // Endpoints carrying passwords/query tokens are never echoed or used. No implicit TCP exposure.
    if !valid_text(value, 4096) || value.contains(['?', '#', '\\']) {
        return Err(DockerProbeStatus::UnsupportedEndpoint);
    }
    if let Some(path) = value.strip_prefix("unix://") {
        validate_absolute_path(path).map_err(|_| DockerProbeStatus::UnsupportedEndpoint)?;
        return Ok(DockerEndpointKind::Unix);
    }
    for (scheme, kind) in [
        ("tcp://", DockerEndpointKind::Tcp),
        ("ssh://", DockerEndpointKind::Ssh),
    ] {
        if let Some(authority) = value.strip_prefix(scheme) {
            if authority.is_empty()
                || !authority.is_ascii()
                || authority.contains(['/', '%'])
                || authority.bytes().any(|b| b.is_ascii_whitespace())
            {
                break;
            }
            if let Some((user, host)) = authority.split_once('@')
                && (kind != DockerEndpointKind::Ssh
                    || user.is_empty()
                    || user.contains(':')
                    || host.is_empty()
                    || host.contains('@'))
            {
                break;
            }
            return Ok(kind);
        }
    }
    Err(DockerProbeStatus::UnsupportedEndpoint)
}
fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, DockerProbeStatus> {
    if bytes.len() > 16 * 1024 {
        return Err(DockerProbeStatus::OutputLimit);
    }
    serde_json::from_slice(bytes).map_err(|_| DockerProbeStatus::InvalidResponse)
}
async fn execute(
    executor: &impl Executor,
    config: &DockerCommandConfig,
    operation: Probe,
) -> Result<Captured, DockerProbeStatus> {
    let command = quoting::command(&config.arguments(operation.arguments()))
        .map_err(|_| DockerProbeStatus::InvalidResponse)?;
    executor
        .execute(command)
        .await
        .map_err(|error| match error {
            RunError::TimedOut => DockerProbeStatus::TimedOut,
            RunError::OutputLimit(_) => DockerProbeStatus::OutputLimit,
            _ => DockerProbeStatus::ConnectionFailed,
        })
}
fn failure(output: &Captured, config: &DockerCommandConfig) -> DockerProbeStatus {
    let text = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    if output.status.code() == Some(255) {
        return DockerProbeStatus::ConnectionFailed;
    }
    if config.sudo
        && (text.contains("a password is required")
            || text.contains("a terminal is required")
            || text.contains("no tty present"))
    {
        return DockerProbeStatus::SudoAuthenticationRequired;
    }
    if config.sudo
        && (text.contains("sudo:")
            || text.contains("sudoers")
            || text.contains("not allowed to execute"))
    {
        return DockerProbeStatus::SudoDenied;
    }
    if output.status.code() == Some(127)
        || text.contains("executable file not found")
        || text.contains("command not found")
    {
        return DockerProbeStatus::DockerMissing;
    }
    if text.contains("permission denied") {
        return DockerProbeStatus::PermissionDenied;
    }
    if text.contains("cannot connect to the docker daemon")
        || text.contains("is the docker daemon running")
        || text.contains("connection refused")
    {
        return DockerProbeStatus::DaemonUnavailable;
    }
    if text.contains("context") && (text.contains("not found") || text.contains("does not exist")) {
        return DockerProbeStatus::InvalidContext;
    }
    DockerProbeStatus::CommandFailed
}
async fn context(
    executor: &impl Executor,
    config: &DockerCommandConfig,
) -> Result<Context, DockerProbeStatus> {
    let output = execute(executor, config, Probe::Context).await?;
    if !output.status.success() {
        return Err(failure(&output, config));
    }
    let found: Context = parse(&output.stdout)?;
    DockerCommandConfig::new(None, Some(found.name.clone()), false)
        .map_err(|_| DockerProbeStatus::InvalidContext)?;
    endpoint_kind(&found.endpoint)?;
    if config
        .context()
        .is_some_and(|selected| selected != found.name)
    {
        return Err(DockerProbeStatus::IdentityChanged);
    }
    Ok(found)
}
async fn probe(
    executor: &impl Executor,
    options: &DockerOptions,
    report: &mut DockerProbeReport,
) -> Result<VerifiedDocker, DockerProbeStatus> {
    let mut config = DockerCommandConfig::from_options(options)
        .map_err(|_| DockerProbeStatus::InvalidContext)?;
    let selected = context(executor, &config).await?;
    report.context = Some(selected.name.clone());
    report.endpoint_kind = Some(endpoint_kind(&selected.endpoint)?);
    report.endpoint = Some(selected.endpoint.clone());
    config.context = Some(selected.name.clone());
    // Pin the dynamic default endpoint, including DOCKER_HOST/rootless overrides. Never context use.
    if selected.name == "default" {
        config.endpoint = Some(selected.endpoint.clone());
    }
    let version = execute(executor, &config, Probe::Version).await?;
    if let Ok(parsed) = parse::<Version>(&version.stdout) {
        if valid_text(&parsed.client, 128) {
            report.client_version = Some(parsed.client);
        }
        report.server_version = parsed.server.filter(|s| valid_text(s, 128));
    }
    if !version.status.success() {
        return Err(failure(&version, &config));
    }
    if report.client_version.is_none() || report.server_version.is_none() {
        return Err(DockerProbeStatus::InvalidResponse);
    }
    let info = execute(executor, &config, Probe::Info).await?;
    if !info.status.success() {
        return Err(failure(&info, &config));
    }
    let info: Info = parse(&info.stdout)?;
    if !valid_text(&info.id, 256) || !valid_text(&info.os, 32) {
        return Err(DockerProbeStatus::InvalidResponse);
    }
    let security = info.security;
    if security
        .as_ref()
        .is_some_and(|values| values.len() > 32 || values.iter().any(|s| !valid_text(s, 256)))
    {
        return Err(DockerProbeStatus::InvalidResponse);
    }
    report.daemon_id = Some(info.id);
    report.rootless = security.map(|values| {
        values
            .iter()
            .any(|s| s.split(',').any(|part| part == "name=rootless"))
    });
    report.os = Some(info.os.clone());
    if info.os != "linux" {
        return Err(DockerProbeStatus::UnsupportedOs);
    }
    // Probe without plugin-specific flags first: a missing plugin makes Docker reject --format
    // before reporting the unknown command. Only an available plugin receives its JSON flag.
    let compose = execute(executor, &config, Probe::ComposePresent).await?;
    if compose.status.success() {
        let json = execute(executor, &config, Probe::Compose).await?;
        if !json.status.success() {
            return Err(failure(&json, &config));
        }
        let compose: Compose = parse(&json.stdout)?;
        if !valid_text(&compose.version, 128) {
            return Err(DockerProbeStatus::InvalidResponse);
        }
        report.compose = ComposeAvailability::Available;
        report.compose_version = Some(compose.version);
    } else {
        let text = String::from_utf8_lossy(&compose.stderr).to_ascii_lowercase();
        if text.contains("compose")
            && (text.contains("not a docker command") || text.contains("unknown command"))
        {
            report.compose = ComposeAvailability::Absent;
        } else if compose.status.code() == Some(255) {
            return Err(DockerProbeStatus::ConnectionFailed);
        }
        // Broken/incompatible plugin stays explicitly unknown; Docker reads still work.
    }
    if context(executor, &config).await? != selected {
        return Err(DockerProbeStatus::IdentityChanged);
    }
    report.status = DockerProbeStatus::Ready;
    Ok(VerifiedDocker {
        config,
        report: report.clone(),
    })
}
#[cfg(test)]
mod tests;
