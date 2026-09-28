mod diagnostics;
pub mod docker;
pub mod policy;
pub mod ssh;
mod storage;
pub mod transport;
use tauri::Manager;
mod backend;
mod commands;
#[cfg(test)]
mod contract_tests;
pub mod domain;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(backend::Backend::new(
                &app.path().app_data_dir()?,
                app.path().home_dir()?,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::list_hosts,
            commands::connect_host,
            commands::list_containers,
            commands::cancel_subscription,
            commands::get_preferences,
            commands::set_theme,
            commands::dependency_diagnostics,
            commands::set_ssh_executable,
            commands::inspect_container,
            commands::container_logs,
            commands::prepare_confirmation,
            commands::mutate_container,
            commands::open_container_terminal,
            commands::get_workspace_mode,
            commands::get_ssh_config_path,
            commands::discover_ssh_hosts,
            commands::select_ssh_alias,
            commands::resolve_ssh_config,
            commands::check_ssh_access,
            commands::switch_workspace
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ContainerDesk");
}

#[cfg(test)]
fn test_directory_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
