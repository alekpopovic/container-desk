use super::*;
use std::time::Duration;
fn draft(alias: &str, config: &str) -> HostDraft {
    HostDraft {
        ssh: SshSelection {
            alias: alias.into(),
            config_path: config.into(),
            use_default_config: false,
        },
        docker: DockerOptions::default(),
        display_name: "Same display name".into(),
        group: "dev".into(),
        labels: vec!["<untrusted>".into()],
        favorite: false,
    }
}
async fn ready(backend: &Backend, mode: WorkspaceMode, id: HostId) -> ConnectionSnapshot {
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            let value = backend
                .host_inventory(InventoryModeRequest { mode: mode.clone() })
                .unwrap();
            let connection = value.connection.unwrap();
            assert_eq!(connection.host_id, Some(id.clone()));
            assert!(
                !matches!(
                    connection.state,
                    ConnectionState::Error | ConnectionState::Degraded
                ),
                "{connection:?}"
            );
            if connection.state == ConnectionState::Ready {
                return connection;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn demo_inventory_has_stable_ids_isolated_metadata_and_host_scoped_connections() {
    let backend = Backend::default();
    let original = backend.preferences().unwrap();
    backend
        .switch_workspace(SwitchWorkspaceRequest::Demo {
            scenario: DemoScenario::Standard,
        })
        .unwrap();
    let first = backend
        .save_host(SaveHostRequest {
            mode: WorkspaceMode::Demo,
            expected_revision: 0,
            id: None,
            draft: draft("demo-direct", "/demo/config"),
        })
        .await
        .unwrap();
    let first_id = first.saved.preferences.hosts[0].id.clone();
    let mut second_draft = draft("demo-jump", "/demo/config");
    second_draft.favorite = true;
    let second = backend
        .save_host(SaveHostRequest {
            mode: WorkspaceMode::Demo,
            expected_revision: 1,
            id: None,
            draft: second_draft,
        })
        .await
        .unwrap();
    let second_id = second.saved.preferences.hosts[1].id.clone();
    assert_ne!(first_id, second_id);
    assert_eq!(
        second.saved.preferences.hosts[0].display_name,
        second.saved.preferences.hosts[1].display_name
    );
    assert!(second.saved.preferences.hosts.iter().all(|h| h.read_only));
    assert!(
        second.connection.is_none(),
        "saving metadata never connects"
    );
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Demo,
            host_id: first_id.clone(),
        })
        .await
        .unwrap();
    let old = ready(&backend, WorkspaceMode::Demo, first_id.clone()).await;
    assert!(!old.has_jump);
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Demo,
            host_id: second_id.clone(),
        })
        .await
        .unwrap();
    let new = ready(&backend, WorkspaceMode::Demo, second_id.clone()).await;
    assert!(new.has_jump);
    assert_eq!(
        backend
            .disconnect_inventory_host(InventoryDisconnectRequest {
                mode: WorkspaceMode::Demo,
                host_id: first_id,
                token: old.token
            })
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleSession
    );
    assert_eq!(
        backend
            .remove_host(RemoveHostRequest {
                mode: WorkspaceMode::Demo,
                expected_revision: 0,
                host_id: second_id.clone()
            })
            .await
            .unwrap_err()
            .code,
        ErrorCode::StorageConflict
    );
    let after = backend
        .remove_host(RemoveHostRequest {
            mode: WorkspaceMode::Demo,
            expected_revision: 2,
            host_id: second_id,
        })
        .await
        .unwrap();
    assert_eq!(after.saved.preferences.hosts.len(), 1);
    assert!(after.connection.is_none());
    assert_eq!(
        backend.preferences().unwrap(),
        original,
        "demo never writes live preferences"
    );
    backend
        .switch_workspace(SwitchWorkspaceRequest::Live)
        .unwrap();
    assert_eq!(
        backend
            .save_host(SaveHostRequest {
                mode: WorkspaceMode::Demo,
                expected_revision: 3,
                id: None,
                draft: draft("late-demo", "/demo/config")
            })
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleSession
    );
}
#[tokio::test]
#[ignore = "requires disposable direct/ProxyJump SSH lab"]
async fn disposable_lab_saved_host_add_connect_disconnect_remove_and_reopen() {
    let manifest = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").expect("explicit lab manifest");
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest)
        .parent()
        .unwrap()
        .join("inventory-app-data");
    let backend = Backend::new(&root, "/unused-lab-home".into());
    let config = lab["config"].as_str().unwrap();
    let original = std::fs::read(config).unwrap();
    let mut ids = Vec::new();
    for (revision, alias) in ["direct-known", "via-known"].into_iter().enumerate() {
        let mut host = draft(alias, config);
        host.docker.executable = Some("/opt/fixture/docker-rootless".into());
        let added = backend
            .save_host(SaveHostRequest {
                mode: WorkspaceMode::Live,
                expected_revision: revision as u32,
                id: None,
                draft: host,
            })
            .await
            .unwrap();
        ids.push(added.saved.preferences.hosts.last().unwrap().id.clone());
    }
    assert_ne!(ids[0], ids[1]);
    assert!(backend.sessions.current().unwrap().is_none());
    let mut previous = None;
    for (index, id) in ids.iter().enumerate() {
        backend
            .connect_inventory_host(InventoryConnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id.clone(),
            })
            .await
            .unwrap();
        let connection = ready(&backend, WorkspaceMode::Live, id.clone()).await;
        assert_eq!(connection.has_jump, index == 1);
        let effective = connection.effective.as_ref().unwrap();
        assert_eq!(effective.user, "lab");
        assert_eq!(effective.proxy_jump.is_some(), index == 1);
        if let Some(token) = previous {
            assert_eq!(
                backend.sessions.snapshot(&token).unwrap_err().code,
                ErrorCode::StaleSession
            );
        }
        backend
            .disconnect_inventory_host(InventoryDisconnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id.clone(),
                token: connection.token.clone(),
            })
            .await
            .unwrap();
        previous = Some(connection.token);
        println!("native saved host: direct/jump {index}, explicit connect and disconnect passed");
    }
    backend
        .remove_host(RemoveHostRequest {
            mode: WorkspaceMode::Live,
            expected_revision: 2,
            host_id: ids[1].clone(),
        })
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(config).unwrap(),
        original,
        "metadata removal cannot change SSH config"
    );
    backend.shutdown().await;
    drop(backend);
    let reopened = Backend::new(&root, "/unused-lab-home".into());
    let saved = reopened
        .host_inventory(InventoryModeRequest {
            mode: WorkspaceMode::Live,
        })
        .unwrap();
    assert_eq!(saved.saved.preferences.hosts.len(), 1);
    assert_eq!(saved.saved.preferences.hosts[0].id, ids[0]);
    assert!(
        saved.connection.is_none(),
        "restart loads metadata, never connections"
    );
    reopened.shutdown().await;
}

#[tokio::test]
async fn editing_metadata_keeps_identity_but_editing_target_cancels_the_old_session() {
    let backend = Backend::default();
    backend
        .switch_workspace(SwitchWorkspaceRequest::Demo {
            scenario: DemoScenario::Standard,
        })
        .unwrap();
    let mut metadata = draft("demo-direct", "/demo/config");
    let saved = backend
        .save_host(SaveHostRequest {
            mode: WorkspaceMode::Demo,
            expected_revision: 0,
            id: None,
            draft: metadata.clone(),
        })
        .await
        .unwrap();
    let id = saved.saved.preferences.hosts[0].id.clone();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Demo,
            host_id: id.clone(),
        })
        .await
        .unwrap();
    let original = ready(&backend, WorkspaceMode::Demo, id.clone()).await;
    metadata.favorite = true;
    let same = backend
        .save_host(SaveHostRequest {
            mode: WorkspaceMode::Demo,
            expected_revision: 1,
            id: Some(id.clone()),
            draft: metadata.clone(),
        })
        .await
        .unwrap();
    assert_eq!(same.connection.unwrap().token, original.token);
    metadata.ssh.alias = "demo-jump".into();
    let changed = backend
        .save_host(SaveHostRequest {
            mode: WorkspaceMode::Demo,
            expected_revision: 2,
            id: Some(id.clone()),
            draft: metadata,
        })
        .await
        .unwrap();
    assert!(
        changed.connection.is_none(),
        "old target data must not attach to edited metadata"
    );
    assert_eq!(
        backend.sessions.snapshot(&original.token).unwrap_err().code,
        ErrorCode::StaleSession
    );
    assert_eq!(changed.saved.preferences.hosts[0].id, id);
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Demo,
            host_id: id.clone(),
        })
        .await
        .unwrap();
    assert!(ready(&backend, WorkspaceMode::Demo, id).await.has_jump);
    backend.shutdown().await;
}

