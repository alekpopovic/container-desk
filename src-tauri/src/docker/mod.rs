//! Typed command preparation only. Dispatch still requires current scope and backend authorization.
use crate::{
    domain::*,
    policy::registry::{CommandPlan, OperationCategory, ResponseKind},
    ssh::{quoting, runner},
};
use std::{ffi::OsString, fmt};
pub(crate) mod inspect;
pub mod listing;
pub(crate) mod logs;
pub(crate) mod probe;
pub(crate) mod stats;

#[derive(Clone, Debug, Default)]
pub struct DockerCommandConfig {
    executable: Option<String>,
    context: Option<String>,
    sudo: bool,
    endpoint: Option<String>,
}
impl DockerCommandConfig {
    pub fn new(
        executable: Option<String>,
        context: Option<String>,
        sudo: bool,
    ) -> Result<Self, AppError> {
        if let Some(path) = &executable {
            validate_absolute_path(path)?;
        }
        if context.as_ref().is_some_and(|context| {
            context.is_empty()
                || context.len() > 256
                || !context.as_bytes()[0].is_ascii_alphanumeric()
                || !context
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        }) {
            return Err(AppError::new(ErrorCode::InvalidRemoteArgument));
        }
        Ok(Self {
            executable,
            context,
            sudo,
            endpoint: None,
        })
    }
    pub fn from_options(options: &DockerOptions) -> Result<Self, AppError> {
        Self::new(
            options.executable.clone(),
            options.context.clone(),
            options.sudo,
        )
    }
    fn arguments(&self, operation: impl IntoIterator<Item = String>) -> Vec<String> {
        let mut args = Vec::new();
        if self.sudo {
            args.extend(["sudo".into(), "-n".into(), "--".into()]);
        }
        args.push(self.executable().to_string());
        if let Some(endpoint) = &self.endpoint {
            args.extend(["--host".into(), endpoint.clone()]);
        } else if let Some(context) = &self.context {
            args.extend(["--context".into(), context.clone()]);
        }
        args.extend(operation);
        args
    }
    pub fn executable(&self) -> &str {
        self.executable.as_deref().unwrap_or("docker")
    }
    pub fn context(&self) -> Option<&str> {
        self.context.as_deref()
    }
    pub fn sudo(&self) -> bool {
        self.sudo
    }
}
/// Remote Linux path syntax only, never a local filesystem probe or shell-profile evaluator.
pub fn validate_absolute_path(path: &str) -> Result<(), AppError> {
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.ends_with('/')
        || path.len() > 4096
        || path.chars().any(char::is_control)
        || path.split('/').any(|part| part == "." || part == "..")
    {
        Err(AppError::new(ErrorCode::InvalidRemoteArgument))
    } else {
        Ok(())
    }
}
/// Not deserializable from IPC and contains no arbitrary-text constructor.
pub struct PreparedCommand {
    encoded: String,
    category: OperationCategory,
    response: ResponseKind,
    timeout_seconds: u32,
}
impl fmt::Debug for PreparedCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedCommand")
            .field("bytes", &self.encoded.len())
            .field("category", &self.category)
            .field("response", &self.response)
            .finish()
    }
}
impl PreparedCommand {
    pub fn encoded(&self) -> &str {
        &self.encoded
    }
    pub fn response(&self) -> &ResponseKind {
        &self.response
    }
    pub fn category(&self) -> &OperationCategory {
        &self.category
    }
    pub fn timeout_seconds(&self) -> u32 {
        self.timeout_seconds
    }
    pub fn structured_ssh_arguments(
        &self,
        selection: &SshSelection,
    ) -> Result<Vec<OsString>, AppError> {
        if self.category == OperationCategory::Terminal {
            return Err(AppError::new(ErrorCode::PermissionDenied));
        }
        let mut args = runner::structured_arguments(selection)?;
        // One remote command string. OpenSSH joins trailing arguments before sending to the login shell.
        args.push(OsString::from(&self.encoded));
        Ok(args)
    }
}
pub fn prepare(
    plan: CommandPlan,
    config: &DockerCommandConfig,
) -> Result<PreparedCommand, AppError> {
    if plan.args().first().map(String::as_str) != Some("docker") {
        return Err(AppError::new(ErrorCode::Internal));
    }
    let args = config.arguments(plan.args().iter().skip(1).cloned());
    Ok(PreparedCommand {
        encoded: quoting::command(&args)?,
        category: plan.category().clone(),
        response: plan.response().clone(),
        timeout_seconds: plan.timeout_seconds(),
    })
}
#[cfg(test)]
mod tests;

pub(crate) mod live_logs;
