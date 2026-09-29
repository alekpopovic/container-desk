//! Opt-in real VM lab. No fixture API, local Docker, or renderer mocks.
use super::*;

#[tokio::test]
#[ignore = "requires tests/lab/integration.py and its disposable VM manifest"]
async fn checkpoint049_disposable_integration() {
    let manifest = std::env::var("CONTAINERDESK_INTEGRATION_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest).parent().unwrap();
    let config = lab["config"].as_str().unwrap();
    let target = ContainerId(lab["liveId"].as_str().unwrap().into());
    for alias in ["direct-owned", "private-owned"] {
        let backend = Backend::new(&root.join(alias), "/unused-lab-home".into());
        let mut host = draft(alias, config);
        host.docker.executable = Some("/usr/bin/docker".into());
        let saved = backend
            .save_host(SaveHostRequest {
                mode: WorkspaceMode::Live,
                expected_revision: 0,
                id: None,
                draft: host,
            })
            .await
            .unwrap();
        let host_id = saved.saved.preferences.hosts[0].id.clone();
        let connect = InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: host_id.clone(),
        };
        backend
            .connect_inventory_host(connect.clone())
            .await
            .unwrap();
        let connected = ready(&backend, WorkspaceMode::Live, host_id.clone()).await;
        assert_eq!(connected.has_jump, alias == "private-owned");
        let scope = backend
            .connect_host(ConnectHostRequest {
                selection: HostSelection {
                    host_id: host_id.clone(),
                    selection_generation: 1,
                },
            })
            .unwrap()
            .scope;
        assert!(!backend.management_state(scope.clone()).unwrap().enabled);
        let listed = backend
            .list_containers(ListContainersRequest {
                scope: scope.clone(),
            })
            .await
            .unwrap();
        let mut actual: Vec<_> = listed
            .containers
            .iter()
            .map(|row| row.id.0.clone())
            .collect();
        let mut expected: Vec<String> = serde_json::from_value(lab["ownedIds"].clone()).unwrap();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
        let inspect = InspectContainerRequest {
            scope: scope.clone(),
            container_id: target.clone(),
            reveal_sensitive: false,
        };
        let detail = backend.inspect_container(inspect.clone()).await.unwrap();
        assert_eq!(detail.summary.state, "running");
        assert!(detail.environment_values_masked);
        assert!(
            !serde_json::to_string(&detail)
                .unwrap()
                .contains("SEEDED_049_ENV")
        );
        let logs = backend
            .container_logs(ContainerLogsRequest {
                scope: scope.clone(),
                container_id: target.clone(),
                tail: 10,
                timeout_seconds: 30,
                since: None,
                until: None,
            })
            .await
            .unwrap();
        assert!(logs.records.iter().any(|r| r.text == "CD049_LOG"));
        let stats = backend
            .container_stats(ContainerStatsRequest {
                scope: scope.clone(),
                container_id: target.clone(),
            })
            .await
            .unwrap();
        assert_eq!(stats.availability, StatsAvailability::Available);
        assert!(
            stats
                .values
                .memory_limit_bytes
                .is_some_and(|value| value > 0.0)
        );
        let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
        let stream = backend
            .follow_container_logs(
                FollowLogsRequest {
                    scope: scope.clone(),
                    container_id: target.clone(),
                    tail: 10,
                    since: None,
                },
                Arc::new(move |batch| sender.try_send(batch).map_err(|_| ())),
            )
            .await
            .unwrap();
        backend
            .ack_container_logs(AckLogsRequest {
                scope: scope.clone(),
                subscription_id: stream.subscription_id.clone(),
                sequence: 0,
            })
            .unwrap();
        let batch = tokio::time::timeout(Duration::from_secs(30), receiver.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(batch.records.iter().any(|r| r.text == "CD049_LOG"));
        tokio::time::timeout(
            Duration::from_secs(3),
            backend.cancel_subscription(CancelSubscriptionRequest {
                scope: scope.clone(),
                subscription_id: stream.subscription_id,
            }),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(backend.read_slots.available_permits(), 4);
        let spec = MutationSpec {
            operation: MutationOperation::Stop,
            container_ids: vec![target.clone()],
            timeout_seconds: 1,
        };
        assert_eq!(
            backend
                .prepare_confirmation(PrepareConfirmationRequest {
                    scope: scope.clone(),
                    operation: ConfirmationOperation::Mutation(spec.clone()),
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        );
        backend
            .set_management(SetManagementRequest {
                scope: scope.clone(),
                enabled: true,
            })
            .unwrap();
        for (operation, state) in [
            (MutationOperation::Stop, "exited"),
            (MutationOperation::Start, "running"),
        ] {
            let spec = MutationSpec {
                operation,
                ..spec.clone()
            };
            let intent = backend
                .prepare_confirmation(PrepareConfirmationRequest {
                    scope: scope.clone(),
                    operation: ConfirmationOperation::Mutation(spec.clone()),
                })
                .await
                .unwrap();
            let request = MutationRequest {
                scope: scope.clone(),
                intent_id: intent.id,
                spec,
            };
            assert_eq!(
                backend
                    .mutate_container(request.clone())
                    .await
                    .unwrap()
                    .outcome,
                MutationOutcome::Succeeded
            );
            assert_eq!(
                backend.mutate_container(request).await.unwrap_err().code,
                ErrorCode::InvalidIntent
            );
            assert_eq!(
                backend
                    .inspect_container(inspect.clone())
                    .await
                    .unwrap()
                    .summary
                    .state,
                state
            );
        }
        let configuration: ComposeConfiguration =
            serde_json::from_value(lab["configuration"].clone()).unwrap();
        let verified = backend
            .verify_compose_project(VerifyComposeRequest {
                scope: scope.clone(),
                configuration: configuration.clone(),
                acknowledged: true,
            })
            .await
            .unwrap();
        assert_eq!(verified.services, vec!["web", "worker"]);
        let mut expected: Vec<ContainerId> =
            serde_json::from_value(lab["composeIds"].clone()).unwrap();
        expected.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(verified.container_ids, expected);
        let compose = ComposeActionSpec {
            verification_id: verified.id,
            configuration,
            services: verified.services,
            container_ids: verified.container_ids,
            operation: ComposeActionOperation::Restart,
            timeout_seconds: 1,
        };
        let intent = backend
            .prepare_confirmation(PrepareConfirmationRequest {
                scope: scope.clone(),
                operation: ConfirmationOperation::Compose(compose.clone()),
            })
            .await
            .unwrap();
        let request = ComposeMutationRequest {
            scope: scope.clone(),
            intent_id: intent.id,
            spec: compose,
        };
        assert_eq!(
            backend
                .mutate_compose_project(request.clone())
                .await
                .unwrap()
                .outcome,
            MutationOutcome::Succeeded
        );
        assert_eq!(
            backend
                .mutate_compose_project(request)
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidIntent
        );
        backend
            .set_terminal_permission(SetManagementRequest {
                scope: scope.clone(),
                enabled: true,
            })
            .unwrap();
        let terminal = TerminalSpec {
            container_id: target.clone(),
            shell: TerminalShell::Sh,
            columns: 80,
            rows: 24,
        };
        let intent = backend
            .prepare_confirmation(PrepareConfirmationRequest {
                scope: scope.clone(),
                operation: ConfirmationOperation::Terminal(terminal.clone()),
            })
            .await
            .unwrap();
        let opened = backend
            .open_container_terminal(TerminalRequest {
                scope: scope.clone(),
                intent_id: intent.id,
                spec: terminal,
            })
            .await
            .unwrap();
        let handle = TerminalHandleRequest {
            scope: scope.clone(),
            terminal_id: opened.terminal_id,
        };
        receive(&backend, &handle, "$ ").await;
        backend
            .terminal_input(TerminalInputRequest {
                scope: scope.clone(),
                terminal_id: handle.terminal_id.clone(),
                sequence: 1,
                bytes: b"printf 'CD049_%s\\n' PTY\r".to_vec(),
            })
            .unwrap();
        receive(&backend, &handle, "CD049_PTY").await;
        backend
            .resize_terminal(TerminalResizeRequest {
                scope: scope.clone(),
                terminal_id: handle.terminal_id.clone(),
                columns: 111,
                rows: 37,
            })
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        backend
            .terminal_input(TerminalInputRequest {
                scope: scope.clone(),
                terminal_id: handle.terminal_id.clone(),
                sequence: 2,
                bytes: b"stty size\r".to_vec(),
            })
            .unwrap();
        receive(&backend, &handle, "37 111").await;
        backend
            .terminal_input(TerminalInputRequest {
                scope: scope.clone(),
                terminal_id: handle.terminal_id.clone(),
                sequence: 3,
                bytes: b"sleep 30\r".to_vec(),
            })
            .unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        backend
            .terminal_input(TerminalInputRequest {
                scope: scope.clone(),
                terminal_id: handle.terminal_id.clone(),
                sequence: 4,
                bytes: vec![3],
            })
            .unwrap();
        backend
            .terminal_input(TerminalInputRequest {
                scope: scope.clone(),
                terminal_id: handle.terminal_id.clone(),
                sequence: 5,
                bytes: b"printf 'CD058_%s\\n' INTERRUPTED\r".to_vec(),
            })
            .unwrap();
        receive(&backend, &handle, "CD058_INTERRUPTED").await;
        backend.close_terminal(handle.clone()).await.unwrap();
        let stale_intent = backend
            .prepare_confirmation(PrepareConfirmationRequest {
                scope: scope.clone(),
                operation: ConfirmationOperation::Mutation(spec.clone()),
            })
            .await
            .unwrap();
        std::fs::write(root.join(format!("cut-{alias}")), b"owned guest SSH only").unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            while !root.join(format!("cut-{alias}-done")).exists() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert!(
            tokio::time::timeout(
                Duration::from_secs(5),
                backend.list_containers(ListContainersRequest {
                    scope: scope.clone()
                })
            )
            .await
            .unwrap()
            .is_err()
        );
        // The health monitor revokes the lost generation asynchronously. Repeated
        // connects while it still reports Ready deliberately share that same attempt.
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let current = backend
                    .host_inventory(InventoryModeRequest {
                        mode: WorkspaceMode::Live,
                    })
                    .unwrap()
                    .connection
                    .unwrap();
                if current.token != connected.token {
                    assert_eq!(current.state, ConnectionState::Degraded);
                    assert_eq!(
                        current.diagnostic.unwrap().code,
                        ConnectionDiagnosticCode::ConnectionLost
                    );
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        backend.connect_inventory_host(connect).await.unwrap();
        ready(&backend, WorkspaceMode::Live, host_id.clone()).await;
        let next = backend
            .connect_host(ConnectHostRequest {
                selection: HostSelection {
                    host_id,
                    selection_generation: 2,
                },
            })
            .unwrap()
            .scope;
        assert_ne!(scope, next);
        assert!(!backend.management_state(next.clone()).unwrap().enabled);
        assert!(backend.inspect_container(inspect).await.is_err());
        assert!(
            backend
                .mutate_container(MutationRequest {
                    scope,
                    intent_id: stale_intent.id,
                    spec
                })
                .await
                .is_err()
        );
        assert!(
            backend
                .terminal_input(TerminalInputRequest {
                    scope: next.clone(),
                    terminal_id: handle.terminal_id,
                    sequence: 3,
                    bytes: b"must not replay".to_vec()
                })
                .is_err()
        );
        assert_eq!(
            backend
                .list_containers(ListContainersRequest { scope: next })
                .await
                .unwrap()
                .containers
                .len(),
            4
        );
        backend.shutdown().await;
        for item in std::fs::read_dir(root.join(alias)).unwrap() {
            let path = item.unwrap().path();
            if path.is_file() {
                let stored = std::fs::read(path).unwrap();
                let stored = String::from_utf8_lossy(&stored);
                for forbidden in ["CD049_LOG", "CD049_PTY", "SEEDED_049_ENV"] {
                    assert!(!stored.contains(forbidden));
                }
            }
        }
        println!(
            "PASS 049: {alias}: handshake, exact inventory, masked inspect, logs, stats, cancelled follow, read-only denial, stop/start once, verified Compose restart once, PTY echo/resize/Ctrl-C/close, actual SSH session loss, reconnect/new read-only scope, no stale action/input or persisted payload"
        );
    }
    // A real guest Docker stop completes; its wrapper then loses the SSH response.
    // The daemon event oracle must still observe exactly one stop, never an automatic retry.
    let backend = Backend::new(&root.join("uncertain"), "/unused-lab-home".into());
    let mut host = draft("direct-owned", config);
    host.docker.executable = Some("/opt/containerdesk/lost-response-docker".into());
    let saved = backend
        .save_host(SaveHostRequest {
            mode: WorkspaceMode::Live,
            expected_revision: 0,
            id: None,
            draft: host,
        })
        .await
        .unwrap();
    let host_id = saved.saved.preferences.hosts[0].id.clone();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: host_id.clone(),
        })
        .await
        .unwrap();
    ready(&backend, WorkspaceMode::Live, host_id.clone()).await;
    let scope = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id,
                selection_generation: 1,
            },
        })
        .unwrap()
        .scope;
    backend
        .set_management(SetManagementRequest {
            scope: scope.clone(),
            enabled: true,
        })
        .unwrap();
    let spec = MutationSpec {
        operation: MutationOperation::Stop,
        container_ids: vec![target.clone()],
        timeout_seconds: 1,
    };
    let intent = backend
        .prepare_confirmation(PrepareConfirmationRequest {
            scope: scope.clone(),
            operation: ConfirmationOperation::Mutation(spec.clone()),
        })
        .await
        .unwrap();
    let outcome = backend
        .mutate_container(MutationRequest {
            scope: scope.clone(),
            intent_id: intent.id,
            spec,
        })
        .await
        .unwrap();
    assert_eq!(outcome.outcome, MutationOutcome::Unknown);
    let preview = backend.prepare_support_report().unwrap();
    for secret in [
        "SEEDED_049_ENV",
        "CD049_LOG",
        "CD049_PTY",
        config,
        &target.0,
    ] {
        assert!(!preview.report.contains(secret));
    }
    let export = root.join("support-redacted.json");
    crate::log_export::write(&export, std::slice::from_ref(&preview.report)).unwrap();
    assert_eq!(
        std::fs::read_to_string(export).unwrap().trim(),
        preview.report
    );
    backend.shutdown().await;
    // Restore only this owned fixture via a fresh explicit application action.
    let recovery = Backend::new(&root.join("uncertain-recovery"), "/unused-lab-home".into());
    let saved = recovery
        .save_host(SaveHostRequest {
            mode: WorkspaceMode::Live,
            expected_revision: 0,
            id: None,
            draft: draft("direct-owned", config),
        })
        .await
        .unwrap();
    let host_id = saved.saved.preferences.hosts[0].id.clone();
    recovery
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: host_id.clone(),
        })
        .await
        .unwrap();
    ready(&recovery, WorkspaceMode::Live, host_id.clone()).await;
    let scope = recovery
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id,
                selection_generation: 1,
            },
        })
        .unwrap()
        .scope;
    assert_eq!(
        recovery
            .inspect_container(InspectContainerRequest {
                scope: scope.clone(),
                container_id: target.clone(),
                reveal_sensitive: false
            })
            .await
            .unwrap()
            .summary
            .state,
        "exited"
    );
    recovery
        .set_management(SetManagementRequest {
            scope: scope.clone(),
            enabled: true,
        })
        .unwrap();
    let spec = MutationSpec {
        operation: MutationOperation::Start,
        container_ids: vec![target],
        timeout_seconds: 1,
    };
    let intent = recovery
        .prepare_confirmation(PrepareConfirmationRequest {
            scope: scope.clone(),
            operation: ConfirmationOperation::Mutation(spec.clone()),
        })
        .await
        .unwrap();
    assert_eq!(
        recovery
            .mutate_container(MutationRequest {
                scope,
                intent_id: intent.id,
                spec
            })
            .await
            .unwrap()
            .outcome,
        MutationOutcome::Succeeded
    );
    recovery.shutdown().await;
    println!(
        "PASS 058: actual stop with lost SSH response stayed unknown without replay; fresh read reconciled exited state before explicit start; native redacted support file excluded seeded payloads and identities"
    );
    for alias in [
        "direct-bad",
        "private-bad",
        "via-bad-jump",
        "direct-unknown",
    ] {
        let backend = Backend::new(&root.join(alias), "/unused-lab-home".into());
        let saved = backend
            .save_host(SaveHostRequest {
                mode: WorkspaceMode::Live,
                expected_revision: 0,
                id: None,
                draft: draft(alias, config),
            })
            .await
            .unwrap();
        backend
            .connect_inventory_host(InventoryConnectRequest {
                mode: WorkspaceMode::Live,
                host_id: saved.saved.preferences.hosts[0].id.clone(),
            })
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let current = backend
                    .host_inventory(InventoryModeRequest {
                        mode: WorkspaceMode::Live,
                    })
                    .unwrap()
                    .connection
                    .unwrap();
                if current.state == ConnectionState::Error {
                    assert_eq!(
                        current.diagnostic.unwrap().code,
                        if alias == "direct-unknown" {
                            ConnectionDiagnosticCode::UnknownHostKey
                        } else {
                            ConnectionDiagnosticCode::ChangedHostKey
                        }
                    );
                    break;
                }
                assert_ne!(current.state, ConnectionState::Ready);
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        backend.shutdown().await;
        println!(
            "PASS 049: {alias}: native unknown/changed host key rejected before Docker readiness"
        );
    }
    let runner = crate::ssh::runner::Runner::default();
    let began = std::time::Instant::now();
    let command = format!(
        "exec {} {}",
        crate::ssh::quoting::single_quote("sleep").unwrap(),
        crate::ssh::quoting::single_quote("10").unwrap()
    );
    let args = ["-F", config, "-T", "-n", "--", "private-owned", &command]
        .into_iter()
        .map(Into::into)
        .collect();
    let error = runner
        .start(
            "/usr/bin/ssh",
            args,
            crate::ssh::runner::Limits {
                deadline: Duration::from_secs(2),
                ..Default::default()
            },
        )
        .unwrap()
        .wait()
        .await
        .unwrap_err();
    assert_eq!(error, crate::ssh::runner::RunError::TimedOut);
    assert!(began.elapsed() < Duration::from_secs(4));
    runner.wait_idle().await;
    println!(
        "PASS 049: native slow remote command over ProxyJump timed out and reaped within four seconds"
    );
}

async fn receive(backend: &Backend, handle: &TerminalHandleRequest, expected: &str) {
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut bytes = Vec::new();
        loop {
            let value = backend.terminal_output(handle.clone()).unwrap();
            assert_ne!(value.state, TerminalState::Exited);
            bytes.extend(value.bytes);
            assert!(bytes.len() < 256 * 1024);
            if String::from_utf8_lossy(&bytes).contains(expected) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
