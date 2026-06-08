use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

const OPEN_SETTINGS_ID: &str = "open_settings";
const QUIT_ID: &str = "quit";

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    let open_settings =
        MenuItem::with_id(app, OPEN_SETTINGS_ID, "Open Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit Shanka", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_settings, &quit])?;
    let icon = app.default_window_icon().cloned();

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
