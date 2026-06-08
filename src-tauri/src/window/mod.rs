use std::{
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const MAIN_WINDOW_LABEL: &str = "main";
const HUD_WINDOW_LABEL: &str = "hud";
const HUD_HIDE_DELAY: Duration = Duration::from_millis(2_200);

static HUD_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
struct HudLayout {
    width: f64,
    height: f64,
    vertical_offset: f64,
    interactive: bool,
}

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    install_settings_window_close_handler(app);
    create_hud_window(app)?;
    println!("[window] settings and HUD window manager ready");
    Ok(())
}

pub fn present_hud(app: &tauri::AppHandle, status: &'static str) {
    let generation = HUD_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    let layout = layout_for_status(status);

    if layout.interactive {
        crate::platform::remember_preview_target_window();
    }

    if let Some(window) = app.get_webview_window(HUD_WINDOW_LABEL) {
        if let Err(error) = window.set_focusable(layout.interactive) {
            println!("[window] failed to update HUD focusable state: {error}");
        }

        if let Err(error) = window.set_size(tauri::PhysicalSize::new(
            layout.width as u32,
            layout.height as u32,
        )) {
            println!("[window] failed to resize HUD window: {error}");
        }

        if let Err(error) = window.set_ignore_cursor_events(!layout.interactive) {
            println!("[window] failed to update HUD click-through: {error}");
        }

        if let Err(error) = position_hud_window(app, &window, layout) {
            println!("[window] failed to position HUD window: {error}");
        }

        if let Err(error) = window.show() {
            println!("[window] failed to show HUD window: {error}");
        }

        if layout.interactive {
            if let Err(error) = window.set_focus() {
                println!("[window] failed to focus HUD window: {error}");
            }
        }
    }

    if should_auto_hide(status) {
        schedule_hud_hide(app.clone(), generation);
    }
}

pub fn hide_hud(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(HUD_WINDOW_LABEL) {
        if let Err(error) = window.hide() {
            println!("[window] failed to hide HUD window: {error}");
        }
        if let Err(error) = window.set_focusable(false) {
            println!("[window] failed to reset HUD focusable state: {error}");
        }
    }
}

pub fn show_settings_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    if let Err(error) = window.show() {
        println!("[window] failed to show settings window: {error}");
    }
    if let Err(error) = window.set_focus() {
        println!("[window] failed to focus settings window: {error}");
    }
}

fn install_settings_window_close_handler(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        println!("[window] settings window not found; close-to-tray handler not installed");
        return;
    };
    let window_for_event = window.clone();

    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Err(error) = window_for_event.hide() {
                println!("[window] failed to hide settings window on close request: {error}");
            }
        }
    });
}

fn create_hud_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(HUD_WINDOW_LABEL).is_some() {
        return Ok(());
    }

    let window =
        WebviewWindowBuilder::new(app, HUD_WINDOW_LABEL, WebviewUrl::App("index.html".into()))
            .title("Shanka HUD")
            .inner_size(180.0, 52.0)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focusable(false)
            .focused(false)
            .visible(false)
            .build()?;

    if let Err(error) = window.set_ignore_cursor_events(true) {
        println!("[window] failed to enable HUD click-through: {error}");
    }

    Ok(())
}

fn position_hud_window(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    layout: HudLayout,
) -> tauri::Result<()> {
    let cursor = app.cursor_position()?;
    let position = clamped_hud_position(app, cursor.x, cursor.y, layout)?;
    window.set_position(tauri::PhysicalPosition::new(
        position.0 as i32,
        position.1 as i32,
    ))
}

fn clamped_hud_position(
    app: &tauri::AppHandle,
    cursor_x: f64,
    cursor_y: f64,
    layout: HudLayout,
) -> tauri::Result<(f64, f64)> {
    let mut x = cursor_x - layout.width / 2.0;
    let mut y = cursor_y - layout.vertical_offset - layout.height;

    if let Some(monitor) = app.monitor_from_point(cursor_x, cursor_y)? {
        let work_area = monitor.work_area();
        let min_x = work_area.position.x as f64 + 8.0;
        let min_y = work_area.position.y as f64 + 8.0;
        let max_x = min_x + work_area.size.width as f64 - layout.width - 16.0;
        let max_y = min_y + work_area.size.height as f64 - layout.height - 16.0;

        x = x.clamp(min_x, max_x.max(min_x));
        y = y.clamp(min_y, max_y.max(min_y));
    }

    Ok((x, y))
}

fn layout_for_status(status: &str) -> HudLayout {
    if status == "preview" {
        return HudLayout {
            width: 420.0,
            height: 228.0,
            vertical_offset: 10.0,
            interactive: true,
        };
    }

    if status == "undo_available" {
        return HudLayout {
            width: 220.0,
            height: 52.0,
            vertical_offset: 8.0,
            interactive: true,
        };
    }

    HudLayout {
        width: 180.0,
        height: 52.0,
        vertical_offset: 8.0,
        interactive: false,
    }
}

fn should_auto_hide(status: &str) -> bool {
    matches!(
        status,
        "replaced" | "undo_available" | "error" | "saved_to_clipboard"
    )
}

fn schedule_hud_hide(app: tauri::AppHandle, generation: u64) {
    thread::spawn(move || {
        thread::sleep(HUD_HIDE_DELAY);

        if HUD_GENERATION.load(Ordering::Acquire) != generation {
            return;
        }

        hide_hud(&app);
    });
}
