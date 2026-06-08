use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    app.plugin(tauri_plugin_autostart::init(
        MacosLauncher::LaunchAgent,
        None,
    ))?;
    println!("[autostart] launch-at-login controller ready");
    Ok(())
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
