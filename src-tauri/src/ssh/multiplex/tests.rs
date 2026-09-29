use super::*;
#[tokio::test]
#[ignore = "requires disposable SSH lab"]
async fn disposable_lab_multiplex_reuses_connection_and_cancels_channels_independently() {
    let path = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").expect("explicit lab manifest");
    let lab: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    for alias in ["direct-known", "via-known"] {
        let selected = SshSelection {
            alias: alias.into(),
            config_path: lab["config"].as_str().unwrap().into(),
            use_default_config: false,
        };
        let mut connection = Connection::new("/usr/bin/ssh", selected).unwrap();
        let report = connection.start().await.unwrap();
        assert_eq!(report.status, SshAccessStatus::Verified, "{report:?}");
        assert_eq!(connection.mode(), SshTransportMode::Multiplexed);
        let command =
            super::super::quoting::command(&["printenv".into(), "SSH_CONNECTION".into()]).unwrap();
        let first = connection
            .start_fixed(command.clone(), Limits::default())
            .unwrap()
            .wait()
            .await
            .unwrap();
        let second = connection
            .start_fixed(command.clone(), Limits::default())
            .unwrap()
            .wait()
            .await
            .unwrap();
        assert!(first.status.success() && second.status.success());
        assert!(!first.stdout.is_empty());
        assert_eq!(
            first.stdout, second.stdout,
            "same server-observed TCP connection"
        );
        let mut slow = connection
            .start_fixed(
                super::super::quoting::command(&["sleep".into(), "30".into()]).unwrap(),
                Limits::default(),
            )
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        slow.cancel();
        assert_eq!(slow.wait().await.unwrap_err(), RunError::Cancelled);
        let third = connection
            .start_fixed(command, Limits::default())
            .unwrap()
            .wait()
            .await
            .unwrap();
        assert!(third.status.success());
        assert_eq!(
            first.stdout, third.stdout,
            "one child cancellation cannot kill the shared master"
        );
        let tty = connection
            .start_channel(
                super::super::quoting::command(&["printenv".into(), "SSH_CONNECTION".into()])
                    .unwrap(),
                Limits::default(),
                true,
            )
            .unwrap()
            .wait()
            .await
            .unwrap();
        assert!(tty.status.success());
        assert_eq!(
            String::from_utf8(tty.stdout).unwrap().trim(),
            String::from_utf8(first.stdout.clone()).unwrap().trim()
        );
        let remaining_a = connection
            .start_fixed(
                super::super::quoting::command(&["sleep".into(), "30".into()]).unwrap(),
                Limits::default(),
            )
            .unwrap();
        let remaining_b = connection
            .start_fixed(
                super::super::quoting::command(&["sleep".into(), "30".into()]).unwrap(),
                Limits::default(),
            )
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let socket = connection.policy.runtime.socket_path();
        connection.close().await;
        assert_eq!(remaining_a.wait().await.unwrap_err(), RunError::Cancelled);
        assert_eq!(remaining_b.wait().await.unwrap_err(), RunError::Cancelled);
        assert!(!socket.exists());
        println!(
            "native multiplex: three reads reused one TCP connection; cancelled channel and closed owned master"
        );
    }
}

