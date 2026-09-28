mod diagnostics;
pub mod policy;
mod storage;
use tauri::Manager;
mod backend;
mod commands;
#[cfg(test)]
mod contract_tests;
pub mod domain;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(backend::Backend::new(&app.path().app_data_dir()?));
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
            commands::open_container_terminal
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ContainerDesk");
}