#[tokio::test]
#[ignore = "requires disposable real Engine with two listing containers"]
async fn checkpoint020_live_inventory_binds_reads_and_cancels_inflight_without_idle_mutex_deadlock()
{
    let manifest = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(lab["realEngine"], true);
    let root = std::path::Path::new(&manifest)
        .parent()
        .unwrap()
        .join("resource-app-data");
    let backend = std::sync::Arc::new(Backend::new(&root, "/unused-lab-home".into()));
    let config = lab["config"].as_str().unwrap();
    let expected: std::collections::HashSet<String> =
        serde_json::from_value(lab["expectedContainerIds"].clone()).unwrap();
    let mut ids = vec![];
    for (revision, alias) in ["direct-known", "via-known"].into_iter().enumerate() {
        let host = draft(alias, config);
        let saved = backend
            .save_host(SaveHostRequest {
                mode: WorkspaceMode::Live,
                expected_revision: revision as u32,
                id: None,
                draft: host,
            })
            .await
            .unwrap();
        ids.push(saved.saved.preferences.hosts.last().unwrap().id.clone());
    }
    let mut previous = None;
    for (index, id) in ids.iter().enumerate() {
        backend
            .connect_inventory_host(InventoryConnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id.clone(),
            })
            .await
            .unwrap();
        let connected = ready(&backend, WorkspaceMode::Live, id.clone()).await;
        let response = backend
            .connect_host(ConnectHostRequest {
                selection: HostSelection {
                    host_id: id.clone(),
                    selection_generation: index as u32 + 1,
                },
            })
            .unwrap();
        assert!(!response.capabilities.management && !response.capabilities.terminal);
        if let Some(old) = previous.take() {
            assert!(
                backend
                    .list_containers(ListContainersRequest { scope: old })
                    .await
                    .is_err()
            );
        }
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            backend.list_containers(ListContainersRequest {
                scope: response.scope.clone(),
            }),
        )
        .await
        .expect("idle watcher must not retain connection lock")
        .unwrap();
        assert_eq!(
            result
                .containers
                .iter()
                .map(|row| row.id.0.clone())
                .collect::<std::collections::HashSet<_>>(),
            expected
        );
        backend
            .disconnect_inventory_host(InventoryDisconnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id.clone(),
                token: connected.token,
            })
            .await
            .unwrap();
        assert!(
            backend
                .list_containers(ListContainersRequest {
                    scope: response.scope.clone()
                })
                .await
                .is_err()
        );
        previous = Some(response.scope);
    }
    let mut slow = draft("via-known", config);
    slow.docker.executable = Some("/opt/fixture/docker-list-hang".into());
    backend
        .save_host(SaveHostRequest {
            mode: WorkspaceMode::Live,
            expected_revision: 2,
            id: Some(ids[1].clone()),
            draft: slow,
        })
        .await
        .unwrap();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: ids[1].clone(),
        })
        .await
        .unwrap();
    let connected = ready(&backend, WorkspaceMode::Live, ids[1].clone()).await;
    let bound = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: ids[1].clone(),
                selection_generation: 3,
            },
        })
        .unwrap();
    let reader = backend.clone();
    let reading = tokio::spawn(async move {
        reader
            .list_containers(ListContainersRequest { scope: bound.scope })
            .await
    });
    let mut observer = crate::ssh::multiplex::Connection::new(
        "/usr/bin/ssh",
        SshSelection {
            alias: "direct-known".into(),
            config_path: config.into(),
            use_default_config: false,
        },
    )
    .unwrap();
    assert_eq!(
        observer.start().await.unwrap().status,
        SshAccessStatus::Verified
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let check = observer
                .start_fixed(
                    crate::ssh::quoting::command(&[
                        "test".into(),
                        "-f".into(),
                        "/tmp/list-inflight".into(),
                    ])
                    .unwrap(),
                    crate::ssh::runner::Limits::default(),
                )
                .unwrap()
                .wait()
                .await
                .unwrap();
            if check.status.success() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("the controlled ps command must actually be dispatched before cancellation");
    observer.close().await;
    assert!(
        !reading.is_finished(),
        "slow list must actually be in flight"
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        backend.disconnect_inventory_host(InventoryDisconnectRequest {
            mode: WorkspaceMode::Live,
            host_id: ids[1].clone(),
            token: connected.token,
        }),
    )
    .await
    .expect("disconnect must cancel reads without waiting for their connection mutex")
    .unwrap();
    assert!(reading.await.unwrap().is_err());
    assert_eq!(backend.read_slots.available_permits(), 4);
    assert_eq!(backend.diagnostic_slot.available_permits(), 1);
    let mut drift = draft("direct-known", config);
    drift.docker.executable = Some("/opt/fixture/docker-list-change".into());
    backend
        .save_host(SaveHostRequest {
            mode: WorkspaceMode::Live,
            expected_revision: 3,
            id: Some(ids[0].clone()),
            draft: drift,
        })
        .await
        .unwrap();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: ids[0].clone(),
        })
        .await
        .unwrap();
    ready(&backend, WorkspaceMode::Live, ids[0].clone()).await;
    let bound = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: ids[0].clone(),
                selection_generation: 4,
            },
        })
        .unwrap();
    assert_eq!(
        backend
            .list_containers(ListContainersRequest {
                scope: bound.scope.clone()
            })
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleSession
    );
    assert!(backend.require_session(&bound.scope).is_err());
    assert_eq!(
        backend.sessions.current().unwrap().unwrap().state,
        ConnectionState::Disconnected
    );
    assert_eq!(backend.diagnostic_slot.available_permits(), 1);
    backend.shutdown().await;
    println!(
        "PASS live resource sessions: direct/ProxyJump actual IDs, old/disconnected scopes rejected, inflight read cancelled below 3s, all permits released; controlled identity drift revokes the session"
    );
}

#[tokio::test]
#[ignore = "requires disposable real Engine with inspect secret fixtures"]
async fn checkpoint021_live_inspect_redacts_reveals_and_rejects_disconnected_scope() {
    let manifest = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(lab["realEngine"], true);
    let root = std::path::Path::new(&manifest)
        .parent()
        .unwrap()
        .join("inspect-app-data");
    let backend = Backend::new(&root, "/unused-lab-home".into());
    let container = ContainerId(lab["expectedContainerIds"][0].as_str().unwrap().into());
    let secrets = ["synthetic-inspect-021-secret", "synthetic-label-021-secret"];
    for (revision, alias) in ["direct-known", "via-known"].into_iter().enumerate() {
        let saved = backend
            .save_host(SaveHostRequest {
                mode: WorkspaceMode::Live,
                expected_revision: revision as u32,
                id: None,
                draft: draft(alias, lab["config"].as_str().unwrap()),
            })
            .await
            .unwrap();
        let id = saved.saved.preferences.hosts.last().unwrap().id.clone();
        backend
            .connect_inventory_host(InventoryConnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id.clone(),
            })
            .await
            .unwrap();
        let connection = ready(&backend, WorkspaceMode::Live, id.clone()).await;
        let scope = backend
            .connect_host(ConnectHostRequest {
                selection: HostSelection {
                    host_id: id.clone(),
                    selection_generation: revision as u32 + 1,
                },
            })
            .unwrap()
            .scope;
        let mut request = InspectContainerRequest {
            scope: scope.clone(),
            container_id: container.clone(),
            reveal_sensitive: false,
        };
        let masked = backend.inspect_container(request.clone()).await.unwrap();
        assert_eq!(masked.summary.id, container);
        assert_eq!(masked.summary.name, "listing-first");
        assert_eq!(masked.summary.state, "created");
        assert!(masked.image_id.is_some());
        assert!(masked.created_at.is_some());
        assert_eq!(masked.summary.health, None);
        assert_eq!(masked.healthcheck_configured, Some(false));
        assert_eq!(masked.oom_killed, Some(false));
        assert_eq!(masked.exposed_ports.len(), 2);
        assert!(masked.summary.ports.iter().all(|p| p.public_port.is_none()));
        let wire = serde_json::to_string(&masked).unwrap();
        for secret in secrets {
            assert!(!wire.contains(secret));
        }
        assert!(
            masked
                .environment
                .iter()
                .any(|v| v.name == "CHECKPOINT_TOKEN" && v.value.is_none())
        );
        request.reveal_sensitive = true;
        let revealed = backend.inspect_container(request.clone()).await.unwrap();
        assert_eq!(
            revealed
                .environment
                .iter()
                .find(|v| v.name == "CHECKPOINT_TOKEN")
                .unwrap()
                .value
                .as_deref(),
            Some(secrets[0])
        );
        assert_eq!(
            revealed
                .labels
                .iter()
                .find(|v| v.name == "innocent")
                .unwrap()
                .value
                .as_deref(),
            Some(secrets[1])
        );
        for secret in secrets {
            assert!(!format!("{revealed:?}").contains(secret));
        }
        request.reveal_sensitive = false;
        assert!(
            backend
                .inspect_container(request.clone())
                .await
                .unwrap()
                .environment
                .iter()
                .all(|v| v.value.is_none())
        );
        let mut missing = request.clone();
        missing.container_id = ContainerId("0".repeat(64));
        assert_eq!(
            backend.inspect_container(missing).await.unwrap_err().code,
            ErrorCode::ContainerNotFound
        );
        let mut forged = request.clone();
        forged.scope.daemon_id = "foreign".into();
        assert!(backend.inspect_container(forged).await.is_err());
        backend
            .disconnect_inventory_host(InventoryDisconnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id,
                token: connection.token,
            })
            .await
            .unwrap();
        request.reveal_sensitive = true;
        assert!(backend.inspect_container(request).await.is_err());
        assert_eq!(backend.read_slots.available_permits(), 4);
        for file in std::fs::read_dir(&root).unwrap() {
            let path = file.unwrap().path();
            if path.is_file() {
                let bytes = std::fs::read(path).unwrap();
                for secret in secrets {
                    assert!(!String::from_utf8_lossy(&bytes).contains(secret));
                }
            }
        }
        println!(
            "PASS real inspect {alias}: exact ID, created lifecycle/image, default masking, explicit reveal, missing ID, stale scope; no values in persisted app files or Debug"
        );
    }
    backend.shutdown().await;
}

