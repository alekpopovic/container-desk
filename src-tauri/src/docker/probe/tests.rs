use super::*;
use std::{collections::VecDeque, os::unix::process::ExitStatusExt, sync::Mutex};
struct Fixture {
    replies: Mutex<VecDeque<Captured>>,
    commands: Mutex<Vec<String>>,
}
impl Executor for Fixture {
    fn execute(&self, command: String) -> ProbeFuture<'_> {
        self.commands.lock().unwrap().push(command);
        Box::pin(async {
            Ok(self
                .replies
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected replay"))
        })
    }
}
fn output(stdout: &str, stderr: &str, code: i32) -> Captured {
    Captured {
        stdout: stdout.as_bytes().to_vec(),
        stderr: stderr.as_bytes().to_vec(),
        status: std::process::ExitStatus::from_raw(code << 8),
    }
}
fn fixture(replies: Vec<Captured>) -> Fixture {
    Fixture {
        replies: Mutex::new(replies.into()),
        commands: Mutex::new(vec![]),
    }
}
fn context_json() -> &'static str {
    r#"{"name":"rootless","endpoint":"unix:///run/user/1000/docker.sock"}"#
}
fn success_replies() -> Vec<Captured> {
    vec![
        output(context_json(), "", 0),
        output(r#"{"client":"29.fixture","server":"29.fixture"}"#, "", 0),
        output(
            r#"{"id":"fixture-daemon","os":"linux","security":["name=rootless","name=seccomp,profile=builtin"]}"#,
            "",
            0,
        ),
        output("", "docker: 'compose' is not a docker command.", 1),
        output(context_json(), "", 0),
    ]
}
#[tokio::test]
async fn rootless_context_compose_absence_and_all_operation_categories_keep_the_binding() {
    use crate::policy::registry::{self, ReadOperation};
    let executor = fixture(success_replies());
    let options = DockerOptions {
        executable: Some("/usr/bin/docker".into()),
        context: Some("rootless".into()),
        sudo: true,
    };
    let mut report = DockerProbeReport::empty(true);
    let binding = probe(&executor, &options, &mut report).await.unwrap();
    assert_eq!(report.status, DockerProbeStatus::Ready);
    assert_eq!(report.rootless, Some(true));
    assert_eq!(report.compose, ComposeAvailability::Absent);
    let id = ContainerId("a".repeat(64));
    let plans = [
        registry::read(&ReadOperation::ListContainers).unwrap(),
        registry::read(&ReadOperation::InspectContainer {
            container_id: id.clone(),
        })
        .unwrap(),
        registry::read(&ReadOperation::ContainerLogs {
            container_id: id.clone(),
            tail: 10,
            timeout_seconds: 10,
        })
        .unwrap(),
        registry::read(&ReadOperation::InspectImage {
            image_id: ImageId("b".repeat(64)),
        })
        .unwrap(),
        registry::confirmation(&ConfirmationOperation::Mutation(MutationSpec {
            operation: MutationOperation::Stop,
            container_ids: vec![id.clone()],
            timeout_seconds: 10,
        }))
        .unwrap(),
        registry::confirmation(&ConfirmationOperation::Terminal(TerminalSpec {
            container_id: id,
            shell: TerminalShell::Sh,
            columns: 80,
            rows: 24,
        }))
        .unwrap(),
    ];
    for plan in plans {
        let prepared = binding.prepare(plan, &report).unwrap();
        assert!(
            prepared
                .encoded()
                .starts_with("exec 'sudo' '-n' '--' '/usr/bin/docker' '--context' 'rootless' ")
        );
    }
    for command in executor.commands.lock().unwrap().iter() {
        assert!(
            command.starts_with("exec 'sudo' '-n' '--' '/usr/bin/docker' '--context' 'rootless' ")
        );
    }
    let mut changed = report.clone();
    changed.daemon_id = Some("different".into());
    assert_eq!(
        binding
            .prepare(
                registry::read(&ReadOperation::ListContainers).unwrap(),
                &changed
            )
            .unwrap_err()
            .code,
        ErrorCode::StaleSession
    );
}
#[tokio::test]
async fn missing_docker_permission_stopped_daemon_and_sudo_errors_are_static_and_not_retried() {
    for (message, code, sudo, expected) in [
        (
            "sh: docker: not found",
            127,
            false,
            DockerProbeStatus::DockerMissing,
        ),
        (
            "permission denied while trying to connect to the Docker daemon socket",
            1,
            false,
            DockerProbeStatus::PermissionDenied,
        ),
        (
            "Cannot connect to the Docker daemon. Is the docker daemon running?",
            1,
            false,
            DockerProbeStatus::DaemonUnavailable,
        ),
        (
            "sudo: a password is required",
            1,
            true,
            DockerProbeStatus::SudoAuthenticationRequired,
        ),
        (
            "lab is not in the sudoers file",
            1,
            true,
            DockerProbeStatus::SudoDenied,
        ),
    ] {
        let executor = fixture(vec![output("", message, code)]);
        let mut report = DockerProbeReport::empty(sudo);
        assert!(
            matches!(probe(&executor, &DockerOptions{sudo,..Default::default()}, &mut report).await, Err(actual) if actual == expected)
        );
        assert_eq!(executor.commands.lock().unwrap().len(), 1);
        assert!(!serde_json::to_string(&report).unwrap().contains(message));
    }
}
#[tokio::test]
async fn default_endpoint_is_pinned_and_context_drift_or_non_linux_is_rejected() {
    let mut replies = success_replies();
    replies[0] = output(
        r#"{"name":"default","endpoint":"unix:///tmp/selected.sock"}"#,
        "",
        0,
    );
    replies[4] = output(
        r#"{"name":"default","endpoint":"unix:///tmp/other.sock"}"#,
        "",
        0,
    );
    let executor = fixture(replies);
    let mut report = DockerProbeReport::empty(false);
    assert!(matches!(
        probe(&executor, &DockerOptions::default(), &mut report).await,
        Err(DockerProbeStatus::IdentityChanged)
    ));
    for command in executor.commands.lock().unwrap().iter().skip(1) {
        assert!(command.contains("'--host' 'unix:///tmp/selected.sock'"));
    }
    let mut replies = success_replies();
    replies[2] = output(r#"{"id":"fixture","os":"windows","security":null}"#, "", 0);
    let executor = fixture(replies);
    assert!(matches!(
        probe(&executor, &DockerOptions::default(), &mut report).await,
        Err(DockerProbeStatus::UnsupportedOs)
    ));
    assert_eq!(executor.commands.lock().unwrap().len(), 3);
    assert_eq!(
        report.rootless, None,
        "missing security information is unknown"
    );
}
#[test]
fn hostile_or_oversized_responses_and_credential_endpoints_never_cross_ipc() {
    for endpoint in [
        "tcp://user:secret@host:2376",
        "ssh://user:secret@host",
        "tcp://host:2376?token=secret",
        "npipe:////pipe/docker",
        "unix:///tmp/../docker.sock",
        "ssh://host\nsecret",
    ] {
        assert!(endpoint_kind(endpoint).is_err());
    }
    assert_eq!(
        endpoint_kind("tcp://127.0.0.1:2376").unwrap(),
        DockerEndpointKind::Tcp
    );
    assert_eq!(
        endpoint_kind("ssh://lab@engine:22").unwrap(),
        DockerEndpointKind::Ssh
    );
    assert!(matches!(
        parse::<Info>(&vec![b' '; 16385]),
        Err(DockerProbeStatus::OutputLimit)
    ));
    assert!(parse::<Info>(b"not-json").is_err());
}

#[tokio::test]
#[ignore = "requires disposable SSH lab with real CLI and synthetic Docker API"]
async fn disposable_lab_docker_cli_probe_matrix() {
    let path = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").expect("explicit lab manifest");
    let lab: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut connection = Connection::new(
        "/usr/bin/ssh",
        SshSelection {
            alias: "direct-known".into(),
            config_path: lab["config"].as_str().unwrap().into(),
            use_default_config: false,
        },
    )
    .unwrap();
    assert_eq!(
        connection.start().await.unwrap().status,
        SshAccessStatus::Verified
    );
    for (binary, context, sudo, expected) in [
        (
            "/opt/fixture/docker-flood",
            None,
            false,
            DockerProbeStatus::OutputLimit,
        ),
        (
            "/opt/fixture/docker-hang",
            None,
            false,
            DockerProbeStatus::TimedOut,
        ),
        (
            "/opt/fixture/missing-docker",
            None,
            false,
            DockerProbeStatus::DockerMissing,
        ),
        (
            "/opt/fixture/docker-stopped",
            None,
            false,
            DockerProbeStatus::DaemonUnavailable,
        ),
        (
            "/opt/fixture/docker-denied",
            None,
            false,
            DockerProbeStatus::PermissionDenied,
        ),
        (
            "/opt/fixture/docker-rootless",
            None,
            false,
            DockerProbeStatus::Ready,
        ),
        (
            "/opt/fixture/docker-stopped",
            Some("rootless"),
            false,
            DockerProbeStatus::Ready,
        ),
        (
            "/opt/fixture/docker-windows",
            None,
            false,
            DockerProbeStatus::UnsupportedOs,
        ),
        (
            "/usr/bin/docker",
            None,
            true,
            DockerProbeStatus::SudoAuthenticationRequired,
        ),
        (
            "/opt/fixture/docker-rootless",
            Some("missing-context"),
            false,
            DockerProbeStatus::InvalidContext,
        ),
    ] {
        let options = DockerOptions {
            executable: Some(binary.into()),
            context: context.map(String::from),
            sudo,
        };
        let (report, binding) = run(&connection, &options).await;
        assert_eq!(
            report.status, expected,
            "case {binary}, context {context:?}, sudo {sudo}; safe report {report:?}"
        );
        assert_eq!(binding.is_some(), expected == DockerProbeStatus::Ready);
        if expected == DockerProbeStatus::Ready {
            assert_eq!(report.rootless, Some(true));
            assert_eq!(
                report.daemon_id.as_deref(),
                Some("containerdesk-api-fixture")
            );
            assert_eq!(
                report.endpoint.as_deref(),
                Some("unix:///tmp/probe-rootless.sock")
            );
            assert_eq!(report.compose, ComposeAvailability::Absent);
        }
        println!("real SSH + remote Docker CLI, synthetic API: {expected:?}");
    }
    connection.close().await;
    let sessions = crate::ssh::sessions::Sessions::default();
    let options = DockerOptions {
        executable: Some("/opt/fixture/docker-rootless".into()),
        context: Some("rootless".into()),
        sudo: false,
    };
    let gate = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let driver = std::sync::Arc::new(crate::ssh::sessions::NativeDriver {
        runner: runner::Runner::default(),
        executable: "/usr/bin/ssh".into(),
        _permit: gate.clone().try_acquire_owned().unwrap(),
        connection: Default::default(),
        docker_options: options.clone(),
        docker_binding: Default::default(),
    });
    let selected = SshSelection {
        alias: "via-known".into(),
        config_path: lab["config"].as_str().unwrap().into(),
        use_default_config: false,
    };
    let snapshot = sessions
        .begin(selected, options, || Ok(driver))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let result = sessions.snapshot(&snapshot.token).unwrap();
            assert!(
                !matches!(
                    result.state,
                    ConnectionState::Error | ConnectionState::Degraded
                ),
                "safe result: {result:?}"
            );
            if result.state == ConnectionState::Ready {
                assert_eq!(
                    result.docker.unwrap().daemon_id.as_deref(),
                    Some("containerdesk-api-fixture")
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    sessions.shutdown().await;
    assert_eq!(gate.available_permits(), 1);
    println!("native ProxyJump session reached Ready against explicitly synthetic Docker API");
}

#[tokio::test]
async fn compose_json_version_is_read_only_after_successful_plugin_discovery() {
    let mut replies = success_replies();
    replies[3] = output("Docker Compose version v2.99.0-fixture", "", 0);
    replies.insert(4, output(r#"{"version":"v2.99.0-fixture"}"#, "", 0));
    let executor = fixture(replies);
    let mut report = DockerProbeReport::empty(false);
    probe(&executor, &DockerOptions::default(), &mut report)
        .await
        .unwrap();
    assert_eq!(report.compose, ComposeAvailability::Available);
    assert_eq!(report.compose_version.as_deref(), Some("v2.99.0-fixture"));
    assert_eq!(executor.commands.lock().unwrap().len(), 6);
}

#[tokio::test]
#[ignore = "requires an explicitly provisioned real Docker Engine, direct and ProxyJump SSH lab"]
async fn checkpoint018_real_engine_version_and_bounded_empty_list_without_client_tools() {
    use crate::policy::registry::{self, ReadOperation};
    assert_eq!(std::env::var("PATH").unwrap(), "/nonexistent");
    let path = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").expect("explicit lab manifest");
    let lab: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(lab["realEngine"], true, "synthetic API does not qualify");
    let mut identity = None;
    for alias in ["direct-known", "via-known"] {
        let mut connection = Connection::new(
            "/usr/bin/ssh",
            SshSelection {
                alias: alias.into(),
                config_path: lab["config"].as_str().unwrap().into(),
                use_default_config: false,
            },
        )
        .unwrap();
        assert_eq!(
            connection.start().await.unwrap().status,
            SshAccessStatus::Verified
        );
        let (initial, binding) = run(&connection, &DockerOptions::default()).await;
        assert_eq!(initial.status, DockerProbeStatus::Ready);
        assert_eq!(
            initial.server_version.as_deref(),
            lab["engineVersion"].as_str()
        );
        assert_eq!(initial.daemon_id.as_deref(), lab["engineId"].as_str());
        assert_eq!(
            initial.endpoint.as_deref(),
            Some("unix:///var/run/docker.sock")
        );
        assert_eq!(initial.rootless, Some(false));
        assert_eq!(initial.os.as_deref(), Some("linux"));
        assert_ne!(
            initial.daemon_id.as_deref(),
            Some("containerdesk-api-fixture")
        );
        if let Some(previous) = identity {
            assert_eq!(initial.daemon_id, Some(previous));
        }
        identity = initial.daemon_id.clone();
        // Re-observe actual identity before dispatch; never substitute renderer input or old report.
        let (fresh, _) = run(&connection, &DockerOptions::default()).await;
        let command = binding
            .unwrap()
            .prepare(
                registry::read(&ReadOperation::ListContainers).unwrap(),
                &fresh,
            )
            .unwrap();
        let result = connection
            .start_fixed(
                command.encoded().to_string(),
                Limits {
                    deadline: Duration::from_secs(5),
                    stdout_bytes: 16 * 1024 * 1024,
                    stderr_bytes: 16 * 1024,
                },
            )
            .unwrap()
            .wait()
            .await
            .unwrap();
        assert!(result.status.success());
        assert!(
            result.stdout.is_empty(),
            "explicitly empty real Engine must return a successful empty JSONL list"
        );
        connection.close().await;
        println!(
            "PASS real Engine {alias}: native SSH, fresh daemon identity, version {}, empty docker ps JSONL, 5 s / 16 MiB bounds; PATH=/nonexistent",
            initial.server_version.unwrap()
        );
    }
}
