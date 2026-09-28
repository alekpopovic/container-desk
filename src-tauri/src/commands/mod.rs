use crate::{backend::Backend, domain::*};

#[tauri::command]
pub fn app_version(app: tauri::AppHandle) -> AppVersion {
    AppVersion {
        version: app.package_info().version.to_string(),
    }
}
#[tauri::command]
pub fn list_hosts(backend: tauri::State<'_, Backend>) -> Result<ListHostsResponse, AppError> {
    backend.list_hosts()
}
#[tauri::command]
pub fn connect_host(
    backend: tauri::State<'_, Backend>,
    request: ConnectHostRequest,
) -> Result<ConnectHostResponse, AppError> {
    backend.connect_host(request)
}
#[tauri::command]
pub async fn list_containers(
    backend: tauri::State<'_, Backend>,
    request: ListContainersRequest,
) -> Result<ListContainersResponse, AppError> {
    backend.list_containers(request).await
}
#[tauri::command]
pub fn cancel_subscription(
    backend: tauri::State<'_, Backend>,
    request: CancelSubscriptionRequest,
) -> Result<CancelSubscriptionResponse, AppError> {
    backend.cancel_subscription(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets};

    #[test]
    fn native_ipc_demo_returns_the_same_dtos_and_clears_old_scope_on_exit() {
        let app = mock_builder()
            .manage(Backend::default())
            .invoke_handler(tauri::generate_handler![
                get_host_inventory,
                save_host,
                remove_host,
                get_workspace_mode,
                switch_workspace,
                begin_ssh_session,
                get_ssh_session,
                disconnect_ssh_session,
                list_containers
            ])
            .build(mock_context(noop_assets()))
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let invoke = |command: &str, body: Value| {
            get_ipc_response(
                &window,
                tauri::webview::InvokeRequest {
                    cmd: command.into(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: "tauri://localhost".parse().unwrap(),
                    body: tauri::ipc::InvokeBody::Json(body),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.into(),
                },
            )
            .map(|response| response.deserialize::<Value>().unwrap())
        };
        assert_eq!(
            invoke("get_workspace_mode", json!({})).unwrap()["mode"],
            "live"
        );
        let mode = invoke(
            "switch_workspace",
            json!({"request":{"mode":"demo","scenario":"standard"}}),
        )
        .unwrap();
        assert_eq!(mode["mode"], "demo");
        let added = invoke("save_host", json!({"request":{"mode":"demo","expectedRevision":0,"id":null,"draft":{"ssh":{"alias":"demo-direct","configPath":"/demo/config","useDefaultConfig":false},"docker":{"executable":null,"context":null,"sudo":false},"displayName":"IPC host","group":"dev","labels":[],"favorite":true}}})).unwrap();
        let inventory = invoke("get_host_inventory", json!({"request":{"mode":"demo"}})).unwrap();
        assert_eq!(
            inventory["saved"]["preferences"]["hosts"][0]["favorite"],
            true
        );
        assert!(inventory["connection"].is_null());
        let removed = invoke("remove_host", json!({"request":{"mode":"demo","expectedRevision":1,"hostId":added["saved"]["preferences"]["hosts"][0]["id"]}})).unwrap();
        assert!(
            removed["saved"]["preferences"]["hosts"]
                .as_array()
                .unwrap()
                .is_empty()
        );

        assert_eq!(invoke("begin_ssh_session", json!({"request":{"selection":{"alias":"fixture","configPath":"/fixture/config","useDefaultConfig":false}}})).unwrap_err()["code"], "permission_denied");
        for command in ["get_ssh_session", "disconnect_ssh_session"] {
            assert_eq!(invoke(command, json!({"request":{"token":{"sessionId":format!("s_{}", "1".repeat(32)),"sessionGeneration":1}}})).unwrap_err()["code"], "permission_denied");
        }

        let response = invoke(
            "list_containers",
            json!({"request":{"scope":mode["scope"]}}),
        )
        .unwrap();
        let decoded: ListContainersResponse = serde_json::from_value(response.clone()).unwrap();
        assert_eq!(decoded.containers.len(), 4);
        assert_eq!(
            decoded.containers[0].ports[0].host_ip.as_deref(),
            Some("::1")
        );
        assert!(!response.to_string().contains("SYNTHETIC_VALUE_NOT_FOR_IPC"));
        assert_eq!(
            invoke("switch_workspace", json!({"request":{"mode":"live"}})).unwrap()["mode"],
            "live"
        );
        assert_eq!(
            invoke(
                "list_containers",
                json!({"request":{"scope":mode["scope"]}})
            )
            .unwrap_err()["code"],
            "session_not_found"
        );
    }

    #[test]
    fn direct_ipc_mutation_is_denied_in_read_only_and_bad_arguments_cannot_dispatch() {
        let backend = Backend::default();
        backend.register_test_session(crate::contract_tests::scope());
        let app = mock_builder()
            .manage(backend)
            .invoke_handler(tauri::generate_handler![
                list_containers,
                inspect_container,
                container_logs,
                prepare_confirmation,
                mutate_container,
                open_container_terminal
            ])
            .build(mock_context(noop_assets()))
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let invoke = |command: &str, request: Value| {
            get_ipc_response(
                &window,
                tauri::webview::InvokeRequest {
                    cmd: command.into(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: "tauri://localhost".parse().unwrap(),
                    body: tauri::ipc::InvokeBody::Json(json!({"request":request})),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.into(),
                },
            )
            .map(|response| response.deserialize::<Value>().unwrap())
        };
        let scope = crate::contract_tests::scope();
        let spec =
            json!({"operation":"restart","containerIds":["a".repeat(64)],"timeoutSeconds":10});
        let mut request =
            json!({"scope":scope,"intentId":format!("i_{}", "1".repeat(32)),"spec":spec});
        assert_eq!(
            invoke("mutate_container", request.clone()).unwrap_err()["code"],
            "permission_denied"
        );
        assert_eq!(
            invoke("list_containers", json!({"scope":scope})).unwrap_err()["code"],
            "session_not_found",
            "policy registration alone cannot invent an owned native session"
        );
        request["spec"]["force"] = true.into();
        assert!(invoke("mutate_container", request.clone()).is_err());
        request["spec"].as_object_mut().unwrap().remove("force");
        request["spec"]["operation"] = "prune".into();
        assert!(invoke("mutate_container", request.clone()).is_err());
        request["spec"]["operation"] = "restart".into();
        request["spec"]["timeoutSeconds"] = (-1).into();
        assert_eq!(
            invoke("mutate_container", request.clone()).unwrap_err()["code"],
            "invalid_limits"
        );
        request["spec"]["timeoutSeconds"] = 10.into();
        request["spec"]["containerIds"] = json!(["--all"]);
        assert_eq!(
            invoke("mutate_container", request).unwrap_err()["code"],
            "invalid_id"
        );
        assert_eq!(
            invoke(
                "container_logs",
                json!({"scope":scope,"containerId":"a".repeat(64),"tail":-1,"timeoutSeconds":30})
            )
            .unwrap_err()["code"],
            "invalid_limits"
        );
        assert_eq!(invoke("open_container_terminal", json!({"scope":scope,"intentId":format!("i_{}", "1".repeat(32)),"spec":{"containerId":"a".repeat(64),"shell":"sh","columns":80,"rows":24}})).unwrap_err()["code"], "permission_denied");
    }

    #[test]
    fn registered_handlers_return_typed_results_without_a_process_launcher() {
        let app = mock_builder()
            .manage(Backend::default())
            .invoke_handler(tauri::generate_handler![
                list_hosts,
                connect_host,
                list_containers,
                cancel_subscription,
                get_preferences,
                set_theme
            ])
            .build(mock_context(noop_assets()))
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let invoke = |command: &str, body: Value| {
            get_ipc_response(
                &window,
                tauri::webview::InvokeRequest {
                    cmd: command.into(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: "tauri://localhost".parse().unwrap(),
                    body: tauri::ipc::InvokeBody::Json(body),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.into(),
                },
            )
            .map(|response| response.deserialize::<Value>().unwrap())
        };
        assert_eq!(
            invoke("list_hosts", json!({})).unwrap(),
            json!({"hosts": []})
        );
        assert_eq!(
            invoke(
                "connect_host",
                json!({"request":{"selection":{"hostId":"--help","selectionGeneration":1}}})
            )
            .unwrap_err()["code"],
            "invalid_id"
        );
        let error = invoke(
            "list_containers",
            json!({"request":{"scope":crate::contract_tests::scope()}}),
        )
        .unwrap_err();
        let fixture: Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/ipc.json")).unwrap();
        assert_eq!(error, fixture["error"]);
        assert_eq!(
            invoke("get_preferences", json!({})).unwrap(),
            fixture["preferences"]
        );
        let saved = invoke(
            "set_theme",
            json!({"request":{"theme":"dark","expectedRevision":0}}),
        )
        .unwrap();
        assert_eq!(saved["preferences"]["theme"], "dark");
        assert_eq!(saved["preferences"]["revision"], 1);
        assert_eq!(
            invoke(
                "set_theme",
                json!({"request":{"theme":"light","expectedRevision":0}})
            )
            .unwrap_err()["code"],
            "storage_conflict"
        );
        assert_eq!(invoke("cancel_subscription", json!({"request":{"scope":crate::contract_tests::scope(),"subscriptionId":"../../socket"}})).unwrap_err()["code"], "invalid_id");
        assert!(invoke("execute", json!({"command":"anything"})).is_err());
    }
}

#[tauri::command]
pub fn get_preferences(
    backend: tauri::State<'_, Backend>,
) -> Result<PreferencesSnapshot, AppError> {
    backend.preferences()
}
#[tauri::command]
pub fn set_theme(
    backend: tauri::State<'_, Backend>,
    request: SetThemeRequest,
) -> Result<PreferencesSnapshot, AppError> {
    backend.set_theme(request)
}

#[tauri::command]
pub async fn dependency_diagnostics(
    backend: tauri::State<'_, Backend>,
) -> Result<DependencyDiagnostics, AppError> {
    backend.diagnostics().await
}
#[tauri::command]
pub async fn set_ssh_executable(
    backend: tauri::State<'_, Backend>,
    request: SetSshExecutableRequest,
) -> Result<SetSshExecutableResponse, AppError> {
    backend.set_ssh_executable(request).await
}

#[tauri::command]
pub async fn inspect_container(
    backend: tauri::State<'_, Backend>,
    request: InspectContainerRequest,
) -> Result<ContainerDetail, AppError> {
    backend.inspect_container(request).await
}
#[tauri::command]
pub fn container_logs(
    backend: tauri::State<'_, Backend>,
    request: ContainerLogsRequest,
) -> Result<LogSnapshot, AppError> {
    backend.container_logs(request)
}
#[tauri::command]
pub fn prepare_confirmation(
    backend: tauri::State<'_, Backend>,
    request: PrepareConfirmationRequest,
) -> Result<ConfirmationIntent, AppError> {
    backend.prepare_confirmation(request)
}
#[tauri::command]
pub fn mutate_container(
    backend: tauri::State<'_, Backend>,
    request: MutationRequest,
) -> Result<MutationResponse, AppError> {
    backend.mutate_container(request)
}
#[tauri::command]
pub fn open_container_terminal(
    backend: tauri::State<'_, Backend>,
    request: TerminalRequest,
) -> Result<TerminalResponse, AppError> {
    backend.open_container_terminal(request)
}

#[tauri::command]
pub fn get_workspace_mode(
    backend: tauri::State<'_, Backend>,
) -> Result<WorkspaceModeSnapshot, AppError> {
    backend.workspace_mode()
}
#[tauri::command]
pub fn switch_workspace(
    backend: tauri::State<'_, Backend>,
    request: SwitchWorkspaceRequest,
) -> Result<WorkspaceModeSnapshot, AppError> {
    backend.switch_workspace(request)
}

#[tauri::command]
pub fn get_ssh_config_path(backend: tauri::State<'_, Backend>) -> Result<SshConfigPath, AppError> {
    backend.config_path(None)
}
#[tauri::command]
pub async fn discover_ssh_hosts(
    backend: tauri::State<'_, Backend>,
    request: DiscoverHostsRequest,
) -> Result<HostDiscovery, AppError> {
    backend.discover_hosts(request).await
}
#[tauri::command]
pub fn select_ssh_alias(
    backend: tauri::State<'_, Backend>,
    request: SelectSshAliasRequest,
) -> Result<SshSelection, AppError> {
    backend.select_alias(request)
}

#[tauri::command]
pub async fn resolve_ssh_config(
    backend: tauri::State<'_, Backend>,
    request: ResolveSshRequest,
) -> Result<EffectiveSshConfig, AppError> {
    backend.resolve_ssh(request).await
}

#[tauri::command]
pub async fn check_ssh_access(
    backend: tauri::State<'_, Backend>,
    request: ResolveSshRequest,
) -> Result<SshAccessReport, AppError> {
    backend.check_ssh_access(request).await
}

#[tauri::command]
pub async fn begin_ssh_session(
    backend: tauri::State<'_, Backend>,
    request: BeginSshRequest,
) -> Result<ConnectionSnapshot, AppError> {
    backend.begin_ssh_session(request).await
}
#[tauri::command]
pub fn get_ssh_session(
    backend: tauri::State<'_, Backend>,
    request: ConnectionRequest,
) -> Result<ConnectionSnapshot, AppError> {
    backend.ssh_session(request)
}
#[tauri::command]
pub async fn disconnect_ssh_session(
    backend: tauri::State<'_, Backend>,
    request: ConnectionRequest,
) -> Result<ConnectionSnapshot, AppError> {
    backend.disconnect_ssh_session(request).await
}

#[tauri::command]
pub fn get_host_inventory(
    backend: tauri::State<'_, Backend>,
    request: InventoryModeRequest,
) -> Result<HostInventory, AppError> {
    backend.host_inventory(request)
}
#[tauri::command]
pub async fn save_host(
    backend: tauri::State<'_, Backend>,
    request: SaveHostRequest,
) -> Result<HostInventory, AppError> {
    backend.save_host(request).await
}
#[tauri::command]
pub async fn remove_host(
    backend: tauri::State<'_, Backend>,
    request: RemoveHostRequest,
) -> Result<HostInventory, AppError> {
    backend.remove_host(request).await
}
#[tauri::command]
pub async fn connect_inventory_host(
    backend: tauri::State<'_, Backend>,
    request: InventoryConnectRequest,
) -> Result<HostInventory, AppError> {
    backend.connect_inventory_host(request).await
}
#[tauri::command]
pub async fn disconnect_inventory_host(
    backend: tauri::State<'_, Backend>,
    request: InventoryDisconnectRequest,
) -> Result<HostInventory, AppError> {
    backend.disconnect_inventory_host(request).await
}
