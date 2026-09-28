//! Resolves only an explicitly selected trusted config/alias. `ssh -G` may run Match exec.
use crate::{diagnostics::validate_executable, domain::*};
use std::{ffi::OsString, io, path::Path, process::Stdio, time::Duration};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
    time::timeout,
};
const STDOUT_LIMIT: usize = 1024 * 1024;
const STDERR_LIMIT: usize = 64 * 1024;
const DEADLINE: Duration = Duration::from_secs(5);
fn error(code: ErrorCode) -> AppError {
    AppError::new(code)
}

pub fn arguments(selection: &SshSelection) -> Result<Vec<OsString>, AppError> {
    super::validate_alias(&selection.alias)?;
    if !Path::new(&selection.config_path).is_absolute()
        || selection.config_path.len() > 4096
        || selection.config_path.chars().any(char::is_control)
    {
        return Err(error(ErrorCode::InvalidConfigPath));
    }
    let mut args = vec![OsString::from("-G")];
    // Explicit -F suppresses the system-wide config. Preserve native defaults when none was selected.
    if !selection.use_default_config {
        args.extend([OsString::from("-F"), OsString::from(&selection.config_path)]);
    }
    args.extend([OsString::from("--"), OsString::from(&selection.alias)]);
    Ok(args)
}

// The group contains only this invocation and ordinary descendants from its trusted config.
// Never signal a reaped child's numeric group ID (it could have been reused).
struct OwnedProbe(Child);
impl OwnedProbe {
    fn stop_group(&self) {
        if let Some(pid) = self.0.id() {
            // SAFETY: process_group(0) created this child's dedicated group; pid fits pid_t.
            unsafe {
                libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
            }
        }
    }
}
impl Drop for OwnedProbe {
    fn drop(&mut self) {
        self.stop_group();
    }
}
async fn capture(reader: impl tokio::io::AsyncRead + Unpin, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .await?;
    if bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            "bounded diagnostic capture",
        ));
    }
    Ok(bytes)
}
pub async fn resolve(
    executable: &str,
    selection: SshSelection,
) -> Result<EffectiveSshConfig, AppError> {
    resolve_with_deadline(executable, selection, DEADLINE).await
}
async fn resolve_with_deadline(
    executable: &str,
    selection: SshSelection,
    deadline: Duration,
) -> Result<EffectiveSshConfig, AppError> {
    let args = arguments(&selection)?; // Validate alias before any possible process creation.
    let executable =
        validate_executable(executable).map_err(|_| error(ErrorCode::SshUnavailable))?;
    let child = Command::new(&executable)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| error(ErrorCode::SshUnavailable))?;
    let mut owned = OwnedProbe(child);
    let stdout = owned.0.stdout.take().expect("piped stdout");
    let stderr = owned.0.stderr.take().expect("piped stderr");
    let result = timeout(deadline, async {
        // Do not reap before readers finish: timeout cleanup must still own the group identity.
        let (stdout, _stderr) =
            tokio::try_join!(capture(stdout, STDOUT_LIMIT), capture(stderr, STDERR_LIMIT))?;
        let status = owned.0.wait().await?;
        Ok::<_, io::Error>((stdout, status))
    })
    .await;
    match result {
        Ok(Ok((stdout, status))) if status.success() => parse(&stdout, selection, &executable),
        other => {
            owned.stop_group();
            let _ = owned.0.kill().await;
            let _ = owned.0.wait().await;
            Err(error(match other {
                Err(_) => ErrorCode::OperationTimedOut,
                Ok(Err(e)) if e.kind() == io::ErrorKind::FileTooLarge => ErrorCode::ResourceLimit,
                _ => ErrorCode::SshResolutionFailed,
            }))
        }
    }
}
fn parse(
    bytes: &[u8],
    selection: SshSelection,
    executable: &Path,
) -> Result<EffectiveSshConfig, AppError> {
    let invalid = || error(ErrorCode::InvalidResponse);
    if bytes.len() > STDOUT_LIMIT {
        return Err(error(ErrorCode::ResourceLimit));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let mut fields = std::collections::HashMap::new();
    let mut has_proxy_command = false;
    for line in text.lines() {
        if line.len() > 16 * 1024 {
            return Err(error(ErrorCode::ResourceLimit));
        }
        let Some((key, value)) = line.split_once(' ') else {
            continue;
        };
        if key == "proxycommand" {
            has_proxy_command = !value.is_empty() && value != "none";
            continue;
        }
        if !["hostname", "user", "port", "proxyjump"].contains(&key) {
            continue;
        }
        if value.is_empty()
            || value.len() > 1024
            || value.chars().any(char::is_control)
            || fields.insert(key, value.to_string()).is_some()
        {
            return Err(invalid());
        }
    }
    let hostname = fields.remove("hostname").ok_or_else(invalid)?;
    let user = fields.remove("user").ok_or_else(invalid)?;
    let port = fields
        .remove("port")
        .ok_or_else(invalid)?
        .parse::<u16>()
        .map_err(|_| invalid())?;
    if port == 0 {
        return Err(invalid());
    }
    Ok(EffectiveSshConfig {
        selection,
        executable_path: executable.to_string_lossy().into_owned(),
        hostname,
        user,
        port,
        proxy_jump: fields.remove("proxyjump").filter(|jump| jump != "none"),
        has_proxy_command,
    })
}
#[cfg(test)]
mod tests;