#[tokio::test]
#[ignore = "requires owned loopback SSH and exact labelled native Docker log fixtures"]
async fn checkpoint023_owned_logs_match_native_cli_and_remain_transient() {
    use std::os::unix::process::ExitStatusExt;
    let manifest = std::env::var("CONTAINERDESK_LOG_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest)
        .parent()
        .unwrap()
        .join("app-data");
    let backend = Backend::new(&root, "/unused-lab-home".into());
    let mut host = draft("logs-owned", lab["config"].as_str().unwrap());
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
    let id = saved.saved.preferences.hosts[0].id.clone();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: id.clone(),
        })
        .await
        .unwrap();
    let connected = ready(&backend, WorkspaceMode::Live, id.clone()).await;
    let scope = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: id.clone(),
                selection_generation: 1,
            },
        })
        .unwrap()
        .scope;
    let request = ContainerLogsRequest {
        scope: scope.clone(),
        container_id: ContainerId(lab["containerId"].as_str().unwrap().into()),
        tail: 100,
        timeout_seconds: 30,
        since: None,
        until: None,
    };
    let snapshot = backend.container_logs(request.clone()).await.unwrap();
    let oracle = crate::docker::logs::decode(
        &request,
        crate::ssh::runner::Captured {
            status: std::process::ExitStatus::from_raw(0),
            stdout: std::fs::read(lab["oracleStdout"].as_str().unwrap()).unwrap(),
            stderr: std::fs::read(lab["oracleStderr"].as_str().unwrap()).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(snapshot, oracle);
    assert_eq!(snapshot.records[0].text, "023-stdout-one");
    assert_eq!(snapshot.records[1].text, "023-stderr-two");
    assert_eq!(snapshot.records[1].channel, LogChannel::StderrAmbiguous);
    assert_eq!(snapshot.records[2].text, "023-stdout-three");
    // json-file normalizes invalid input before the CLI emits it; raw-invalid transport is covered by parser fixtures.
    assert!(
        snapshot
            .records
            .iter()
            .any(|r| r.text.contains("\u{fffd}023-invalid"))
    );
    let raw_stderr = std::fs::read(lab["oracleStderr"].as_str().unwrap()).unwrap();
    let oversized = raw_stderr
        .split(|b| *b == b'\n')
        .any(|line| line.len() > 256 * 1024 + 31);
    assert_eq!(snapshot.truncated, oversized);
    assert!(snapshot.records.iter().all(|r| r.text.len() <= 256 * 1024));
    assert!(!format!("{snapshot:?}").contains("023-synthetic-log-private"));
    let mut range = request.clone();
    range.until = Some("1".into());
    assert!(
        backend
            .container_logs(range)
            .await
            .unwrap()
            .records
            .is_empty()
    );
    let mut unsupported = request.clone();
    unsupported.container_id = ContainerId(lab["unsupportedId"].as_str().unwrap().into());
    assert_eq!(
        backend.container_logs(unsupported).await.unwrap_err().code,
        ErrorCode::LogDriverUnsupported
    );
    let mut invalid = request.clone();
    invalid.tail = 0;
    assert_eq!(
        backend.container_logs(invalid).await.unwrap_err().code,
        ErrorCode::InvalidLimits
    );
    backend
        .disconnect_inventory_host(InventoryDisconnectRequest {
            mode: WorkspaceMode::Live,
            host_id: id,
            token: connected.token,
        })
        .await
        .unwrap();
    assert!(backend.container_logs(request).await.is_err());
    assert_eq!(backend.read_slots.available_permits(), 4);
    for entry in std::fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            assert!(
                !String::from_utf8_lossy(&std::fs::read(path).unwrap())
                    .contains("023-synthetic-log-private")
            );
        }
    }
    backend.shutdown().await;
    println!(
        "PASS real owned logs: exact independent CLI comparison, stdout/stderr order, exited container, unsupported driver, Engine-normalized UTF-8, actual CLI line bounds, range, stale scope and no persisted raw logs; PATH excludes client tools"
    );
}

