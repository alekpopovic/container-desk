use crate::domain::*;
use std::{
    ffi::CString,
    fs, io,
    os::unix::{
        ffi::OsStrExt,
        fs::{FileTypeExt, MetadataExt},
    },
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command, time::timeout};

pub const DEFAULT_SSH: &str = "/usr/bin/ssh";
const OUTPUT_LIMIT: usize = 4096;
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

fn report(path: &str, status: SshStatus, version: Option<String>) -> SshDiagnostic {
    let message = match status {
        SshStatus::Ready => "OpenSSH is available.",
        SshStatus::Missing => {
            "SSH was not found. Install OpenSSH or choose its absolute executable path."
        }
        SshStatus::NotExecutable => "The selected SSH path is not an executable regular file.",
        SshStatus::Untrusted => {
            "Choose an absolute SSH path owned by you or root, without group/world-writable files or directories."
        }
        SshStatus::Failed => {
            "The SSH version check failed. Check the selected executable in your terminal."
        }
        SshStatus::TimedOut => "The SSH version check exceeded three seconds and was stopped.",
        SshStatus::OutputLimit => {
            "The SSH version check exceeded its output limit and was stopped."
        }
        SshStatus::InvalidVersion => {
            "The selected executable did not return a recognized OpenSSH version."
        }
    };
    SshDiagnostic {
        path: path
            .chars()
            .take(4096)
            .filter(|c| !c.is_control())
            .collect(),
        status,
        version,
        message: message.into(),
    }
}

pub fn validate_executable(path: &str) -> Result<PathBuf, SshStatus> {
    if !Path::new(path).is_absolute() || path.len() > 4096 || path.chars().any(char::is_control) {
        return Err(SshStatus::Untrusted);
    }
    let canonical = fs::canonicalize(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            SshStatus::Missing
        } else {
            SshStatus::Untrusted
        }
    })?;
    let metadata = fs::metadata(&canonical).map_err(|_| SshStatus::Missing)?;
    if !metadata.is_file() || metadata.mode() & 0o111 == 0 {
        return Err(SshStatus::NotExecutable);
    }
    let cpath = CString::new(canonical.as_os_str().as_bytes()).map_err(|_| SshStatus::Untrusted)?;
    // SAFETY: CString remains valid and null-terminated throughout access; geteuid has no arguments.
    if unsafe { libc::access(cpath.as_ptr(), libc::X_OK) } != 0 {
        return Err(SshStatus::NotExecutable);
    }
    let uid = unsafe { libc::geteuid() };
    if (metadata.uid() != 0 && metadata.uid() != uid) || metadata.mode() & 0o022 != 0 {
        return Err(SshStatus::Untrusted);
    }
    for ancestor in canonical.parent().into_iter().flat_map(Path::ancestors) {
        let parent = fs::metadata(ancestor).map_err(|_| SshStatus::Untrusted)?;
        let root_sticky = parent.uid() == 0 && parent.mode() & 0o1000 != 0;
        if (parent.uid() != 0 && parent.uid() != uid)
            || (parent.mode() & 0o022 != 0 && !root_sticky)
        {
            return Err(SshStatus::Untrusted);
        }
    }
    Ok(canonical)
}

async fn read_bounded(reader: impl tokio::io::AsyncRead + Unpin) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    reader
        .take(OUTPUT_LIMIT as u64 + 1)
        .read_to_end(&mut output)
        .await?;
    if output.len() > OUTPUT_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            "version output limit",
        ));
    }
    Ok(output)
}