struct UnrelatedMaster(std::path::PathBuf);
impl UnrelatedMaster {
    async fn check(&self) -> bool {
        let result = Runner::default()
            .start(
                "/usr/bin/ssh",
                vec![
                    "-F".into(),
                    "/dev/null".into(),
                    "-S".into(),
                    self.0.clone().into_os_string(),
                    "-O".into(),
                    "check".into(),
                    "--".into(),
                    "fixture".into(),
                ],
                Limits {
                    deadline: Duration::from_secs(3),
                    ..Limits::default()
                },
            )
            .unwrap()
            .wait()
            .await
            .unwrap();
        result.status.success()
    }
}
impl Drop for UnrelatedMaster {
    fn drop(&mut self) {
        // Test-only unrelated master: bounded cleanup even after an assertion failure.
        use std::process::{Command, Stdio};
        if let Ok(mut child) = Command::new("/usr/bin/ssh")
            .args(["-F", "/dev/null", "-S"])
            .arg(&self.0)
            .args(["-O", "exit", "--", "fixture"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            let started = std::time::Instant::now();
            while child.try_wait().ok().flatten().is_none() {
                if started.elapsed() > Duration::from_secs(2) {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
}
#[tokio::test]
#[ignore = "requires disposable SSH lab"]
async fn disposable_lab_ownership_long_paths_stale_socket_fallback_and_master_death() {
    use std::os::unix::{fs::MetadataExt, net::UnixListener};
    let manifest = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").expect("explicit lab manifest");
    let lab: serde_json::Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    let lab_root = std::path::Path::new(&manifest).parent().unwrap();
    let unrelated = UnrelatedMaster(lab_root.join("unrelated-master.sock"));
    let arguments = vec![
        "-f".into(),
        "-N".into(),
        "-M".into(),
        "-o".into(),
        "ControlPersist=60".into(),
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "StrictHostKeyChecking=yes".into(),
        "-o".into(),
        "ForwardAgent=no".into(),
        "-F".into(),
        lab["config"].as_str().unwrap().into(),
        "-S".into(),
        unrelated.0.clone().into_os_string(),
        "--".into(),
        "direct-known".into(),
    ];
    let started = Runner::default()
        .start("/usr/bin/ssh", arguments, Limits::default())
        .unwrap()
        .wait()
        .await
        .unwrap();
    assert!(started.status.success());
    assert!(unrelated.check().await);
    let long = lab_root.join("a".repeat(100)).join("b".repeat(100));
    fs::create_dir_all(&long).unwrap();
    let long_config = long.join("config");
    fs::write(
        &long_config,
        format!(
            "Host *\n ControlMaster auto\n ControlPath {}\n{}",
            unrelated.0.display(),
            fs::read_to_string(lab["config"].as_str().unwrap()).unwrap()
        ),
    )
    .unwrap();
    assert!(long_config.as_os_str().len() > 240);
    let selected = SshSelection {
        alias: "direct-known".into(),
        config_path: long_config.to_str().unwrap().into(),
        use_default_config: false,
    };
    let mut connection = Connection::new("/usr/bin/ssh", selected.clone()).unwrap();
    assert!(connection.policy.runtime.socket_path().as_os_str().len() <= 80);
    assert_eq!(
        connection.start().await.unwrap().status,
        SshAccessStatus::Verified
    );
    assert_eq!(
        fs::metadata(connection.policy.runtime.path())
            .unwrap()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(connection.policy.runtime.socket_path())
            .unwrap()
            .mode()
            & 0o777,
        0o600
    );
    let read =
        super::super::quoting::command(&["printenv".into(), "SSH_CONNECTION".into()]).unwrap();
    let app_read = connection
        .start_fixed(read.clone(), Limits::default())
        .unwrap()
        .wait()
        .await
        .unwrap();
    let user_read = Runner::default()
        .start(
            "/usr/bin/ssh",
            vec![
                "-F".into(),
                "/dev/null".into(),
                "-S".into(),
                unrelated.0.clone().into_os_string(),
                "-o".into(),
                "ProxyCommand=/usr/bin/false".into(),
                "-T".into(),
                "--".into(),
                "fixture".into(),
                read.into(),
            ],
            Limits::default(),
        )
        .unwrap()
        .wait()
        .await
        .unwrap();
    assert!(app_read.status.success() && user_read.status.success());
    assert_ne!(
        app_read.stdout, user_read.stdout,
        "app must not borrow a source-configured user control master"
    );
    // Unexpected death, observed before any next application command: no native reconnect fallback.
    assert!(
        control_command(
            &connection.control,
            &connection.executable,
            &connection.policy,
            "exit"
        )
        .await
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!connection.healthy().await);
    let marker =
        super::super::quoting::command(&["printf".into(), "must-not-reconnect".into()]).unwrap();
    match connection.start_fixed(marker, Limits::default()) {
        Err(error) => assert_eq!(error.code, ErrorCode::Disconnected),
        Ok(job) => assert!(!job.wait().await.unwrap().status.success()),
    }
    connection.close().await;
    assert!(
        unrelated.check().await,
        "app cleanup must retain the unrelated user master"
    );

    // Same coordinator shutdown invoked by the native app Exit callback.
    let sessions = super::super::sessions::Sessions::default();
    let gate = Arc::new(tokio::sync::Semaphore::new(1));
    let docker_options = DockerOptions {
        executable: Some("/opt/fixture/docker-stopped".into()),
        ..Default::default()
    };
    let driver = Arc::new(super::super::sessions::NativeDriver {
        runner: Runner::default(),
        executable: "/usr/bin/ssh".into(),
        _permit: gate.clone().try_acquire_owned().unwrap(),
        connection: Default::default(),
        docker_options: docker_options.clone(),
        docker_binding: Default::default(),
    });
    let snapshot = sessions
        .begin(selected.clone(), docker_options, || Ok(driver))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let current = sessions.snapshot(&snapshot.token).unwrap();
            assert_ne!(current.state, ConnectionState::Error);
            if current.state == ConnectionState::Degraded {
                assert_eq!(current.transport_mode, SshTransportMode::Multiplexed);
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    sessions.shutdown().await;
    assert_eq!(gate.available_permits(), 1);
    assert!(
        unrelated.check().await,
        "app session shutdown leaves unrelated master running"
    );

    let mut fallback = Connection::new("/usr/bin/ssh", selected).unwrap();
    let policy = fallback.policy.clone();
    let stale = policy.runtime.socket_path();
    let listener = UnixListener::bind(&stale).unwrap();
    drop(listener); // Stale socket at an unexpected location; never adopt/remove it.
    assert_eq!(
        fallback.start().await.unwrap().status,
        SshAccessStatus::Verified
    );
    assert_eq!(fallback.mode(), SshTransportMode::DirectFallback);
    let command =
        super::super::quoting::command(&["printenv".into(), "SSH_CONNECTION".into()]).unwrap();
    let one = fallback
        .start_fixed(command.clone(), Limits::default())
        .unwrap()
        .wait()
        .await
        .unwrap();
    let two = fallback
        .start_fixed(command, Limits::default())
        .unwrap()
        .wait()
        .await
        .unwrap();
    assert!(one.status.success() && two.status.success());
    assert_ne!(
        one.stdout, two.stdout,
        "explicit direct fallback uses separate TCP connections"
    );
    fallback.close().await;
    assert!(stale.exists(), "unknown stale entry was preserved");
    assert!(unrelated.check().await);
    fs::remove_file(stale).unwrap(); // Only this test created the stale socket.
    drop(policy);
    println!(
        "native ownership: long config, master death without replay, visible direct fallback, unrelated master retained"
    );
}

#[tokio::test]
#[ignore = "requires disposable SSH lab"]
async fn disposable_lab_native_idle_persistence_expires_without_reconnect() {
    let path = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").expect("explicit lab manifest");
    let lab: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let selected = SshSelection {
        alias: "direct-known".into(),
        config_path: lab["config"].as_str().unwrap().into(),
        use_default_config: false,
    };
    let mut connection = Connection::new("/usr/bin/ssh", selected.clone()).unwrap();
    // Same builder, shorter test-only idle period; production is a fixed 60 seconds.
    assert_eq!(
        connection.start_with_persistence(2).await.unwrap().status,
        SshAccessStatus::Verified
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(
        !connection.healthy().await,
        "native idle timer expires without control polling"
    );
    tokio::time::timeout(Duration::from_secs(6), connection.wait_lost())
        .await
        .unwrap();
    assert!(!connection.healthy().await);
    connection.close().await;
    let mut monitored = Connection::new("/usr/bin/ssh", selected).unwrap();
    assert_eq!(
        monitored.start_with_persistence(2).await.unwrap().status,
        SshAccessStatus::Verified
    );
    tokio::time::timeout(Duration::from_secs(6), monitored.wait_lost())
        .await
        .unwrap();
    assert!(
        !monitored.healthy().await,
        "control health polling must not defeat the app idle deadline"
    );
    monitored.close().await;
    println!("native ControlPersist and monitored app idle deadline expired; no reconnect loop");
}

#[tokio::test]
#[ignore = "requires explicitly owned 041 SSH lab; ends by interrupting its server"]
async fn checkpoint041_owned_fallback_health_detects_loss_without_touching_foreign_socket() {
    let manifest = std::env::var("CONTAINERDESK_LOG_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest).parent().unwrap();
    let mut connection = Connection::new(
        "/usr/bin/ssh",
        SshSelection {
            alias: "logs-owned".into(),
            config_path: lab["config"].as_str().unwrap().into(),
            use_default_config: false,
        },
    )
    .unwrap();
    let runtime = connection.policy.runtime.clone();
    let socket = runtime.socket_path();
    fs::write(&socket, b"owned-test-foreign-socket-marker").unwrap();
    assert_eq!(
        connection.start().await.unwrap().status,
        SshAccessStatus::Verified
    );
    assert_eq!(connection.mode(), SshTransportMode::DirectFallback);
    assert!(connection.client().healthy().await);
    fs::write(root.join("cut-network"), b"owned server only").unwrap();
    let started = std::time::Instant::now();
    loop {
        if !connection.client().healthy().await {
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(8));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    connection.close().await;
    assert_eq!(
        fs::read(&socket).unwrap(),
        b"owned-test-foreign-socket-marker"
    );
    fs::remove_file(socket).unwrap();
    drop(runtime);
    println!(
        "PASS native direct fallback: exact bounded marker health check succeeds then detects owned server loss; foreign socket-path contents unchanged, strict trust retained."
    );
}
