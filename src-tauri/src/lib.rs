mod bridge;
mod clipboard;
mod config;
mod hotkey;
mod hud;
mod input;
mod persona;
mod pipeline;
mod platform;
mod rewrite;
mod selection;
mod tray;
mod window;

#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
fn get_hud_state() -> hud::HudUpdate {
    hud::current()
}

#[tauri::command]
fn get_hotkey_config(app: tauri::AppHandle) -> Result<config::HotkeyConfig, String> {
    config::load_or_create(&app)
        .map(|config| config.hotkeys)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_app_settings(app: tauri::AppHandle) -> Result<config::AppSettingsConfig, String> {
    config::load_or_create(&app)
        .map(|config| config.settings)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_persona_config(app: tauri::AppHandle) -> Result<persona::PersonaConfig, String> {
    config::load_or_create(&app)
        .map(|config| persona::normalize_config(&config.personas))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_hotkey_config(
    app: tauri::AppHandle,
    hotkeys: config::HotkeyConfig,
) -> Result<config::HotkeyConfig, String> {
    let resolved = hotkeys.resolve().map_err(|error| error.to_string())?;
    hotkey::reload(&app, resolved)?;
    config::save_hotkeys(&app, hotkeys).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_app_settings(
    app: tauri::AppHandle,
    settings: config::AppSettingsConfig,
) -> Result<config::AppSettingsConfig, String> {
    config::save_settings(&app, settings).map_err(|error| error.to_string())
}

#[tauri::command]
fn test_provider_connection(settings: config::AppSettingsConfig) -> Result<(), String> {
    rewrite::test_provider_connection(settings).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_persona_config(
    app: tauri::AppHandle,
    personas: persona::PersonaConfig,
) -> Result<persona::PersonaConfig, String> {
    config::save_personas(&app, personas).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_hotkey_recording_active(active: bool) {
    hotkey::set_recording_active(active);
}

#[tauri::command]
fn copy_safe_preview(
    app: tauri::AppHandle,
    preview_id: u64,
    edited_text: Option<String>,
) -> Result<(), String> {
    pipeline::copy_safe_preview(&app, preview_id, edited_text).map_err(|error| {
        hud::error(&app, error.error_code());
        error.to_string()
    })
}

#[tauri::command]
fn replace_safe_preview(
    app: tauri::AppHandle,
    preview_id: u64,
    edited_text: Option<String>,
) -> Result<(), String> {
    pipeline::replace_safe_preview(&app, preview_id, edited_text).map_err(|error| {
        hud::error(&app, error.error_code());
        error.to_string()
    })
}

#[tauri::command]
fn regenerate_safe_preview(app: tauri::AppHandle, preview_id: u64, persona_id: Option<String>) {
    pipeline::regenerate_safe_preview(app, preview_id, persona_id);
}

#[tauri::command]
fn dismiss_safe_preview(app: tauri::AppHandle, preview_id: Option<u64>) {
    pipeline::dismiss_safe_preview(&app, preview_id);
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
            config::setup(handle)?;
            clipboard::setup(handle)?;
            input::setup(handle)?;
            selection::setup(handle)?;
            rewrite::setup(handle)?;
            hotkey::setup(handle)?;
            bridge::setup(handle)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_version,
            get_hud_state,
            get_app_settings,
            get_hotkey_config,
            get_persona_config,
            save_app_settings,
            test_provider_connection,
            save_hotkey_config,
            save_persona_config,
            set_hotkey_recording_active,
            copy_safe_preview,
            replace_safe_preview,
            regenerate_safe_preview,
            dismiss_safe_preview
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
