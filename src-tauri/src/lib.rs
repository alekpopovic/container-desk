mod diagnostics;
pub mod docker;
mod log_export;
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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(backend::Backend::new(
                &app.path().app_data_dir()?,
                app.path().home_dir()?,
            ));
            tauri::async_runtime::spawn(ssh::runtime::recover());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::get_host_inventory,
            commands::save_host,
            commands::remove_host,
            commands::connect_inventory_host,
            commands::disconnect_inventory_host,
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
            commands::container_stats,
            commands::follow_container_logs,
            commands::follow_docker_events,
            commands::ack_docker_events,
            commands::ack_container_logs,
            commands::export_container_logs,
            commands::prepare_confirmation,
            commands::mutate_container,
            commands::open_container_terminal,
            commands::get_workspace_mode,
            commands::get_ssh_config_path,
            commands::discover_ssh_hosts,
            commands::select_ssh_alias,
            commands::resolve_ssh_config,
            commands::check_ssh_access,
            commands::begin_ssh_session,
            commands::get_ssh_session,
            commands::disconnect_ssh_session,
            commands::switch_workspace
        ])
        .build(tauri::generate_context!())
        .expect("failed to build ContainerDesk")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                tauri::async_runtime::block_on(app.state::<backend::Backend>().shutdown());
            }
        });
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
