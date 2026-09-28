fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "app_version",
            "list_hosts",
            "connect_host",
            "list_containers",
            "cancel_subscription",
            "get_preferences",
            "set_theme",
            "dependency_diagnostics",
            "set_ssh_executable",
            "inspect_container",
            "container_logs",
            "prepare_confirmation",
            "mutate_container",
            "open_container_terminal",
        ]),
    ))
    .expect("failed to build ContainerDesk application context");
}
