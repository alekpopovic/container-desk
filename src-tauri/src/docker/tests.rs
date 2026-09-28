use super::*;
use crate::policy::registry::{self, ReadOperation};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
};
struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "containerdesk-remote-{}-{}",
            std::process::id(),
            crate::test_directory_suffix()
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
    fn binary(&self) -> String {
        let path = self.0.join("docker with space and 'quote");
        symlink(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/process/argv.sh"),
            &path,
        )
        .unwrap();
        path.to_string_lossy().into_owned()
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn decode(command: &PreparedCommand) -> Vec<String> {
    // Explicit inert local POSIX harness for the remote shell layer, not production SSH spawning.
    let output = std::process::Command::new("/bin/sh")
        .args(["-c", command.encoded()])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.ends_with(&[0]));
    output.stdout[..output.stdout.len() - 1]
        .split(|b| *b == 0)
        .map(|b| std::str::from_utf8(b).unwrap().into())
        .collect()
}
#[test]
fn container_image_and_template_arguments_survive_shell_encoding() {
    let lab = Lab::new();
    let config =
        DockerCommandConfig::new(Some(lab.binary()), Some("lab-context".into()), false).unwrap();
    let plan = registry::read(&ReadOperation::ListContainers).unwrap();
    let command = prepare(plan, &config).unwrap();
    assert_eq!(
        decode(&command),
        [
            "--context",
            "lab-context",
            "ps",
            "--all",
            "--no-trunc",
            "--format",
            "{{json .}}"
        ]
    );
    let id = "a".repeat(64);
    let command = prepare(
        registry::read(&ReadOperation::InspectContainer {
            container_id: ContainerId(id.clone()),
        })
        .unwrap(),
        &config,
    )
    .unwrap();
    assert_eq!(
        decode(&command),
        [
            "--context",
            "lab-context",
            "inspect",
            "--type",
            "container",
            "--",
            &id
        ]
    );
    let image = format!("sha256:{}", "b".repeat(64));
    let command = prepare(
        registry::read(&ReadOperation::InspectImage {
            image_id: ImageId(image.clone()),
        })
        .unwrap(),
        &config,
    )
    .unwrap();
    assert_eq!(
        decode(&command),
        ["--context", "lab-context", "image", "inspect", "--", &image]
    );
    let selected = SshSelection {
        alias: "alias-kept".into(),
        config_path: "/fixture/config with spaces".into(),
        use_default_config: false,
    };
    let args = command.structured_ssh_arguments(&selected).unwrap();
    assert_eq!(
        &args[args.len() - 3..],
        [
            OsString::from("--"),
            OsString::from("alias-kept"),
            OsString::from(command.encoded())
        ]
    );
    assert_eq!(command.response(), &ResponseKind::ImageDetail);
}
#[test]
fn invalid_ids_contexts_paths_and_limits_fail_before_any_shell_command() {
    for value in [
        "--all",
        "image:tag",
        "a;id",
        "short",
        &"A".repeat(64),
        &format!("sha512:{}", "a".repeat(64)),
    ] {
        assert!(ImageId(value.into()).validate().is_err());
        assert!(
            registry::read(&ReadOperation::InspectContainer {
                container_id: ContainerId(value.into())
            })
            .is_err()
        );
        assert!(
            registry::read(&ReadOperation::InspectImage {
                image_id: ImageId(value.into())
            })
            .is_err()
        );
    }
    assert!(ImageId("a".repeat(64)).validate().is_ok());
    for path in [
        "docker",
        "-option",
        "/",
        "//bin/docker",
        "/usr/../bin/docker",
        "/usr/./docker",
        "/tmp/docker\n",
        "/tmp/docker\0",
    ] {
        assert!(DockerCommandConfig::new(Some(path.into()), None, false).is_err());
    }
    for context in ["", "--host", "a b", "a;id", "$(id)", "a\nb"] {
        assert!(DockerCommandConfig::new(None, Some(context.into()), false).is_err());
    }
    assert!(
        registry::read(&ReadOperation::ContainerLogs {
            container_id: ContainerId("a".repeat(64)),
            tail: -1,
            timeout_seconds: 30
        })
        .is_err()
    );
    assert!(
        DockerCommandConfig::new(
            Some("/opt/docker with 'apostrophe'/$literal`name`".into()),
            None,
            false
        )
        .is_ok()
    );
}
#[test]
fn optional_sudo_is_fixed_noninteractive_and_terminal_cannot_use_snapshot_argv() {
    let config = DockerCommandConfig::new(
        Some("/usr/local/bin/docker".into()),
        Some("rootless".into()),
        true,
    )
    .unwrap();
    let command = prepare(
        registry::read(&ReadOperation::ListContainers).unwrap(),
        &config,
    )
    .unwrap();
    assert!(
        command
            .encoded()
            .starts_with("exec 'sudo' '-n' '--' '/usr/local/bin/docker' '--context' 'rootless' ")
    );
    let operation = ConfirmationOperation::Terminal(TerminalSpec {
        container_id: ContainerId("c".repeat(64)),
        shell: TerminalShell::Sh,
        columns: 80,
        rows: 24,
    });
    let terminal = prepare(registry::confirmation(&operation).unwrap(), &config).unwrap();
    let selected = SshSelection {
        alias: "fixture".into(),
        config_path: "/dev/null".into(),
        use_default_config: false,
    };
    assert_eq!(
        terminal
            .structured_ssh_arguments(&selected)
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    );
    let operation = ConfirmationOperation::Mutation(MutationSpec {
        operation: MutationOperation::Stop,
        container_ids: vec![ContainerId("a".repeat(64))],
        timeout_seconds: 120,
    });
    let prepared = prepare(
        registry::confirmation(&operation).unwrap(),
        &DockerCommandConfig::default(),
    )
    .unwrap();
    assert_eq!(prepared.timeout_seconds(), 130);
    assert_eq!(prepared.category(), &OperationCategory::Mutation);
    assert!(prepared.encoded().contains("'stop' '-t' '120' '--'"));
    assert!(!format!("{prepared:?}").contains("stop"));
}
