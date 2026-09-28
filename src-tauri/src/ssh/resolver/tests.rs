use super::*;
use std::{fs, io, os::unix::fs::PermissionsExt};
struct Lab(std::path::PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "containerdesk-resolver-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
    fn selection(&self) -> SshSelection {
        SshSelection {
            alias: "fixture-target".into(),
            config_path: self
                .0
                .join("config with spaces")
                .to_string_lossy()
                .into_owned(),
            use_default_config: false,
        }
    }
    fn executable(&self, text: &str) -> String {
        let path = self.0.join("ssh-fixture");
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path.to_string_lossy().into_owned()
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[tokio::test]
async fn native_g_agrees_on_overlap_include_jump_and_original_alias() {
    let lab = Lab::new();
    let selection = lab.selection();
    fs::write(
        lab.0.join("included"),
        "Host fixture-*\n User fixture-user\n ProxyJump jump-one,jump-two\nHost *\n",
    )
    .unwrap();
    let config = format!(
        "Host fixture-target\n HostName 192.0.2.10\n Port 2222\nInclude \"{}\"\nHost *\n HostName 192.0.2.99\n User ignored-user\n Port 22\nIdentityFile /fixture/SECRET_IDENTITY_REFERENCE\n",
        lab.0.join("included").display()
    );
    fs::write(&selection.config_path, &config).unwrap();
    let result = resolve(&Runner::default(), "/usr/bin/ssh", selection.clone())
        .await
        .unwrap();
    let native = std::process::Command::new("/usr/bin/ssh")
        .args(["-G", "-F", &selection.config_path, "--", &selection.alias])
        .output()
        .unwrap();
    assert!(native.status.success());
    let native = String::from_utf8(native.stdout).unwrap();
    for line in [
        "hostname 192.0.2.10",
        "user fixture-user",
        "port 2222",
        "proxyjump jump-one,jump-two",
    ] {
        assert!(native.lines().any(|v| v == line));
    }
    assert_eq!(result.hostname, "192.0.2.10");
    assert_eq!(result.user, "fixture-user");
    assert_eq!(result.port, 2222);
    assert_eq!(result.proxy_jump.as_deref(), Some("jump-one,jump-two"));
    assert_eq!(result.selection.alias, "fixture-target");
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("SECRET_IDENTITY_REFERENCE")
    );
    assert_eq!(fs::read_to_string(&selection.config_path).unwrap(), config);
    let mut defaults = selection;
    defaults.use_default_config = true;
    assert_eq!(
        arguments(&defaults).unwrap(),
        vec![
            OsString::from("-G"),
            OsString::from("--"),
            OsString::from("fixture-target")
        ]
    );
}
#[tokio::test]
async fn native_match_exec_is_acknowledged_but_proxy_command_contents_are_private() {
    let lab = Lab::new();
    let selection = lab.selection();
    let marker = lab.0.join("match-ran");
    fs::write(&selection.config_path, format!("Match exec \"touch {}\"\n User match-user\nHost *\n HostName 192.0.2.20\n ProxyCommand echo SECRET_COMMAND_VALUE\n", marker.display())).unwrap();
    let result = resolve(&Runner::default(), "/usr/bin/ssh", selection)
        .await
        .unwrap();
    assert!(marker.exists());
    assert_eq!(result.user, "match-user");
    assert!(result.has_proxy_command);
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("SECRET_COMMAND_VALUE")
    );
}
#[tokio::test]
async fn aliases_are_separate_argv_and_invalid_alias_never_starts_a_child() {
    let lab = Lab::new();
    let mut selection = lab.selection();
    let executable = lab.executable("#!/bin/sh\nprintf '%s\\n' \"$@\" > \"${0%/*}/args\"\nprintf 'hostname 192.0.2.1\\nuser fixture\\nport 22\\n'\n");
    for alias in ["-F", "a b", "a;touch", "a\nb", "$(id)"] {
        selection.alias = alias.into();
        assert_eq!(
            resolve(&Runner::default(), &executable, selection.clone())
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidAlias
        );
        assert!(!lab.0.join("args").exists());
    }
    selection.alias = "safe-fixture".into();
    resolve(&Runner::default(), &executable, selection.clone())
        .await
        .unwrap();
    assert_eq!(
        fs::read_to_string(lab.0.join("args")).unwrap(),
        format!("-G\n-F\n{}\n--\nsafe-fixture\n", selection.config_path)
    );
}
#[tokio::test]
async fn capture_is_bounded_and_timeout_reaps_the_direct_child() {
    let lab = Lab::new();
    let selection = lab.selection();
    let path = lab.executable("#!/bin/sh\necho $$ > \"${0%/*}/pid\"\nexec /bin/sleep 30\n");
    let start = std::time::Instant::now();
    assert_eq!(
        resolve_with_deadline(
            &Runner::default(),
            &path,
            selection.clone(),
            Duration::from_millis(100)
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::OperationTimedOut
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    let pid = fs::read_to_string(lab.0.join("pid"))
        .unwrap()
        .trim()
        .parse::<libc::pid_t>()
        .unwrap();
    // SAFETY: signal 0 only checks whether the recorded disposable child remains.
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    let path = lab.executable("#!/bin/sh\nwhile :; do printf 'SECRET_OUTPUT_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX' >&2; done\n");
    let failure = resolve(&Runner::default(), &path, selection.clone())
        .await
        .unwrap_err();
    assert_eq!(failure.code, ErrorCode::ResourceLimit);
    assert!(!failure.message.contains("SECRET_OUTPUT"));
    let path = lab.executable("#!/bin/sh\nprintf 'SECRET_STDERR' >&2\nexit 7\n");
    assert_eq!(
        resolve(&Runner::default(), &path, selection)
            .await
            .unwrap_err()
            .code,
        ErrorCode::SshResolutionFailed
    );
}

#[test]
fn malformed_or_ambiguous_effective_fields_never_produce_success() {
    let lab = Lab::new();
    for bytes in [
        b"".as_slice(),
        b"hostname host\nuser user\nport 0\n",
        b"hostname host\nuser user\nport 22\nport 23\n",
        b"hostname host\nport 22\n",
        b"hostname host\nuser user\nport 65536\n",
    ] {
        assert_eq!(
            parse(bytes, lab.selection(), Path::new("/usr/bin/ssh"))
                .unwrap_err()
                .code,
            ErrorCode::InvalidResponse
        );
    }
}