#[tokio::test]
#[ignore = "requires owned native SSH stream lab and explicit server-loss controller"]
async fn checkpoint024_owned_stream_bounds_stop_and_network_loss() {
    let manifest = std::env::var("CONTAINERDESK_LOG_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest).parent().unwrap();
    let backend = Backend::new(&root.join("stream-app-data"), "/unused-lab-home".into());
    let mut host = draft("logs-owned", lab["config"].as_str().unwrap());
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
    let id = saved.saved.preferences.hosts[0].id.clone();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: id.clone(),
        })
        .await
        .unwrap();
    ready(&backend, WorkspaceMode::Live, id.clone()).await;
    let scope = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: id,
                selection_generation: 1,
            },
        })
        .unwrap()
        .scope;
    let request = FollowLogsRequest {
        scope: scope.clone(),
        container_id: ContainerId(lab["liveId"].as_str().unwrap().into()),
        tail: 100,
        since: None,
    };
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let response = backend
        .follow_container_logs(
            request.clone(),
            Arc::new(move |batch| sender.try_send(batch).map_err(|_| ())),
        )
        .await
        .unwrap();
    let ack = |sequence| AckLogsRequest {
        scope: scope.clone(),
        subscription_id: response.subscription_id.clone(),
        sequence,
    };
    backend.ack_container_logs(ack(0)).unwrap();
    let first = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(
        first
            .records
            .iter()
            .any(|r| r.text == "024-synthetic-live-line")
    );
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(
        receiver.try_recv().is_err(),
        "no second IPC batch before ACK"
    );
    backend.ack_container_logs(ack(first.sequence)).unwrap();
    let next = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(next.gap && next.dropped_records > 0);
    println!(
        "Native paused consumer: explicit drop marker = {} records, max IPC batch = {} records",
        next.dropped_records,
        next.records.len()
    );
    tokio::time::timeout(
        Duration::from_secs(3),
        backend.cancel_subscription(CancelSubscriptionRequest {
            scope: scope.clone(),
            subscription_id: response.subscription_id,
        }),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(backend.read_slots.available_permits(), 4);
    // A second actual SSH stream loses its owned sshd/connection under heavy output.
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let response = backend
        .follow_container_logs(
            request,
            Arc::new(move |batch| sender.try_send(batch).map_err(|_| ())),
        )
        .await
        .unwrap();
    backend
        .ack_container_logs(AckLogsRequest {
            scope: scope.clone(),
            subscription_id: response.subscription_id.clone(),
            sequence: 0,
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    std::fs::write(root.join("cut-network"), b"owned-server-only").unwrap();
    let started = std::time::Instant::now();
    while backend.read_slots.available_permits() != 4 {
        assert!(
            started.elapsed() < Duration::from_secs(6),
            "network loss must reap owned stream and release permit without renderer ACK"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let result = backend
        .cancel_subscription(CancelSubscriptionRequest {
            scope,
            subscription_id: response.subscription_id,
        })
        .await;
    assert!(result.is_ok() || result.unwrap_err().code == ErrorCode::SubscriptionNotFound);
    backend.shutdown().await;
    for entry in std::fs::read_dir(root.join("stream-app-data")).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            assert!(
                !String::from_utf8_lossy(&std::fs::read(path).unwrap())
                    .contains("024-synthetic-live-line")
            );
        }
    }
    println!(
        "PASS native stream: bounded ACK delivery, visible dropped count, explicit stop after reaping, actual server loss under load releases permits without ACK; no raw log persistence"
    );
}

#[tokio::test]
#[ignore = "requires owned native stats SSH/Docker lab and disappearance controller"]
async fn checkpoint026_owned_stats_running_stopped_disappeared_and_disconnect() {
    let manifest = std::env::var("CONTAINERDESK_LOG_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest)
        .parent()
        .unwrap()
        .join("stats-app-data");
    let backend = Backend::new(&root, "/unused-lab-home".into());
    let mut host = draft("logs-owned", lab["config"].as_str().unwrap());
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
    let id = saved.saved.preferences.hosts[0].id.clone();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: id.clone(),
        })
        .await
        .unwrap();
    let connected = ready(&backend, WorkspaceMode::Live, id.clone()).await;
    let scope = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: id.clone(),
                selection_generation: 1,
            },
        })
        .unwrap()
        .scope;

    let request = ContainerStatsRequest {
        scope: scope.clone(),
        container_id: ContainerId(lab["liveId"].as_str().unwrap().into()),
    };
    let (first, second) = tokio::join!(
        backend.container_stats(request.clone()),
        backend.container_stats(request.clone())
    );
    let sample = match (first, second) {
        (Ok(s), Err(e)) | (Err(e), Ok(s)) => {
            assert_eq!(e.code, ErrorCode::ResourceLimit);
            s
        }
        other => panic!("one host permits only one sample: {other:?}"),
    };
    assert_eq!(sample.availability, StatsAvailability::Available);
    assert!(sample.values.memory_limit_bytes.is_some_and(|n| n > 0.0));
    assert!(sample.values.cpu_percent.is_some());
    let stopped = backend
        .container_stats(ContainerStatsRequest {
            scope: scope.clone(),
            container_id: ContainerId(lab["containerId"].as_str().unwrap().into()),
        })
        .await
        .unwrap();
    assert_eq!(stopped.availability, StatsAvailability::Stopped);
    assert_eq!(stopped.values, StatsValues::default());
    let pending = backend.container_stats(request.clone());
    let stop = async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        backend
            .disconnect_inventory_host(InventoryDisconnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id.clone(),
                token: connected.token,
            })
            .await
            .unwrap();
    };
    let began = std::time::Instant::now();
    let (read, ()) = tokio::join!(pending, stop);
    assert!(read.is_err());
    assert!(began.elapsed() < std::time::Duration::from_secs(5));
    assert_eq!(backend.read_slots.available_permits(), 4);
    assert!(backend.container_stats(request).await.is_err());
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: id.clone(),
        })
        .await
        .unwrap();
    ready(&backend, WorkspaceMode::Live, id.clone()).await;
    let next = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: id,
                selection_generation: 2,
            },
        })
        .unwrap()
        .scope;
    let request = ContainerStatsRequest {
        scope: next,
        container_id: ContainerId(lab["liveId"].as_str().unwrap().into()),
    };
    std::fs::write(
        root.parent().unwrap().join("remove-during-stats"),
        b"owned fixture only",
    )
    .unwrap();
    let vanished = backend.container_stats(request.clone()).await.unwrap();
    assert_eq!(vanished.availability, StatsAvailability::Missing);
    assert_eq!(vanished.values, StatsValues::default());
    assert_eq!(
        backend.container_stats(request).await.unwrap().availability,
        StatsAvailability::Missing
    );
    backend.shutdown().await;
    println!(
        "PASS native stats: actual CLI values, per-host concurrent request denied, stopped zero output replaced by a gap, disappearance between state/stats reads is a gap, disconnect reaps in-flight read and restores slots; old scope rejected."
    );
}

#[tokio::test]
#[ignore = "requires owned Docker events SSH lab with lifecycle controller"]
async fn checkpoint027_owned_events_reconnect_gap_and_deleted_before_inspect() {
    let manifest = std::env::var("CONTAINERDESK_LOG_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest).parent().unwrap();
    let backend = Backend::new(&root.join("events-app-data"), "/unused-lab-home".into());
    let mut host = draft("logs-owned", lab["config"].as_str().unwrap());
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
    let id = saved.saved.preferences.hosts[0].id.clone();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: id.clone(),
        })
        .await
        .unwrap();
    ready(&backend, WorkspaceMode::Live, id.clone()).await;
    let scope = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: id,
                selection_generation: 1,
            },
        })
        .unwrap()
        .scope;
    let target = ContainerId(lab["containerId"].as_str().unwrap().into());
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let response = backend
        .follow_docker_events(
            FollowEventsRequest {
                scope: scope.clone(),
                since: None,
            },
            Arc::new(move |b| sender.try_send(b).map_err(|_| ())),
        )
        .await
        .unwrap();
    assert_eq!(
        backend
            .follow_docker_events(
                FollowEventsRequest {
                    scope: scope.clone(),
                    since: None
                },
                Arc::new(|_| Ok(()))
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    let ack = |sequence| AckLogsRequest {
        scope: scope.clone(),
        subscription_id: response.subscription_id.clone(),
        sequence,
    };
    backend.ack_docker_events(ack(0)).unwrap();
    let first = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(first.gap, "New streams never promise a complete history");
    backend.ack_docker_events(ack(first.sequence)).unwrap();
    std::fs::write(root.join("event-start"), b"owned fixture only").unwrap();
    let mut seen = Vec::new();
    let mut since = None;
    tokio::time::timeout(Duration::from_secs(10), async {
        while !seen.contains(&ContainerEventAction::Die) {
            let batch = receiver.recv().await.unwrap();
            assert!(!batch.ended);
            for event in batch.events {
                assert_eq!(event.actor_id, target);
                let nanos = event.timestamp_unix_nanos.parse::<u64>().unwrap();
                since = Some(format!(
                    "{}.{:09}",
                    nanos / 1_000_000_000,
                    nanos % 1_000_000_000
                ));
                seen.push(event.action);
            }
            backend.ack_docker_events(ack(batch.sequence)).unwrap();
        }
    })
    .await
    .unwrap();
    assert!(seen.contains(&ContainerEventAction::Start));
    let mut foreign = scope.clone();
    foreign.session_generation += 1;
    assert_eq!(
        backend
            .cancel_subscription(CancelSubscriptionRequest {
                scope: foreign,
                subscription_id: response.subscription_id.clone()
            })
            .await
            .unwrap_err()
            .code,
        ErrorCode::SubscriptionNotFound
    );
    backend
        .cancel_subscription(CancelSubscriptionRequest {
            scope: scope.clone(),
            subscription_id: response.subscription_id,
        })
        .await
        .unwrap();
    assert_eq!(backend.read_slots.available_permits(), 4);
    std::fs::write(root.join("event-delete"), b"owned stopped fixture only").unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        while !root.join("event-delete-done").exists() {
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        backend
            .inspect_container(InspectContainerRequest {
                scope: scope.clone(),
                container_id: target.clone(),
                reveal_sensitive: false
            })
            .await
            .unwrap_err()
            .code,
        ErrorCode::ContainerNotFound
    );
    let snapshot = backend
        .list_containers(ListContainersRequest {
            scope: scope.clone(),
        })
        .await
        .unwrap();
    assert!(!snapshot.containers.iter().any(|c| c.id == target));
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let response = backend
        .follow_docker_events(
            FollowEventsRequest {
                scope: scope.clone(),
                since,
            },
            Arc::new(move |b| sender.try_send(b).map_err(|_| ())),
        )
        .await
        .unwrap();
    backend
        .ack_docker_events(AckLogsRequest {
            scope: scope.clone(),
            subscription_id: response.subscription_id.clone(),
            sequence: 0,
        })
        .unwrap();
    let first = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(first.gap);
    assert!(
        first
            .events
            .iter()
            .any(|e| e.actor_id == target && e.action == ContainerEventAction::Destroy)
    );
    // Lose the owned server without returning ACK; shared runner must reap and release slots.
    std::fs::write(root.join("cut-network"), b"owned server only").unwrap();
    tokio::time::timeout(Duration::from_secs(6), async {
        while backend.read_slots.available_permits() != 4 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    backend.shutdown().await;
    assert_eq!(backend.event_streams.active_count(), 0);
    println!(
        "PASS native events: scoped start/die/destroy records; one-stream bound; foreign cancel denied; stop releases permits; deletion during gap gives ContainerNotFound and authoritative snapshot removal; replay still marks gap; real SSH loss reaps without ACK."
    );
}

#[tokio::test]
#[ignore = "requires owned SSH/Docker slow-read command gate"]
async fn checkpoint028_owned_reads_bound_slow_daemon_and_cancel_old_generation() {
    let manifest = std::env::var("CONTAINERDESK_LOG_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest).parent().unwrap();
    let backend = Backend::new(&root.join("reads-app-data"), "/unused-lab-home".into());
    let mut host = draft("logs-owned", lab["config"].as_str().unwrap());
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
    let id = saved.saved.preferences.hosts[0].id.clone();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: id.clone(),
        })
        .await
        .unwrap();
    let connected = ready(&backend, WorkspaceMode::Live, id.clone()).await;
    let scope = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: id.clone(),
                selection_generation: 1,
            },
        })
        .unwrap()
        .scope;
    std::fs::write(root.join("delay-list"), b"owned delayed read").unwrap();
    let first = backend.list_containers(ListContainersRequest {
        scope: scope.clone(),
    });
    let second = backend.list_containers(ListContainersRequest {
        scope: scope.clone(),
    });
    let control = async {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let started = std::fs::read_dir(root)
                    .unwrap()
                    .filter_map(Result::ok)
                    .filter(|e| e.file_name().to_string_lossy().starts_with("list-started-"))
                    .count();
                if started == 2 {
                    break;
                }
                assert!(started < 3);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(backend.read_hosts.count(&id), 2);
        assert_eq!(backend.read_slots.available_permits(), 2);
        assert_eq!(
            backend
                .list_containers(ListContainersRequest {
                    scope: scope.clone()
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        let began = std::time::Instant::now();
        backend
            .disconnect_inventory_host(InventoryDisconnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id.clone(),
                token: connected.token,
            })
            .await
            .unwrap();
        assert!(began.elapsed() < Duration::from_secs(5));
    };
    let (first, second, ()) = tokio::join!(first, second, control);
    assert!(first.is_err() && second.is_err());
    assert_eq!(backend.read_hosts.count(&id), 0);
    assert_eq!(backend.read_slots.available_permits(), 4);
    std::fs::remove_file(root.join("delay-list")).unwrap();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: id.clone(),
        })
        .await
        .unwrap();
    ready(&backend, WorkspaceMode::Live, id.clone()).await;
    let next = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: id,
                selection_generation: 2,
            },
        })
        .unwrap()
        .scope;
    assert_ne!(scope.session_id, next.session_id);
    assert!(
        backend
            .list_containers(ListContainersRequest { scope })
            .await
            .is_err()
    );
    let snapshot = backend
        .list_containers(ListContainersRequest { scope: next })
        .await
        .unwrap();
    assert_eq!(snapshot.containers.len(), 3);
    backend.shutdown().await;
    println!(
        "PASS native read scheduler admission: two slow SSH/Docker reads at most, third rejected, disconnect reaps both and releases all slots, old scope rejected, fresh session returns three owned containers."
    );
}

