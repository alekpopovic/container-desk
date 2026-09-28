mod backend;
mod commands;
#[cfg(test)]
mod contract_tests;
pub mod domain;

pub fn run() {
    tauri::Builder::default()
        .manage(backend::Backend::default())
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::list_hosts,
            commands::connect_host,
            commands::list_containers,
            commands::cancel_subscription
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ContainerDesk");
}