pub async fn inspect_ssh(path: &str) -> SshDiagnostic {
    let canonical = match validate_executable(path) {
        Ok(path) => path,
        Err(status) => return report(path, status, None),
    };
    let mut child = match Command::new(canonical)
        .args(["-V"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return report(path, SshStatus::Failed, None),
    };
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let result = timeout(PROBE_TIMEOUT, async {
        tokio::try_join!(read_bounded(stdout), read_bounded(stderr), child.wait())
    })
    .await;
    let failure = match result {
        Ok(Ok((stdout, stderr, status))) if status.success() => {
            let version = [stderr, stdout].into_iter().find_map(|bytes| {
                let text = std::str::from_utf8(&bytes).ok()?.trim();
                if text.len() <= 512
                    && text.starts_with("OpenSSH_")
                    && !text.chars().any(char::is_control)
                {
                    Some(text.to_string())
                } else {
                    None
                }
            });
            return report(
                path,
                if version.is_some() {
                    SshStatus::Ready
                } else {
                    SshStatus::InvalidVersion
                },
                version,
            );
        }
        Ok(Ok(_)) => SshStatus::Failed,
        Ok(Err(error)) if error.kind() == io::ErrorKind::FileTooLarge => SshStatus::OutputLimit,
        Ok(Err(_)) => SshStatus::Failed,
        Err(_) => SshStatus::TimedOut,
    };
    // No retry; always reap the directly spawned version probe on timeout/error.
    let _ = child.kill().await;
    let _ = child.wait().await;
    report(path, failure, None)
}

pub async fn inspect_agent(path: Option<&Path>) -> AgentDiagnostic {
    let status = match path {
        None => AgentStatus::Unset,
        Some(path) if path.as_os_str().is_empty() => AgentStatus::Unset,
        Some(path) => match fs::metadata(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => AgentStatus::Missing,
            Err(_) => AgentStatus::Inaccessible,
            Ok(m) if !m.file_type().is_socket() => AgentStatus::NotSocket,
            Ok(_) => match timeout(
                Duration::from_millis(500),
                tokio::net::UnixStream::connect(path),
            )
            .await
            {
                Ok(Ok(_stream)) => AgentStatus::Reachable,
                _ => AgentStatus::Inaccessible,
            },
        },
    };
    let message = match status {
        AgentStatus::Unset => "SSH_AUTH_SOCK is not set. Existing configured keys may still work.",
        AgentStatus::Missing => {
            "The SSH agent socket is missing. Start or reconnect your agent in your terminal."
        }
        AgentStatus::NotSocket => "SSH_AUTH_SOCK does not point to a Unix socket.",
        AgentStatus::Inaccessible => {
            "The SSH agent socket could not be reached. Check your terminal's agent setup."
        }
        AgentStatus::Reachable => "The SSH agent socket is reachable. Identities were not queried.",
    };
    AgentDiagnostic {
        status,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::{fs::PermissionsExt, net::UnixListener};
    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "containerdesk-diagnostics-{}-{}",
                std::process::id(),
                crate::test_directory_suffix()
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
        fn executable(&self, contents: &str) -> String {
            let path = self.0.join("ssh-fixture");
            fs::write(&path, contents).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            path.to_str().unwrap().into()
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[tokio::test]
    async fn absolute_native_ssh_ignores_minimal_path() {
        // Run this test process with PATH=/nonexistent (see evidence); production resolves no PATH.
        let ssh = inspect_ssh(DEFAULT_SSH).await;
        assert_eq!(ssh.status, SshStatus::Ready, "{}", ssh.message);
        assert!(ssh.version.unwrap().starts_with("OpenSSH_"));
    }
    #[tokio::test]
    async fn rejects_missing_non_executable_and_unsafe_overrides() {
        let dir = Dir::new();
        assert_eq!(
            inspect_ssh(dir.0.join("absent").to_str().unwrap())
                .await
                .status,
            SshStatus::Missing
        );
        let path = dir.executable("not executed");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(inspect_ssh(&path).await.status, SshStatus::NotExecutable);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o777)).unwrap();
        assert_eq!(inspect_ssh(&path).await.status, SshStatus::Untrusted);
        assert_eq!(inspect_ssh("ssh").await.status, SshStatus::Untrusted);
        assert_eq!(
            inspect_ssh("/usr/bin/ssh\n").await.status,
            SshStatus::Untrusted
        );
    }
    #[tokio::test]
    async fn probes_are_bounded_and_unrecognized_output_is_not_exposed() {
        let dir = Dir::new();
        let path = dir.executable("#!/bin/sh\nprintf 'SECRET_SENTINEL' >&2\n");
        let result = inspect_ssh(&path).await;
        assert_eq!(result.status, SshStatus::InvalidVersion);
        assert!(
            !serde_json::to_string(&result)
                .unwrap()
                .contains("SECRET_SENTINEL")
        );
        let path = dir.executable("#!/bin/sh\nwhile :; do printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'; done\n");
        assert_eq!(inspect_ssh(&path).await.status, SshStatus::OutputLimit);
        let path = dir.executable("#!/bin/sh\nexec /bin/sleep 10\n");
        let started = std::time::Instant::now();
        assert_eq!(inspect_ssh(&path).await.status, SshStatus::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
    #[tokio::test]
    async fn agent_probe_checks_socket_without_sending_protocol_or_reading_keys() {
        let dir = Dir::new();
        assert_eq!(inspect_agent(None).await.status, AgentStatus::Unset);
        assert_eq!(
            inspect_agent(Some(&dir.0.join("missing"))).await.status,
            AgentStatus::Missing
        );
        assert_eq!(
            inspect_agent(Some(&dir.0)).await.status,
            AgentStatus::NotSocket
        );
        let path = dir.0.join("agent.sock");
        let listener = UnixListener::bind(&path).unwrap();
        assert_eq!(
            inspect_agent(Some(&path)).await.status,
            AgentStatus::Reachable
        );
        let (mut socket, _) = listener.accept().unwrap();
        let mut data = [0u8; 1];
        assert_eq!(std::io::Read::read(&mut socket, &mut data).unwrap(), 0);
        drop(listener);
        // Another test may briefly fork with this listener inherited until exec closes it.
        // The socket is truthfully reachable during that window; assert eventual closure.
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while inspect_agent(Some(&path)).await.status != AgentStatus::Inaccessible {
            assert!(
                std::time::Instant::now() < deadline,
                "closed fixture listener stayed reachable"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
