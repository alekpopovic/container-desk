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
