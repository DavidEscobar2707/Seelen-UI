use tauri::{Builder, Wry};

pub fn register_plugins(app_builder: Builder<Wry>) -> Builder<Wry> {
    app_builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
}
