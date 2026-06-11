use tauri_plugin_autostart::ManagerExt;

pub const FROM_AUTOSTART_ARG: &str = "--from-autostart";

pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_autostart::Builder::new()
        .arg(FROM_AUTOSTART_ARG)
        .build()
}

pub fn setup(app: &tauri::AppHandle) {
    println!("[autostart] launch-at-login controller ready");
    if matches!(is_enabled(app), Ok(true)) {
        if let Err(error) = refresh_enabled_entry(app) {
            println!("[autostart] failed to refresh launch-at-login entry: {error}");
        }
    }
}

pub fn is_enabled(app: &tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("failed to read launch-at-login state: {error}"))
}

pub fn set_enabled(app: &tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    let autolaunch = app.autolaunch();

    if enabled {
        if matches!(autolaunch.is_enabled(), Ok(true)) {
            return Ok(true);
        }

        autolaunch.enable().map_err(|error| {
            format!(
                "failed to {} launch-at-login: {error}",
                if enabled { "enable" } else { "disable" }
            )
        })?;
    } else {
        if let Err(error) = autolaunch.disable() {
            if matches!(autolaunch.is_enabled(), Ok(false)) {
                return Ok(false);
            }

            return Err(format!("failed to disable launch-at-login: {error}"));
        }
    }

    let final_state = autolaunch.is_enabled().unwrap_or(enabled);
    if final_state == enabled {
        Ok(final_state)
    } else {
        Err(format!(
            "failed to {} launch-at-login: system state did not change",
            if enabled { "enable" } else { "disable" }
        ))
    }
}

fn refresh_enabled_entry(app: &tauri::AppHandle) -> Result<(), String> {
    app.autolaunch()
        .enable()
        .map_err(|error| format!("failed to refresh launch-at-login entry: {error}"))
}

pub fn started_from_autostart() -> bool {
    started_from_autostart_args(std::env::args())
}

fn started_from_autostart_args<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .any(|arg| arg.as_ref() == FROM_AUTOSTART_ARG)
}

#[cfg(test)]
mod tests {
    use super::started_from_autostart_args;

    #[test]
    fn started_from_autostart_detects_explicit_flag() {
        assert!(started_from_autostart_args(["shanka", "--from-autostart"]));
    }

    #[test]
    fn started_from_autostart_ignores_normal_launch() {
        assert!(!started_from_autostart_args(["shanka"]));
    }
}