#[tokio::test]
#[ignore = "requires isolated Engine/SSH Compose lab, plugin enabled and absent wrappers"]
async fn checkpoint029_owned_compose_projects_plugin_and_label_fallback() {
    let manifest = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest).parent().unwrap();
    for (index, (alias, binary, plugin)) in [
        (
            "direct-known",
            "/opt/fixture/docker-with-compose",
            ComposeAvailability::Available,
        ),
        ("via-known", "/usr/bin/docker", ComposeAvailability::Absent),
    ]
    .into_iter()
    .enumerate()
    {
        let backend = Backend::new(
            &root.join(format!("compose-{index}")),
            "/unused-lab-home".into(),
        );
        let mut host = draft(alias, lab["config"].as_str().unwrap());
        host.docker.executable = Some(binary.into());
        let saved = backend
            .save_host(SaveHostRequest {
                mode: WorkspaceMode::Live,
                expected_revision: 0,
                id: None,
                draft: host,
            })
            .await
            .unwrap();
        let id = saved.saved.preferences.hosts[0].id.clone();
        backend
            .connect_inventory_host(InventoryConnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id.clone(),
            })
            .await
            .unwrap();
        let connected = ready(&backend, WorkspaceMode::Live, id.clone()).await;
        let scope = backend
            .connect_host(ConnectHostRequest {
                selection: HostSelection {
                    host_id: id.clone(),
                    selection_generation: 1,
                },
            })
            .unwrap()
            .scope;
        let response = backend
            .list_compose(ListComposeRequest {
                scope: scope.clone(),
            })
            .await
            .unwrap();
        assert_eq!(response.plugin, plugin);
        assert!(response.listing_error.is_none());
        assert_eq!(response.projects.len(), 2);
        for p in &response.projects {
            assert!(p.from_labels);
            assert_eq!(p.from_plugin, plugin == ComposeAvailability::Available);
            assert_eq!(p.configuration, ComposeConfigurationStatus::Unverified);
            assert_eq!(p.instances.len(), 1);
            assert_eq!(p.instances[0].service.as_deref(), Some("web"));
            assert_eq!(p.instances[0].state, "created");
        }
        assert!(
            response.projects[0]
                .config_files_reported
                .contains(&"/remote/nonexistent/compose.yml".into())
        );
        let detail = backend
            .inspect_container(InspectContainerRequest {
                scope: scope.clone(),
                container_id: response.projects[0].instances[0].container_id.clone(),
                reveal_sensitive: false,
            })
            .await
            .unwrap();
        assert_eq!(detail.summary.compose.unwrap().project, "checkpoint-a");
        assert!(detail.environment_values_masked);
        assert_eq!(
            backend
                .host_inventory(InventoryModeRequest {
                    mode: WorkspaceMode::Live
                })
                .unwrap()
                .connection
                .unwrap()
                .token,
            connected.token
        );
        let mut foreign = scope;
        foreign.daemon_id = "other-daemon".into();
        assert!(
            backend
                .list_compose(ListComposeRequest { scope: foreign })
                .await
                .is_err()
        );
        backend.shutdown().await;
    }
    println!(
        "PASS native Compose: real CLI/isolated Engine direct with plugin and ProxyJump without plugin; two projects retain separate web services and created instances; nonexistent paths remain unverified; inspect supplements exact labels; same session reused and foreign daemon rejected."
    );
}

