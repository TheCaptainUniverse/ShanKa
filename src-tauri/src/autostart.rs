use tauri_plugin_autostart::ManagerExt;

pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_autostart::Builder::new().build()
}

pub fn setup(_app: &tauri::AppHandle) {
    println!("[autostart] launch-at-login controller ready");
}

pub fn is_enabled(app: &tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("failed to read launch-at-login state: {error}"))
}

pub fn set_enabled(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };

    result.map_err(|error| {
        format!(
            "failed to {} launch-at-login: {error}",
            if enabled { "enable" } else { "disable" }
        )
    })
}
