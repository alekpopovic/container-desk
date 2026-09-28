//! Resolves only an explicitly selected trusted config/alias. `ssh -G` may run Match exec.
use super::runner::{Limits, RunError, Runner};
use crate::domain::*;
use std::{ffi::OsString, path::Path, time::Duration};
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

pub async fn resolve(
    runner: &Runner,
    executable: &str,
    selection: SshSelection,
) -> Result<EffectiveSshConfig, AppError> {
    resolve_with_deadline(runner, executable, selection, DEADLINE).await
}
async fn resolve_with_deadline(
    runner: &Runner,
    executable: &str,
    selection: SshSelection,
    deadline: Duration,
) -> Result<EffectiveSshConfig, AppError> {
    let args = arguments(&selection)?;
    let executable = crate::diagnostics::validate_executable(executable)
        .map_err(|_| error(ErrorCode::SshUnavailable))?;
    let output = runner
        .start(
            executable
                .to_str()
                .ok_or_else(|| error(ErrorCode::SshUnavailable))?,
            args,
            Limits {
                deadline,
                stdout_bytes: STDOUT_LIMIT,
                stderr_bytes: STDERR_LIMIT,
            },
        )
        .map_err(map_failure)?
        .wait()
        .await
        .map_err(map_failure)?;
    if !output.status.success() {
        return Err(error(ErrorCode::SshResolutionFailed));
    }
    parse(&output.stdout, selection, &executable)
}
fn map_failure(failure: RunError) -> AppError {
    error(match failure {
        RunError::Unavailable => ErrorCode::SshUnavailable,
        RunError::Busy | RunError::OutputLimit(_) => ErrorCode::ResourceLimit,
        RunError::TimedOut => ErrorCode::OperationTimedOut,
        RunError::Cancelled => ErrorCode::OperationCancelled,
        RunError::InvalidInput | RunError::Io => ErrorCode::SshResolutionFailed,
    })
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
