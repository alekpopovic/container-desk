#[derive(serde::Serialize)]
pub struct AppVersion {
    version: String,
}

#[tauri::command]
pub fn app_version(app: tauri::AppHandle) -> AppVersion {
    AppVersion {
        version: app.package_info().version.to_string(),
    }
}
