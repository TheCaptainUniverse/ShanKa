use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Wry,
};

const OPEN_SETTINGS_ID: &str = "open_settings";
const QUIT_ID: &str = "quit";
const DEFAULT_LOCALE: &str = "zh-CN";

static TRAY_MENU_ITEMS: std::sync::Mutex<Option<TrayMenuItems>> = std::sync::Mutex::new(None);

struct TrayMenuItems {
    open_settings: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    let labels = menu_labels_for_locale(DEFAULT_LOCALE);
    let open_settings = MenuItem::with_id(
        app,
        OPEN_SETTINGS_ID,
        labels.open_settings,
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_ID, labels.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_settings, &quit])?;
    let icon = crate::app_icon::load()
        .ok()
        .or_else(|| app.default_window_icon().cloned());

    remember_menu_items(open_settings.clone(), quit.clone());

    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_SETTINGS_ID => crate::window::show_settings_window(app),
            QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::window::show_settings_window(tray.app_handle());
            }
        });

    if let Some(icon) = icon {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    println!("[tray] tray controller ready");
    Ok(())
}

pub fn update_locale(locale: &str) -> Result<(), String> {
    let labels = menu_labels_for_locale(locale);
    let items = TRAY_MENU_ITEMS
        .lock()
        .map_err(|error| format!("tray menu lock failed: {error}"))?;
    let Some(items) = items.as_ref() else {
        return Ok(());
    };

    items
        .open_settings
        .set_text(labels.open_settings)
        .map_err(|error| format!("failed to update tray settings menu: {error}"))?;
    items
        .quit
        .set_text(labels.quit)
        .map_err(|error| format!("failed to update tray quit menu: {error}"))?;

    Ok(())
}

fn remember_menu_items(open_settings: MenuItem<Wry>, quit: MenuItem<Wry>) {
    match TRAY_MENU_ITEMS.lock() {
        Ok(mut items) => {
            *items = Some(TrayMenuItems {
                open_settings,
                quit,
            });
        }
        Err(error) => {
            println!("[tray] failed to remember menu items: {error}");
        }
    }
}

#[derive(Clone, Copy)]
struct TrayMenuLabels {
    open_settings: &'static str,
    quit: &'static str,
}

fn menu_labels_for_locale(locale: &str) -> TrayMenuLabels {
    if locale.to_ascii_lowercase().starts_with("en") {
        return TrayMenuLabels {
            open_settings: "Open Settings",
            quit: "Quit Shanka",
        };
    }

    TrayMenuLabels {
        open_settings: "打开设置",
        quit: "退出闪改",
    }
}

#[cfg(test)]
mod tests {
    use super::menu_labels_for_locale;

    #[test]
    fn tray_menu_labels_default_to_chinese() {
        let labels = menu_labels_for_locale("zh-CN");

        assert_eq!(labels.open_settings, "打开设置");
        assert_eq!(labels.quit, "退出闪改");
    }

    #[test]
    fn tray_menu_labels_support_english() {
        let labels = menu_labels_for_locale("en-US");

        assert_eq!(labels.open_settings, "Open Settings");
        assert_eq!(labels.quit, "Quit Shanka");
    }
}
