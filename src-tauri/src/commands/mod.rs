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
pub fn list_containers(
    backend: tauri::State<'_, Backend>,
    request: ListContainersRequest,
) -> Result<ListContainersResponse, AppError> {
    backend.list_containers(request)
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
