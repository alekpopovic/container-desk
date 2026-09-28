//! Explicit, noninteractive access check. Never receives credentials or changes trust stores.
use super::runner::{Limits, RunError, Runner};
use crate::domain::*;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, symlink},
    path::Path,
    sync::Arc,
    time::Duration,
};

// First-obtained options in this Host * section also apply to native ProxyJump children via -F.
// Custom ProxyCommand/Match exec remain trusted executable configuration, not a sandbox.
pub(super) const POLICY: &str = "Host *\n\
 BatchMode yes\n PasswordAuthentication no\n KbdInteractiveAuthentication no\n NumberOfPasswordPrompts 0\n\
 StrictHostKeyChecking yes\n UpdateHostKeys no\n CheckHostIP no\n VerifyHostKeyDNS no\n\
 ForwardAgent no\n ForwardX11 no\n ForwardX11Trusted no\n Tunnel no\n ClearAllForwardings yes\n\
 PermitLocalCommand no\n ControlMaster no\n ControlPath none\n ControlPersist no\n\
 ForkAfterAuthentication no\n RemoteCommand none\n RequestTTY no\n\
 ConnectTimeout 10\n ServerAliveInterval 15\n ServerAliveCountMax 2\n";

/// Private policy overlay; source files are referenced, never copied or rewritten.
/// The process owner retains this lease until cancellation/deadline cleanup has reaped SSH.
pub(crate) struct PolicyConfig {
    pub runtime: Arc<super::runtime::RuntimeLease>,
    pub selection: SshSelection,
}
impl PolicyConfig {
    pub fn create(selection: &SshSelection) -> Result<Self, AppError> {
        super::resolver::arguments(selection)?;
        let invalid = || AppError::new(ErrorCode::InvalidConfigPath);
        let source = Path::new(&selection.config_path);
        let source_exists = match fs::metadata(source) {
            Ok(metadata) if metadata.is_file() => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && selection.use_default_config => {
                false
            }
            _ => return Err(invalid()),
        };
        let runtime = Arc::new(super::runtime::RuntimeLease::create()?);
        let policy = Self {
            selection: SshSelection {
                alias: selection.alias.clone(),
                config_path: runtime
                    .path()
                    .join("policy.conf")
                    .to_string_lossy()
                    .into_owned(),
                use_default_config: false,
            },
            runtime,
        };
        let mut text = POLICY.to_string();
        if source_exists {
            let link = policy.runtime.path().join("user.conf");
            symlink(source, &link).map_err(|_| invalid())?;
            text.push_str(&format!("Include {}\n", link.display()));
        }
        if selection.use_default_config {
            // Native user config precedes system config. Reset a trailing Host/Match condition.
            text.push_str("Host *\nInclude /etc/ssh/ssh_config\n");
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&policy.selection.config_path)
            .map_err(|_| invalid())?;
        file.write_all(text.as_bytes()).map_err(|_| invalid())?;
        Ok(policy)
    }
}

pub async fn probe(
    runner: &Runner,
    executable: &str,
    selection: SshSelection,
) -> Result<SshAccessReport, AppError> {
    let policy = PolicyConfig::create(&selection)?;
    let mut args = super::runner::structured_arguments(&policy.selection)?;
    // Fixed inert POSIX command: authentication and remote command execution, not Docker readiness.
    args.push(
        super::quoting::command(&["printf".into(), "containerdesk-access-ok".into()])?.into(),
    );
    let result = runner
        .start_owned(
            executable,
            args,
            Limits {
                deadline: Duration::from_secs(15),
                stdout_bytes: 1024,
                stderr_bytes: 64 * 1024,
            },
            Box::new(policy),
        )
        .map_err(map_start)?
        .wait()
        .await;
    report(selection, result)
}
pub(super) fn report(
    selection: SshSelection,
    result: Result<super::runner::Captured, RunError>,
) -> Result<SshAccessReport, AppError> {
    let (status, ssh_error) = match result {
        Ok(output) if output.status.success() && output.stdout == b"containerdesk-access-ok" => {
            (SshAccessStatus::Verified, None)
        }
        Ok(output) if output.status.code().is_some_and(|code| code != 255) => {
            (SshAccessStatus::RemoteCommandFailed, None)
        }
        Ok(output) => classify(&output.stderr),
        Err(RunError::TimedOut) => (SshAccessStatus::TimedOut, None),
        Err(RunError::Cancelled) => return Err(AppError::new(ErrorCode::OperationCancelled)),
        Err(RunError::OutputLimit(_)) => (SshAccessStatus::OutputLimit, None),
        Err(_) => (SshAccessStatus::ConnectionFailed, None),
    };
    Ok(SshAccessReport {
        selection,
        status,
        ssh_error: ssh_error.map(str::to_owned),
    })
}
fn map_start(error: RunError) -> AppError {
    AppError::new(match error {
        RunError::Unavailable => ErrorCode::SshUnavailable,
        RunError::Busy => ErrorCode::ResourceLimit,
        _ => ErrorCode::Internal,
    })
}
fn classify(stderr: &[u8]) -> (SshAccessStatus, Option<&'static str>) {
    let text = String::from_utf8_lossy(stderr);
    // Only recognized diagnostic phrases cross IPC. Raw stderr can contain banners, secrets and paths.
    if text.contains("REMOTE HOST IDENTIFICATION HAS CHANGED") {
        (
            SshAccessStatus::ChangedHostKey,
            Some("REMOTE HOST IDENTIFICATION HAS CHANGED!"),
        )
    } else if text.contains("you have requested strict checking") {
        (
            SshAccessStatus::UnknownHostKey,
            Some("No host key is known and you have requested strict checking."),
        )
    } else if text.contains("Host key verification failed") {
        (
            SshAccessStatus::HostKeyRejected,
            Some("Host key verification failed."),
        )
    } else if text.contains("Permission denied") || text.contains("no mutual signature supported") {
        (
            SshAccessStatus::AuthenticationFailed,
            Some("Permission denied (SSH authentication)."),
        )
    } else {
        (SshAccessStatus::ConnectionFailed, None)
    }
}

#[cfg(test)]
mod tests;
