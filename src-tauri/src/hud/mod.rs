use serde::Serialize;
use std::sync::Mutex;
use tauri::Emitter;

const HUD_UPDATE_EVENT: &str = "hud:update";
const HUD_WINDOW_LABEL: &str = "hud";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudUpdate {
    pub status: &'static str,
    pub message: Option<String>,
    pub original_text: Option<String>,
    pub error_code: Option<&'static str>,
    pub preview_id: Option<u64>,
    pub persona_id: Option<String>,
}

static CURRENT_HUD_UPDATE: Mutex<HudUpdate> = Mutex::new(HudUpdate {
    status: "idle",
    message: None,
    original_text: None,
    error_code: None,
    preview_id: None,
    persona_id: None,
});

pub fn current() -> HudUpdate {
    CURRENT_HUD_UPDATE
        .lock()
        .map(|update| update.clone())
        .unwrap_or_else(|_| HudUpdate {
            status: "idle",
            message: None,
            original_text: None,
            error_code: None,
            preview_id: None,
            persona_id: None,
        })
}

pub fn refining(app: &tauri::AppHandle) {
    emit(app, "refining", None, None, None, None, None);
}

pub fn preview(
    app: &tauri::AppHandle,
    preview_id: u64,
    original_text: impl Into<String>,
    text: impl Into<String>,
    persona_id: impl Into<String>,
) {
    emit(
        app,
        "preview",
        Some(text.into()),
        Some(original_text.into()),
        None,
        Some(preview_id),
        Some(persona_id.into()),
    );
}

pub fn preview_error(
    app: &tauri::AppHandle,
    preview_id: u64,
    original_text: impl Into<String>,
    text: impl Into<String>,
    persona_id: impl Into<String>,
    error_code: &'static str,
) {
    emit(
        app,
        "preview",
        Some(text.into()),
        Some(original_text.into()),
        Some(error_code),
        Some(preview_id),
        Some(persona_id.into()),
    );
}

pub fn undo_available(app: &tauri::AppHandle) {
    emit(app, "undo_available", None, None, None, None, None);
}

pub fn saved_to_clipboard(app: &tauri::AppHandle) {
    emit(app, "saved_to_clipboard", None, None, None, None, None);
}

pub fn error(app: &tauri::AppHandle, error_code: &'static str) {
    emit(app, "error", None, None, Some(error_code), None, None);
}

fn emit(
    app: &tauri::AppHandle,
    status: &'static str,
    message: Option<String>,
    original_text: Option<String>,
    error_code: Option<&'static str>,
    preview_id: Option<u64>,
    persona_id: Option<String>,
) {
    let update = HudUpdate {
        status,
        message,
        original_text,
        error_code,
        preview_id,
        persona_id,
    };

    if let Ok(mut current) = CURRENT_HUD_UPDATE.lock() {
        *current = update.clone();
    }

    crate::window::present_hud(app, status);

    if let Err(error) = app.emit_to(HUD_WINDOW_LABEL, HUD_UPDATE_EVENT, update) {
        println!("[hud] failed to emit {HUD_UPDATE_EVENT}: {error}");
    }
}
