mod bridge;
mod clipboard;
mod hotkey;
mod input;
mod platform;
mod tray;
mod window;

#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            platform::setup(handle)?;
            window::setup(handle)?;
            tray::setup(handle)?;
            hotkey::setup(handle)?;
            clipboard::setup(handle)?;
            input::setup(handle)?;
            bridge::setup(handle)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
