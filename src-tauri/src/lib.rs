mod app_icon;
mod autostart;
mod bridge;
mod clipboard;
mod config;
mod history;
mod hotkey;
mod hud;
mod input;
mod keychain;
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
    let mut settings = config::load_settings(&app).map_err(|error| error.to_string())?;
    settings.launch_at_login = autostart::is_enabled(&app).unwrap_or(settings.launch_at_login);
    Ok(settings)
}

#[tauri::command]
fn get_persona_config(app: tauri::AppHandle) -> Result<persona::PersonaConfig, String> {
    config::load_or_create(&app)
        .map(|config| persona::normalize_config(&config.personas))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_platform_status() -> platform::PlatformStatus {
    platform::status()
}

#[tauri::command]
fn open_platform_permission_settings() -> Result<(), String> {
    platform::open_permission_settings()
}

#[tauri::command]
fn get_rewrite_history(app: tauri::AppHandle) -> Result<Vec<history::RewriteHistoryItem>, String> {
    history::load(&app)
}

#[tauri::command]
fn clear_rewrite_history(app: tauri::AppHandle) -> Result<(), String> {
    history::clear(&app)
}

#[tauri::command]
fn copy_history_result(app: tauri::AppHandle, history_id: u64) -> Result<(), String> {
    let item = history::load(&app)?
        .into_iter()
        .find(|item| item.id == history_id)
        .ok_or_else(|| "history item is no longer available".to_string())?;
    clipboard::copy_text_to_clipboard(&item.result_text).map_err(|error| error.to_string())
}

#[tauri::command]
fn copy_last_replacement_original(app: tauri::AppHandle) -> Result<(), String> {
    pipeline::copy_last_replacement_original(&app).map_err(|error| {
        hud::error(&app, error.error_code());
        error.to_string()
    })
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
fn generate_persona_draft(
    app: tauri::AppHandle,
    name: String,
    locale: Option<String>,
) -> Result<rewrite::GeneratedPersonaDraft, String> {
    rewrite::generate_persona_draft(&app, &name, locale.as_deref())
        .map_err(|error| error.to_string())
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
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            println!("[app] existing Shanka instance activated");
            crate::window::show_settings_window(app);
        }))
        .plugin(autostart::plugin())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            platform::setup(handle)?;
            window::setup(handle)?;
            tray::setup(handle)?;
            autostart::setup(handle);
            config::setup(handle)?;
            history::setup(handle)?;
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
            get_platform_status,
            get_rewrite_history,
            open_platform_permission_settings,
            clear_rewrite_history,
            copy_history_result,
            copy_last_replacement_original,
            save_app_settings,
            test_provider_connection,
            generate_persona_draft,
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
