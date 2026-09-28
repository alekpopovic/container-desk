use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::{
    sync::Semaphore,
    time::{Duration, sleep},
};
struct Controlled {
    gates: [Semaphore; 3],
    calls: AtomicUsize,
    fail_probe: bool,
    closed: AtomicUsize,
}
impl Controlled {
    fn new(fail_probe: bool) -> Arc<Self> {
        Arc::new(Self {
            gates: std::array::from_fn(|_| Semaphore::new(0)),
            calls: AtomicUsize::new(0),
            fail_probe,
            closed: AtomicUsize::new(0),
        })
    }
    fn release(&self, stage: usize) {
        self.gates[stage].add_permits(1);
    }
}
impl StageDriver for Controlled {
    fn run<'a>(
        &'a self,
        stage: ConnectionStage,
        _: &'a SshSelection,
    ) -> StageFuture<'a, StageOutcome> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let index = match stage {
                ConnectionStage::Resolve => 0,
                ConnectionStage::Authenticate => 1,
                ConnectionStage::Probe => 2,
            };
            self.gates[index].acquire().await.unwrap().forget();
            match stage {
                ConnectionStage::Resolve => StageOutcome::Resolved {
                    has_jump: true,
                    effective: None,
                },
                ConnectionStage::Authenticate => {
                    StageOutcome::Authenticated(SshTransportMode::DirectFallback)
                }
                ConnectionStage::Probe if self.fail_probe => {
                    StageOutcome::Failed(ConnectionDiagnosticCode::RemoteCommandFailed)
                }
                ConnectionStage::Probe => StageOutcome::Ready,
            }
        })
    }
    fn quiesce(&self) -> StageFuture<'_, ()> {
        Box::pin(async {
            self.closed.fetch_add(1, Ordering::SeqCst);
        })
    }
}
fn selection(alias: &str) -> SshSelection {
    SshSelection {
        alias: alias.into(),
        config_path: "/fixture/config".into(),
        use_default_config: false,
    }
}
async fn wait_state(
    sessions: &Sessions,
    token: &ConnectionToken,
    state: ConnectionState,
) -> ConnectionSnapshot {
    let started = Instant::now();
    loop {
        let snapshot = sessions.snapshot(token).unwrap();
        if snapshot.state == state {
            return snapshot;
        }
        assert!(started.elapsed() < Duration::from_secs(2));
        sleep(Duration::from_millis(1)).await;
    }
}
#[tokio::test]
async fn slow_resolution_duplicate_click_and_cancel_invalidate_old_callbacks() {
    let sessions = Sessions::default();
    let driver = Controlled::new(false);
    let first = sessions
        .begin(selection("one"), Default::default(), || Ok(driver.clone()))
        .await
        .unwrap();
    let duplicate = sessions
        .begin(selection("one"), Default::default(), || {
            panic!("must not dispatch twice")
        })
        .await
        .unwrap();
    assert_eq!(first.token, duplicate.token);
    sleep(Duration::from_millis(10)).await;
    assert_eq!(driver.calls.load(Ordering::SeqCst), 1);
    let cancelled = sessions.disconnect(&first.token).await.unwrap();
    assert_eq!(cancelled.state, ConnectionState::Disconnected);
    assert!(cancelled.token.session_generation > first.token.session_generation);
    driver.release(0);
    assert_eq!(
        sessions.snapshot(&first.token).unwrap_err().code,
        ErrorCode::StaleSession
    );
    assert_eq!(
        sessions.snapshot(&cancelled.token).unwrap().state,
        ConnectionState::Disconnected
    );
    assert_eq!(driver.closed.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn switch_during_probe_ignores_old_result_and_stale_disconnect_cannot_stop_new_host() {
    let sessions = Sessions::default();
    let first = Controlled::new(false);
    let old = sessions
        .begin(selection("old"), Default::default(), || Ok(first.clone()))
        .await
        .unwrap();
    first.release(0);
    first.release(1);
    wait_state(&sessions, &old.token, ConnectionState::Probing).await;
    let next = Controlled::new(false);
    let new = sessions
        .begin(selection("new"), Default::default(), || Ok(next.clone()))
        .await
        .unwrap();
    first.release(2);
    assert_eq!(
        sessions.disconnect(&old.token).await.unwrap_err().code,
        ErrorCode::StaleSession
    );
    next.release(0);
    next.release(1);
    next.release(2);
    let ready = wait_state(&sessions, &new.token, ConnectionState::Ready).await;
    assert_eq!(ready.selection.alias, "new");
    assert_eq!(ready.durations.len(), 3);
    assert!(ready.has_jump);
    assert!(ready.diagnostic.is_none());
    let disconnected = sessions.disconnect(&ready.token).await.unwrap();
    assert_eq!(disconnected.state, ConnectionState::Disconnected);
    assert_eq!(next.closed.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn remote_probe_failure_is_distinct_and_explicit_retry_gets_new_generation() {
    let sessions = Sessions::default();
    let driver = Controlled::new(true);
    let first = sessions
        .begin(selection("one"), Default::default(), || Ok(driver.clone()))
        .await
        .unwrap();
    for stage in 0..3 {
        driver.release(stage);
    }
    let failed = wait_state(&sessions, &first.token, ConnectionState::Error).await;
    assert_eq!(
        failed.diagnostic,
        Some(ConnectionDiagnostic {
            stage: ConnectionStage::Probe,
            code: ConnectionDiagnosticCode::RemoteCommandFailed
        })
    );
    assert_eq!(driver.calls.load(Ordering::SeqCst), 3);
    let retry = Controlled::new(false);
    let second = sessions
        .begin(selection("one"), Default::default(), || Ok(retry.clone()))
        .await
        .unwrap();
    assert!(second.token.session_generation > first.token.session_generation);
    assert_ne!(second.token.session_id, first.token.session_id);
    sessions.disconnect(&second.token).await.unwrap();
}

#[tokio::test]
#[ignore = "requires explicit disposable SSH lab"]
async fn disposable_lab_session_driver_reports_real_authentication_and_no_false_docker_readiness() {
    let path =
        std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").expect("explicit lab manifest required");
    let lab: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let sessions = Sessions::default();
    for (alias, expected, code) in [
        (
            "direct-command-failed",
            ConnectionState::Error,
            ConnectionDiagnosticCode::RemoteCommandFailed,
        ),
        (
            "direct-known",
            ConnectionState::Degraded,
            ConnectionDiagnosticCode::DockerUnavailable,
        ),
        (
            "via-jump-absent",
            ConnectionState::Error,
            ConnectionDiagnosticCode::AuthenticationFailed,
        ),
    ] {
        let gate = Arc::new(Semaphore::new(1));
        let docker_options = DockerOptions {
            executable: Some("/opt/fixture/docker-stopped".into()),
            ..Default::default()
        };
        let driver = Arc::new(NativeDriver {
            runner: super::super::runner::Runner::default(),
            executable: "/usr/bin/ssh".into(),
            _permit: gate.clone().try_acquire_owned().unwrap(),
            connection: Default::default(),
            docker_options: docker_options.clone(),
            docker_binding: Default::default(),
        });
        let selected = SshSelection {
            alias: alias.into(),
            config_path: lab["config"].as_str().unwrap().into(),
            use_default_config: false,
        };
        let snapshot = sessions
            .begin(selected, docker_options, || Ok(driver))
            .await
            .unwrap();
        let result = wait_state(&sessions, &snapshot.token, expected).await;
        assert_eq!(result.diagnostic.as_ref().unwrap().code, code);
        assert_eq!(result.has_jump, alias.starts_with("via-"));
        sessions.shutdown().await;
        assert_eq!(
            sessions.snapshot(&snapshot.token).unwrap_err().code,
            ErrorCode::StaleSession
        );
        assert_eq!(
            gate.available_permits(),
            1,
            "app shutdown waits for native cleanup"
        );
        println!(
            "native session {alias}: {:?}, {:?}, {} completed stages",
            result.state,
            code,
            result.durations.len()
        );
    }
}

#[tokio::test]
async fn native_cancel_closes_a_slow_ssh_handshake_before_releasing_the_probe_gate() {
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let directory = std::path::Path::new("/tmp").join(format!(
        "containerdesk-slow-handshake-{}",
        crate::test_directory_suffix()
    ));
    std::fs::create_dir(&directory).unwrap();
    let config = directory.join("config");
    std::fs::write(
        &config,
        format!(
            "Host fixture\n HostName 127.0.0.1\n Port {}\n IdentityAgent none\n",
            listener.local_addr().unwrap().port()
        ),
    )
    .unwrap();
    let sessions = Sessions::default();
    let gate = Arc::new(Semaphore::new(1));
    let driver = Arc::new(NativeDriver {
        runner: super::super::runner::Runner::default(),
        executable: "/usr/bin/ssh".into(),
        _permit: gate.clone().try_acquire_owned().unwrap(),
        connection: Default::default(),
        docker_options: Default::default(),
        docker_binding: Default::default(),
    });
    let selected = SshSelection {
        alias: "fixture".into(),
        config_path: config.to_str().unwrap().into(),
        use_default_config: false,
    };
    let first = sessions
        .begin(selected, Default::default(), || Ok(driver))
        .await
        .unwrap();
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        sessions.snapshot(&first.token).unwrap().state,
        ConnectionState::Connecting
    );
    let mut prefix = [0; 4];
    tokio::time::timeout(Duration::from_secs(1), socket.read_exact(&mut prefix))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        &prefix, b"SSH-",
        "wait for actual client banner before cancelling"
    );
    let start = Instant::now();
    let cancelled = sessions.disconnect(&first.token).await.unwrap();
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(cancelled.state, ConnectionState::Disconnected);
    assert_eq!(gate.available_permits(), 1);
    let mut bytes = vec![];
    tokio::time::timeout(Duration::from_secs(1), socket.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    std::fs::remove_file(config).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[tokio::test]
async fn changing_docker_mode_for_the_same_alias_invalidates_the_previous_attempt() {
    let sessions = Sessions::default();
    let first = Controlled::new(false);
    let before = sessions
        .begin(selection("one"), DockerOptions::default(), || {
            Ok(first.clone())
        })
        .await
        .unwrap();
    let second = Controlled::new(false);
    let changed = DockerOptions {
        context: Some("rootless".into()),
        sudo: true,
        executable: None,
    };
    let after = sessions
        .begin(selection("one"), changed.clone(), || Ok(second))
        .await
        .unwrap();
    assert!(after.token.session_generation > before.token.session_generation);
    assert_eq!(after.docker_options, changed);
    assert_eq!(
        sessions.snapshot(&before.token).unwrap_err().code,
        ErrorCode::StaleSession
    );
    assert_eq!(first.closed.load(Ordering::SeqCst), 1);
    sessions.shutdown().await;
}