#[tokio::test]
#[ignore = "requires explicitly owned lifecycle SSH/Docker lab"]
async fn checkpoint032_owned_mutations_and_disconnect_never_replay() {
    use std::time::Duration;
    let manifest = std::env::var("CONTAINERDESK_LOG_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let control = std::path::Path::new(&manifest).parent().unwrap();
    let root = control.join("management-app-data");
    let backend = Backend::new(&root, "/unused-lab-home".into());
    let mut host = draft("logs-owned", lab["config"].as_str().unwrap());
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
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: host_id.clone(),
        })
        .await
        .unwrap();
    let connected = ready(&backend, WorkspaceMode::Live, host_id.clone()).await;
    let scope = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: host_id.clone(),
                selection_generation: 1,
            },
        })
        .unwrap()
        .scope;
    let target = ContainerId(lab["liveId"].as_str().unwrap().into());
    async fn confirm(
        backend: &Backend,
        scope: &SessionScope,
        target: &ContainerId,
        operation: MutationOperation,
    ) -> Result<MutationRequest, AppError> {
        let spec = MutationSpec {
            operation,
            container_ids: vec![target.clone()],
            timeout_seconds: 1,
        };
        let intent = backend
            .prepare_confirmation(PrepareConfirmationRequest {
                scope: scope.clone(),
                operation: ConfirmationOperation::Mutation(spec.clone()),
            })
            .await?;
        Ok(MutationRequest {
            scope: scope.clone(),
            intent_id: intent.id,
            spec,
        })
    }
    async fn inspect(
        backend: &Backend,
        scope: &SessionScope,
        target: &ContainerId,
    ) -> ContainerDetail {
        backend
            .inspect_container(InspectContainerRequest {
                scope: scope.clone(),
                container_id: target.clone(),
                reveal_sensitive: false,
            })
            .await
            .unwrap()
    }
    let baseline = std::fs::read_to_string(control.join("mutation-count"))
        .unwrap_or_default()
        .lines()
        .count();
    assert_eq!(
        confirm(&backend, &scope, &target, MutationOperation::Stop)
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
    assert_eq!(
        confirm(
            &backend,
            &scope,
            &ContainerId("f".repeat(64)),
            MutationOperation::Stop
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::ContainerNotFound
    );
    let first_slot = backend.read_hosts.acquire(&host_id).unwrap();
    let second_slot = backend.read_hosts.acquire(&host_id).unwrap();
    let preparation = confirm(&backend, &scope, &target, MutationOperation::Stop);
    let release = async {
        tokio::time::sleep(Duration::from_millis(250)).await;
        drop(first_slot);
        drop(second_slot);
    };
    let (prepared, ()) = tokio::join!(preparation, release);
    assert!(
        prepared.is_ok(),
        "Confirmation must await admission before its single native read"
    );
    let before = inspect(&backend, &scope, &target).await.started_at;
    for (operation, expected) in [
        (MutationOperation::Stop, "exited"),
        (MutationOperation::Start, "running"),
        (MutationOperation::Restart, "running"),
    ] {
        let request = confirm(&backend, &scope, &target, operation.clone())
            .await
            .unwrap();
        let repeated = request.clone();
        assert_eq!(
            backend.mutate_container(request).await.unwrap().outcome,
            MutationOutcome::Succeeded
        );
        assert_eq!(
            backend.mutate_container(repeated).await.unwrap_err().code,
            ErrorCode::InvalidIntent
        );
        let actual = inspect(&backend, &scope, &target).await;
        assert_eq!(actual.summary.state, expected);
        if expected == "running" {
            assert_eq!(actual.summary.health.as_deref(), Some("starting"));
            assert_ne!(actual.started_at, before);
        }
        let snapshot = backend
            .list_containers(ListContainersRequest {
                scope: scope.clone(),
            })
            .await
            .unwrap();
        assert_eq!(
            snapshot
                .containers
                .iter()
                .find(|row| row.id == target)
                .unwrap()
                .state,
            expected
        );
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        if inspect(&backend, &scope, &target)
            .await
            .summary
            .health
            .as_deref()
            == Some("healthy")
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Owned healthcheck did not become healthy"
        );
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    std::fs::write(
        control.join("hold-mutation"),
        "hold only owned mutation response",
    )
    .unwrap();
    let request = confirm(&backend, &scope, &target, MutationOperation::Restart)
        .await
        .unwrap();
    let replay = request.clone();
    let mutation = backend.mutate_container(request);
    tokio::pin!(mutation);
    tokio::select! {
        result = &mut mutation => panic!("Mutation returned before controlled disconnect: {result:?}"),
        () = async {
            let deadline = std::time::Instant::now() + Duration::from_secs(15);
            while !control.join("mutation-dispatched").exists() {
                assert!(std::time::Instant::now() < deadline);
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        } => ()
    }
    backend
        .disconnect_inventory_host(InventoryDisconnectRequest {
            mode: WorkspaceMode::Live,
            host_id: host_id.clone(),
            token: connected.token,
        })
        .await
        .unwrap();
    assert_eq!(mutation.await.unwrap().outcome, MutationOutcome::Unknown);
    std::fs::remove_file(control.join("hold-mutation")).unwrap();
    assert_eq!(
        backend.activity_records().unwrap().last().unwrap().outcome,
        crate::activity::ActivityOutcome::Unknown
    );
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: host_id.clone(),
        })
        .await
        .unwrap();
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
    assert!(backend.mutate_container(replay).await.is_err());
    assert_eq!(
        confirm(&backend, &next, &target, MutationOperation::Restart)
            .await
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    );
    assert_eq!(
        inspect(&backend, &next, &target).await.summary.state,
        "running"
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        std::fs::read_to_string(control.join("mutation-count"))
            .unwrap()
            .lines()
            .count(),
        baseline + 4
    );
    backend
        .set_management(SetManagementRequest {
            scope: next.clone(),
            enabled: true,
        })
        .unwrap();
    let drift_request = confirm(&backend, &next, &target, MutationOperation::Stop)
        .await
        .unwrap();
    std::fs::write(
        control.join("mutation-identity-drift"),
        "controlled probe mismatch, not an Engine change",
    )
    .unwrap();
    let drift_result = backend.mutate_container(drift_request).await.unwrap();
    assert_eq!(drift_result.outcome, MutationOutcome::Failed);
    assert_eq!(drift_result.results[0].error, Some(ErrorCode::StaleSession));
    assert!(!drift_result.results[0].dispatched);
    assert!(backend.require_session(&next).is_err());
    assert_eq!(
        std::fs::read_to_string(control.join("mutation-count"))
            .unwrap()
            .lines()
            .count(),
        baseline + 4
    );
    std::fs::remove_file(control.join("mutation-identity-drift")).unwrap();
    println!(
        "PASS controlled daemon identity drift after confirmation: native SSH probe mismatch invalidated the session before mutation dispatch; command count unchanged."
    );
    backend.shutdown().await;
    println!(
        "PASS native lifecycle through ProxyJump: explicit opt-in and owned full IDs, stop/start/restart states, changed StartedAt, health starting then healthy; response loss after real restart recorded Unknown; reconnect stayed read-only and exact mutation count remained four with no replay; client PATH=/nonexistent."
    );
}

