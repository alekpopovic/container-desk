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
            commands::set_theme
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ContainerDesk");
}
