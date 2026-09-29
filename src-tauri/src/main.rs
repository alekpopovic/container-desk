fn main() {
    // Tauri/Wry otherwise allows WebKit automation in a release build when this
    // environment variable is set. Only explicitly instrumented lab builds opt in.
    #[cfg(not(feature = "native-automation"))]
    {
        // SAFETY: this executable is still single-threaded, before Tauri/GTK,
        // Tokio or any application worker is initialized. Keep this at entry.
        unsafe {
            for name in [
                "TAURI_WEBVIEW_AUTOMATION",
                "TAURI_AUTOMATION",
                "TAURI_WEBDRIVER_PORT",
                "WEBKIT_INSPECTOR_SERVER",
                "WEBKIT_INSPECTOR_HTTP_SERVER",
            ] {
                std::env::remove_var(name);
            }
        }
    }
    containerdesk_lib::run();
}