#[tokio::test]
#[ignore = "requires explicit owned 033 batch/removal SSH lab"]
async fn checkpoint033_owned_batch_partial_cancel_and_stopped_removal() {
    use std::time::Duration;
    let manifest = std::env::var("CONTAINERDESK_LOG_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let control = std::path::Path::new(&manifest).parent().unwrap();
    let root = control.join("batch-app-data");
    let backend = Backend::new(&root, "/unused-lab-home".into());
    let mut host = draft("logs-owned", lab["config"].as_str().unwrap());
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
    let live = ContainerId(lab["liveId"].as_str().unwrap().into());
    let gone = ContainerId(lab["containerId"].as_str().unwrap().into());
    let denied = ContainerId(lab["unsupportedId"].as_str().unwrap().into());
    async fn confirm(
        backend: &Backend,
        scope: &SessionScope,
        ids: &[ContainerId],
        operation: MutationOperation,
    ) -> Result<MutationRequest, AppError> {
        let spec = MutationSpec {
            operation,
            container_ids: ids.to_vec(),
            timeout_seconds: 1,
        };
        let intent = backend
            .prepare_confirmation(PrepareConfirmationRequest {
                scope: scope.clone(),
                operation: ConfirmationOperation::Mutation(spec.clone()),
            })
            .await?;
        Ok(MutationRequest {
            scope: scope.clone(),
            intent_id: intent.id,
            spec,
        })
    }
    let count = || {
        std::fs::read_to_string(control.join("mutation-count"))
            .unwrap_or_default()
            .lines()
            .count()
    };
    let baseline = count();
    assert_eq!(
        confirm(
            &backend,
            &scope,
            std::slice::from_ref(&live),
            MutationOperation::Remove
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::ContainerNotStopped
    );
    assert_eq!(count(), baseline);
    let partial = confirm(
        &backend,
        &scope,
        &[live.clone(), gone.clone(), denied.clone()],
        MutationOperation::Stop,
    )
    .await
    .unwrap();
    let partial_id = partial.intent_id.clone();
    std::fs::write(control.join("event-delete"), "owned stopped fixture only").unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        while !control.join("event-delete-done").exists() {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    std::fs::write(
        control.join("deny-batch-target"),
        "controlled permission denial",
    )
    .unwrap();
    let response = backend.mutate_container(partial.clone()).await.unwrap();
    assert_eq!(response.outcome, MutationOutcome::Partial);
    assert_eq!(
        response.results[0].outcome,
        MutationTargetOutcome::Succeeded
    );
    assert!(response.results[0].dispatched);
    assert_eq!(
        response.results[1].error,
        Some(ErrorCode::ContainerNotFound)
    );
    assert!(!response.results[1].dispatched);
    assert_eq!(response.results[2].error, Some(ErrorCode::PermissionDenied));
    assert!(response.results[2].dispatched);
    assert_eq!(count(), baseline + 2);
    assert!(backend.mutate_container(partial).await.is_err());
    std::fs::remove_file(control.join("deny-batch-target")).unwrap();
    {
        let pending = confirm(
            &backend,
            &scope,
            &[live.clone(), denied.clone()],
            MutationOperation::Start,
        )
        .await
        .unwrap();
        let cancel = CancelMutationRequest {
            scope: scope.clone(),
            intent_id: pending.intent_id.clone(),
        };
        std::fs::write(
            control.join("hold-mutation"),
            "hold owned response after action",
        )
        .unwrap();
        let dispatched = backend.mutate_container(pending);
        tokio::pin!(dispatched);
        tokio::select! {
            result = &mut dispatched => panic!("batch completed before controlled cancellation: {result:?}"),
            result = tokio::time::timeout(Duration::from_secs(20), async {while !control.join("mutation-dispatched").exists() {tokio::time::sleep(Duration::from_millis(25)).await;}}) => result.unwrap(),
        }
        let mut foreign = cancel.clone();
        foreign.scope.session_generation += 1;
        assert!(
            !backend
                .cancel_mutation(foreign)
                .unwrap()
                .pending_cancellation_requested
        );
        assert!(
            backend
                .cancel_mutation(cancel)
                .unwrap()
                .pending_cancellation_requested
        );
        std::fs::remove_file(control.join("hold-mutation")).unwrap();
        let response = dispatched.await.unwrap();
        assert_eq!(
            response.results[0].outcome,
            MutationTargetOutcome::Succeeded
        );
        assert!(response.results[0].dispatched);
        assert_eq!(
            response.results[1].outcome,
            MutationTargetOutcome::Cancelled
        );
        assert!(!response.results[1].dispatched);
        assert_eq!(count(), baseline + 3);
    }
    // The exact target becomes running after removal confirmation: application recheck refuses it.
    let stop = confirm(
        &backend,
        &scope,
        std::slice::from_ref(&live),
        MutationOperation::Stop,
    )
    .await
    .unwrap();
    assert_eq!(
        backend.mutate_container(stop).await.unwrap().outcome,
        MutationOutcome::Succeeded
    );
    let removal = confirm(
        &backend,
        &scope,
        std::slice::from_ref(&live),
        MutationOperation::Remove,
    )
    .await
    .unwrap();
    let start = confirm(
        &backend,
        &scope,
        std::slice::from_ref(&live),
        MutationOperation::Start,
    )
    .await
    .unwrap();
    assert_eq!(
        backend.mutate_container(start).await.unwrap().outcome,
        MutationOutcome::Succeeded
    );
    let before_refusal = count();
    let refused = backend.mutate_container(removal).await.unwrap();
    assert_eq!(
        refused.results[0].error,
        Some(ErrorCode::ContainerNotStopped)
    );
    assert!(!refused.results[0].dispatched);
    assert_eq!(count(), before_refusal);
    let stop = confirm(
        &backend,
        &scope,
        std::slice::from_ref(&live),
        MutationOperation::Stop,
    )
    .await
    .unwrap();
    assert_eq!(
        backend.mutate_container(stop).await.unwrap().outcome,
        MutationOutcome::Succeeded
    );
    let removal = confirm(
        &backend,
        &scope,
        &[live.clone(), denied.clone()],
        MutationOperation::Remove,
    )
    .await
    .unwrap();
    let removed = backend.mutate_container(removal).await.unwrap();
    assert_eq!(removed.outcome, MutationOutcome::Succeeded);
    assert!(removed.results.iter().all(|r| r.dispatched));
    assert!(
        backend
            .list_containers(ListContainersRequest {
                scope: scope.clone()
            })
            .await
            .unwrap()
            .containers
            .is_empty()
    );
    let records = backend.activity_records().unwrap();
    assert_eq!(
        records.iter().find(|r| r.id == partial_id).unwrap().outcome,
        crate::activity::ActivityOutcome::Partial
    );
    backend.shutdown().await;
    drop(backend);
    let reopened = Backend::new(&root, "/unused-lab-home".into());
    assert_eq!(reopened.activity_records().unwrap(), records);
    reopened.shutdown().await;
    assert_eq!(count(), baseline + 8);
    println!(
        "PASS native batch over strict ProxyJump: success, actual disappearance, controlled permission denial individually retained; pending cancellation preserved dispatched success and sent no second command; running removal refused at preparation and again after a real state race; two stopped removals succeeded without force/volume flags; durable partial history survived reopen; exact eight commands, no replay, PATH=/nonexistent."
    );
}

#[tokio::test]
#[ignore = "requires explicitly owned isolated Engine with image fixtures and SSH manifest"]
async fn checkpoint034_owned_images_dangling_tags_metadata_and_exact_references() {
    let manifest = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest).parent().unwrap();
    let oracle = &lab["imageOracle"];
    let mut previous_scope = None;
    for (index, alias) in ["direct-known", "via-known"].into_iter().enumerate() {
        let data = root.join(format!("images-{index}"));
        let backend = Backend::new(&data, "/unused-lab-home".into());
        let mut host = draft(alias, lab["config"].as_str().unwrap());
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
        let id = saved.saved.preferences.hosts[0].id.clone();
        backend
            .connect_inventory_host(InventoryConnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id.clone(),
            })
            .await
            .unwrap();
        ready(&backend, WorkspaceMode::Live, id.clone()).await;
        let scope = backend
            .connect_host(ConnectHostRequest {
                selection: HostSelection {
                    host_id: id,
                    selection_generation: 1,
                },
            })
            .unwrap()
            .scope;
        if let Some(foreign) = previous_scope.take() {
            assert!(
                backend
                    .list_images(ListImagesRequest {
                        scope: foreign,
                        dangling_only: false
                    })
                    .await
                    .is_err()
            );
        }
        let inventory = backend
            .list_images(ListImagesRequest {
                scope: scope.clone(),
                dangling_only: false,
            })
            .await
            .unwrap();
        assert_eq!(inventory.images.len(), 2);
        let image = inventory
            .images
            .iter()
            .find(|image| image.id.0 == oracle["id"].as_str().unwrap())
            .unwrap();
        assert_eq!(serde_json::to_value(&image.tags).unwrap(), oracle["tags"]);
        let dangling = backend
            .list_images(ListImagesRequest {
                scope: scope.clone(),
                dangling_only: true,
            })
            .await
            .unwrap();
        assert_eq!(dangling.images.len(), 1);
        assert!(dangling.images[0].tags.is_empty());
        assert_eq!(
            dangling.images[0].id.0,
            oracle["danglingId"].as_str().unwrap()
        );
        let detail = backend
            .inspect_image(InspectImageRequest {
                scope: scope.clone(),
                image_id: image.id.clone(),
            })
            .await
            .unwrap();
        assert_eq!(detail.size_bytes, oracle["size"].as_u64());
        assert_eq!(detail.labels.len(), 2);
        assert!(
            detail
                .labels
                .iter()
                .all(|label| label.value.is_none() && label.masked)
        );
        assert_eq!(
            detail
                .containers
                .iter()
                .map(|row| row.container_id.0.clone())
                .collect::<Vec<_>>(),
            oracle["references"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        );
        let encoded = serde_json::to_string(&detail).unwrap();
        assert!(
            !encoded.contains("synthetic-image-034-secret")
                && !encoded.contains("synthetic-label-034-secret")
        );
        assert_eq!(
            backend
                .inspect_image(InspectImageRequest {
                    scope: scope.clone(),
                    image_id: ImageId(format!("sha256:{}", "f".repeat(64)))
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::ImageNotFound
        );
        let untagged = backend
            .inspect_image(InspectImageRequest {
                scope: scope.clone(),
                image_id: dangling.images[0].id.clone(),
            })
            .await
            .unwrap();
        assert!(untagged.tags.is_empty() && untagged.containers.is_empty());
        assert!(!backend.management_state(scope.clone()).unwrap().enabled);
        previous_scope = Some(scope);
        backend.shutdown().await;
        for file in [
            data.join("activity/history.json"),
            data.join("preferences/settings.json"),
        ] {
            if let Ok(bytes) = std::fs::read(file) {
                assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-image-034-secret"));
            }
        }
    }
    println!(
        "PASS native images direct and ProxyJump: isolated Engine oracle matched two immutable identities, multiple tags deduplicated, real dangling filter, metadata size and exact two container references; all label values masked, missing image typed, foreign host scope rejected, read-only permission unchanged; PATH=/nonexistent."
    );
}

#[tokio::test]
#[ignore = "requires explicitly owned isolated Engine volume fixtures and SSH manifest"]
async fn checkpoint035_owned_volumes_metadata_mounts_and_disappearing_container() {
    let manifest = std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest).parent().unwrap();
    let oracle = &lab["volumeOracle"];
    let mut previous_scope = None;
    for (index, alias) in ["direct-known", "via-known"].into_iter().enumerate() {
        let data = root.join(format!("volumes-{index}"));
        let backend = Backend::new(&data, "/unused-lab-home".into());
        let mut host = draft(alias, lab["config"].as_str().unwrap());
        host.docker.executable = Some("/opt/fixture/docker-volume-read".into());
        let saved = backend
            .save_host(SaveHostRequest {
                mode: WorkspaceMode::Live,
                expected_revision: 0,
                id: None,
                draft: host,
            })
            .await
            .unwrap();
        let id = saved.saved.preferences.hosts[0].id.clone();
        backend
            .connect_inventory_host(InventoryConnectRequest {
                mode: WorkspaceMode::Live,
                host_id: id.clone(),
            })
            .await
            .unwrap();
        ready(&backend, WorkspaceMode::Live, id.clone()).await;
        let scope = backend
            .connect_host(ConnectHostRequest {
                selection: HostSelection {
                    host_id: id,
                    selection_generation: 1,
                },
            })
            .unwrap()
            .scope;
        if let Some(foreign) = previous_scope.take() {
            assert!(
                backend
                    .list_volumes(ListVolumesRequest { scope: foreign })
                    .await
                    .is_err()
            );
        }
        let inventory = backend
            .list_volumes(ListVolumesRequest {
                scope: scope.clone(),
            })
            .await
            .unwrap();
        assert_eq!(inventory.volumes.len(), 3);
        for key in ["named", "unused", "anonymous"] {
            assert!(
                inventory
                    .volumes
                    .iter()
                    .any(|row| row.name.0 == oracle[key].as_str().unwrap())
            );
        }
        let request = InspectVolumeRequest {
            scope: scope.clone(),
            name: VolumeName(oracle["named"].as_str().unwrap().into()),
        };
        let detail = backend.inspect_volume(request.clone()).await.unwrap();
        assert_eq!(
            detail.mountpoint_reported.as_deref(),
            oracle["mountpoint"].as_str()
        );
        assert_eq!(detail.references.len(), 2);
        assert_eq!(
            serde_json::to_value(
                detail
                    .references
                    .iter()
                    .map(|row| &row.container_id)
                    .collect::<Vec<_>>()
            )
            .unwrap(),
            oracle["references"]
        );
        assert!(
            detail
                .references
                .iter()
                .all(|row| row.destination.as_deref() == Some("/data"))
        );
        assert!(
            detail
                .references
                .iter()
                .any(|row| row.read_only == Some(true))
        );
        assert!(
            detail
                .references
                .iter()
                .any(|row| row.read_only == Some(false))
        );
        assert_eq!(detail.labels.len(), 1);
        assert!(detail.labels[0].masked && detail.labels[0].value.is_none());
        assert!(
            !serde_json::to_string(&detail)
                .unwrap()
                .contains("synthetic-volume-035-secret")
        );
        if index == 0 {
            assert_eq!(
                detail.reference_observation,
                ReferenceObservation::Incomplete
            );
            assert_eq!(
                detail.unresolved_container_ids,
                vec![ContainerId(oracle["raceId"].as_str().unwrap().into())]
            );
        } else {
            assert_eq!(
                detail.reference_observation,
                ReferenceObservation::Referenced
            );
        }
        let reconciled = backend.inspect_volume(request).await.unwrap();
        assert_eq!(
            reconciled.reference_observation,
            ReferenceObservation::Referenced
        );
        assert!(reconciled.unresolved_container_ids.is_empty());
        for key in ["anonymous", "unused"] {
            let detail = backend
                .inspect_volume(InspectVolumeRequest {
                    scope: scope.clone(),
                    name: VolumeName(oracle[key].as_str().unwrap().into()),
                })
                .await
                .unwrap();
            if key == "anonymous" {
                assert_eq!(
                    detail.reference_observation,
                    ReferenceObservation::Referenced
                );
                assert_eq!(detail.references.len(), 1);
                assert_eq!(detail.references[0].destination.as_deref(), Some("/cache"));
            } else {
                assert_eq!(
                    detail.reference_observation,
                    ReferenceObservation::Unreferenced
                );
                assert!(detail.references.is_empty());
                assert_eq!(detail.options.len(), 3);
                assert!(
                    detail
                        .options
                        .iter()
                        .all(|row| row.masked && row.value.is_none())
                );
            }
        }
        assert_eq!(
            backend
                .inspect_volume(InspectVolumeRequest {
                    scope: scope.clone(),
                    name: VolumeName("missing-checkpoint-volume".into())
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::VolumeNotFound
        );
        assert!(!backend.management_state(scope.clone()).unwrap().enabled);
        previous_scope = Some(scope);
        backend.shutdown().await;
        for file in [
            data.join("activity/history.json"),
            data.join("preferences/settings.json"),
        ] {
            if let Ok(bytes) = std::fs::read(file) {
                assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-volume-035-secret"));
            }
        }
    }
    println!(
        "PASS native volumes direct and ProxyJump: named/anonymous/unused metadata matched isolated Engine oracle; actual container deletion during inspect retained two correct mount references and marked incomplete, reread reconciled; RO/RW mounts, masked labels/options, typed missing volume, old scope rejection, unchanged read-only management; PATH=/nonexistent."
    );
}

#[tokio::test]
#[ignore = "requires explicitly owned dual-stack network and strict SSH log lab"]
async fn checkpoint036_owned_networks_match_real_attachment_oracle() {
    let manifest = std::env::var("CONTAINERDESK_LOG_LAB_MANIFEST").unwrap();
    let lab: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let root = std::path::Path::new(&manifest).parent().unwrap();
    let backend = Backend::new(&root.join("network-app-data"), "/unused-lab-home".into());
    let mut host = draft("logs-owned", lab["config"].as_str().unwrap());
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
    let id = saved.saved.preferences.hosts[0].id.clone();
    backend
        .connect_inventory_host(InventoryConnectRequest {
            mode: WorkspaceMode::Live,
            host_id: id.clone(),
        })
        .await
        .unwrap();
    ready(&backend, WorkspaceMode::Live, id.clone()).await;
    let scope = backend
        .connect_host(ConnectHostRequest {
            selection: HostSelection {
                host_id: id,
                selection_generation: 1,
            },
        })
        .unwrap()
        .scope;
    let oracle = &lab["networkOracle"];
    let list = backend
        .list_networks(ListNetworksRequest {
            scope: scope.clone(),
        })
        .await
        .unwrap();
    assert_eq!(list.networks.len(), 1);
    let row = &list.networks[0];
    assert_eq!(row.id.0, oracle["Id"].as_str().unwrap());
    assert_eq!(row.name, oracle["Name"].as_str().unwrap());
    assert_eq!(row.internal, Some(true));
    assert_eq!(row.ipv6, Some(true));
    let detail = backend
        .inspect_network(InspectNetworkRequest {
            scope: scope.clone(),
            network_id: row.id.clone(),
        })
        .await
        .unwrap();
    assert!(detail.attachments_reported && !detail.metadata_incomplete);
    let expected = oracle["Containers"].as_object().unwrap();
    assert_eq!(detail.attachments.len(), expected.len());
    assert_eq!(detail.attachments.len(), 1);
    for attachment in &detail.attachments {
        let raw = &expected[&attachment.endpoint_key];
        assert_eq!(
            attachment.container_id.as_ref().unwrap().0,
            lab["liveId"].as_str().unwrap()
        );
        assert_eq!(
            attachment.ipv4_address.as_deref(),
            raw["IPv4Address"].as_str()
        );
        assert_eq!(
            attachment.ipv6_address.as_deref(),
            raw["IPv6Address"].as_str()
        );
        assert!(attachment.ipv4_address.is_some() && attachment.ipv6_address.is_some());
    }
    let configs = oracle["IPAM"]["Config"].as_array().unwrap();
    assert_eq!(detail.ipam_config.len(), configs.len());
    for (entry, raw) in detail.ipam_config.iter().zip(configs) {
        assert_eq!(entry.subnet.as_deref(), raw["Subnet"].as_str());
        assert_eq!(entry.gateway.as_deref(), raw["Gateway"].as_str());
    }
    assert!(
        detail
            .labels
            .iter()
            .chain(&detail.options)
            .all(|row| row.masked && row.value.is_none())
    );
    assert!(
        !serde_json::to_string(&detail)
            .unwrap()
            .contains("synthetic-network-036-secret")
    );
    assert!(!backend.management_state(scope.clone()).unwrap().enabled);
    let mut foreign = scope.clone();
    foreign.daemon_id = "other-network-daemon".into();
    assert!(
        backend
            .list_networks(ListNetworksRequest { scope: foreign })
            .await
            .is_err()
    );
    backend.shutdown().await;
    assert!(
        backend
            .list_networks(ListNetworksRequest { scope })
            .await
            .is_err()
    );
    println!(
        "PASS actual network over strict ProxyJump: owned internal dual-stack bridge, full network identity, exact nonzero attachment count and container identity, both IP families/IPAM matched independent Docker inspect; options/labels masked, foreign/disconnected scope refused, management stayed read-only; PATH=/nonexistent."
    );
}
