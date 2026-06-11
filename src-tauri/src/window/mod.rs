use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const MAIN_WINDOW_LABEL: &str = "main";
const HUD_WINDOW_LABEL: &str = "hud";
const HUD_HIDE_DELAY: Duration = Duration::from_millis(2_200);

static HUD_GENERATION: AtomicU64 = AtomicU64::new(0);
static HUD_POSITION: Mutex<HudPositionState> = Mutex::new(HudPositionState {
    anchor: None,
    preview_position: None,
});

#[derive(Clone, Copy)]
struct HudLayout {
    width: f64,
    height: f64,
    vertical_offset: f64,
    interactive: bool,
}

#[derive(Clone, Copy)]
struct HudPoint {
    x: f64,
    y: f64,
}

struct HudPositionState {
    anchor: Option<HudPoint>,
    preview_position: Option<HudPoint>,
}

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    apply_settings_window_icon(app);
    install_settings_window_close_handler(app);
    create_hud_window(app)?;
    println!("[window] settings and HUD window manager ready");
    Ok(())
}

pub fn present_hud(app: &tauri::AppHandle, status: &'static str) {
    let generation = HUD_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    let layout = layout_for_status(status);

    if status == "refining" {
        begin_hud_position_session(app);
    }

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

        if let Err(error) = position_hud_window(app, &window, layout, status) {
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

fn apply_settings_window_icon(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        println!("[window] settings window not found; app icon was not applied");
        return;
    };

    match crate::app_icon::load() {
        Ok(icon) => {
            if let Err(error) = window.set_icon(icon) {
                println!("[window] failed to apply settings window icon: {error}");
            }
        }
        Err(error) => {
            println!("[window] failed to load app icon: {error}");
        }
    }
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

    if let Ok(icon) = crate::app_icon::load() {
        if let Err(error) = window.set_icon(icon) {
            println!("[window] failed to apply HUD window icon: {error}");
        }
    }

    if let Err(error) = window.set_ignore_cursor_events(true) {
        println!("[window] failed to enable HUD click-through: {error}");
    }

    Ok(())
}

fn position_hud_window(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    layout: HudLayout,
    status: &str,
) -> tauri::Result<()> {
    let position = hud_position_for_status(app, layout, status)?;
    window.set_position(tauri::PhysicalPosition::new(
        position.x as i32,
        position.y as i32,
    ))
}

fn begin_hud_position_session(app: &tauri::AppHandle) {
    let anchor = app.cursor_position().ok().map(|cursor| HudPoint {
        x: cursor.x,
        y: cursor.y,
    });

    match HUD_POSITION.lock() {
        Ok(mut state) => {
            state.anchor = anchor;
            state.preview_position = None;
        }
        Err(error) => {
            println!("[window] failed to reset HUD position session: {error}");
        }
    }
}

fn hud_position_for_status(
    app: &tauri::AppHandle,
    layout: HudLayout,
    status: &str,
) -> tauri::Result<HudPoint> {
    if status == "preview" {
        if let Some(position) = locked_preview_position() {
            return Ok(position);
        }
    }

    let anchor = hud_anchor(app)?;
    let position = clamped_hud_position(app, anchor.x, anchor.y, layout)?;

    if status == "preview" {
        remember_preview_position(position);
    }

    Ok(position)
}

fn locked_preview_position() -> Option<HudPoint> {
    HUD_POSITION
        .lock()
        .ok()
        .and_then(|state| state.preview_position)
}

fn remember_preview_position(position: HudPoint) {
    if let Ok(mut state) = HUD_POSITION.lock() {
        if state.preview_position.is_none() {
            state.preview_position = Some(position);
        }
    }
}

fn hud_anchor(app: &tauri::AppHandle) -> tauri::Result<HudPoint> {
    if let Some(anchor) = HUD_POSITION.lock().ok().and_then(|state| state.anchor) {
        return Ok(anchor);
    }

    let cursor = app.cursor_position()?;
    Ok(HudPoint {
        x: cursor.x,
        y: cursor.y,
    })
}

fn clamped_hud_position(
    app: &tauri::AppHandle,
    cursor_x: f64,
    cursor_y: f64,
    layout: HudLayout,
) -> tauri::Result<HudPoint> {
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

    Ok(HudPoint { x, y })
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
