use super::*;
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
