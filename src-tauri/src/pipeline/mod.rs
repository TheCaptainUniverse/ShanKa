use crate::{
    clipboard, config, history, hud, persona,
    rewrite::{self, RewriteError},
    selection::{self, SelectionError, SelectionMode},
};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};

static SAFE_PREVIEW_ID: AtomicU64 = AtomicU64::new(0);
static SAFE_PREVIEW: Mutex<Option<SafePreview>> = Mutex::new(None);
static LAST_REPLACEMENT: Mutex<Option<LastReplacement>> = Mutex::new(None);

#[derive(Clone, Debug)]
struct SafePreview {
    id: u64,
    original_text: String,
    replacement_text: String,
    persona_id: String,
}

#[derive(Clone, Debug)]
struct LastReplacement {
    original_text: String,
}

#[derive(Debug)]
pub struct PipelineOutcome {
    pub selected_characters: usize,
    pub replacement_characters: usize,
    pub selection_provider: &'static str,
    pub rewrite_provider: &'static str,
    pub rewrite_duration: Duration,
    pub total_duration: Duration,
}

#[derive(Debug)]
pub enum PipelineError {
    Selection(SelectionError),
    Rewrite(RewriteError),
    Replacement(SelectionError),
}

impl PipelineError {
    pub(crate) fn error_code(&self) -> &'static str {
        match self {
            Self::Selection(SelectionError::NoTextSelected) => "NO_TEXT_SELECTED",
            Self::Selection(SelectionError::Clipboard(_)) => "CLIPBOARD_ACCESS_FAILED",
            Self::Selection(SelectionError::Input(_)) => "PASTE_BLOCKED",
            Self::Selection(SelectionError::PermissionDenied(_)) => "PLATFORM_PERMISSION_REQUIRED",
            Self::Selection(_) => "API_ERROR",
            Self::Rewrite(RewriteError::Config(_)) => "API_CONFIG_MISSING",
            Self::Rewrite(RewriteError::Timeout) => "NETWORK_TIMEOUT",
            Self::Rewrite(RewriteError::InvalidResponse(_)) => "PROVIDER_RESPONSE_INVALID",
            Self::Rewrite(_) => "API_ERROR",
            Self::Replacement(SelectionError::Clipboard(_)) => "CLIPBOARD_ACCESS_FAILED",
            Self::Replacement(_) => "PASTE_BLOCKED",
        }
    }
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Selection(error) => write!(formatter, "selection failed: {error}"),
            Self::Rewrite(error) => write!(formatter, "rewrite failed: {error}"),
            Self::Replacement(error) => write!(formatter, "replacement failed: {error}"),
        }
    }
}

impl std::error::Error for PipelineError {}

pub fn run(app: &tauri::AppHandle, mode: SelectionMode) -> Result<PipelineOutcome, PipelineError> {
    let started_at = Instant::now();
    if let Err(error) = clear_any_safe_preview() {
        println!("[pipeline] failed to clear stale Safe Mode preview before new run: {error}");
    }
    crate::platform::remember_preview_target_window();
    hud::refining(app);
    if let Some(message) = crate::platform::text_operation_permission_issue() {
        let error = PipelineError::Selection(SelectionError::PermissionDenied(message.to_string()));
        hud::error(app, error.error_code());
        return Err(error);
    }

    let outcome = match mode {
        SelectionMode::Safe => run_safe_preview(app),
        SelectionMode::Magic => run_magic_replacement(app),
    };

    if let Err(error) = &outcome {
        hud::error(app, error.error_code());
    }

    outcome.map(|mut outcome| {
        outcome.total_duration = started_at.elapsed();
        outcome
    })
}

pub fn copy_safe_preview(
    app: &tauri::AppHandle,
    preview_id: u64,
    edited_text: Option<String>,
) -> Result<(), PipelineError> {
    let preview = current_safe_preview(preview_id).map_err(PipelineError::Replacement)?;
    let replacement_text = edited_text.unwrap_or_else(|| preview.replacement_text.clone());
    clipboard::copy_text_to_clipboard(&replacement_text).map_err(PipelineError::Replacement)?;
    record_history(
        app,
        history::RewriteHistoryRecord {
            mode: "safe",
            original_text: &preview.original_text,
            result_text: &replacement_text,
            persona_id: Some(&preview.persona_id),
            action: "copied",
            replaced: false,
        },
    );
    hud::saved_to_clipboard(app);
    println!(
        "[pipeline] Safe Mode preview {} copied {} characters",
        preview.id,
        replacement_text.chars().count()
    );
    Ok(())
}

pub fn replace_safe_preview(
    app: &tauri::AppHandle,
    preview_id: u64,
    edited_text: Option<String>,
) -> Result<(), PipelineError> {
    let preview = current_safe_preview(preview_id).map_err(PipelineError::Replacement)?;
    let replacement_text = edited_text.unwrap_or_else(|| preview.replacement_text.clone());
    crate::window::hide_hud(app);
    crate::platform::restore_preview_target_window();
    let replaced = replace_or_save_to_clipboard(app, &replacement_text)?;
    clear_safe_preview(preview_id).map_err(PipelineError::Replacement)?;
    if replaced {
        remember_last_replacement(&preview.original_text);
        println!(
            "[pipeline] Safe Mode preview {} replaced selection with {} characters",
            preview.id,
            replacement_text.chars().count()
        );
    } else {
        println!(
            "[pipeline] Safe Mode preview {} saved {} characters to clipboard after PASTE_BLOCKED",
            preview.id,
            replacement_text.chars().count()
        );
    }
    record_history(
        app,
        history::RewriteHistoryRecord {
            mode: "safe",
            original_text: &preview.original_text,
            result_text: &replacement_text,
            persona_id: Some(&preview.persona_id),
            action: if replaced {
                "replaced"
            } else {
                "saved_to_clipboard"
            },
            replaced,
        },
    );
    Ok(())
}

pub fn regenerate_safe_preview(app: tauri::AppHandle, preview_id: u64, persona_id: Option<String>) {
    thread::spawn(move || {
        let preview = match current_safe_preview(preview_id) {
            Ok(preview) => preview,
            Err(error) => {
                hud::error(&app, PipelineError::Replacement(error).error_code());
                return;
            }
        };

        let persona_id = persona_id.unwrap_or(preview.persona_id);
        let started_at = Instant::now();
        let rewrite = match rewrite::rewrite_selected_text_with_persona(
            &app,
            SelectionMode::Safe,
            &preview.original_text,
            Some(&persona_id),
        ) {
            Ok(rewrite) => rewrite,
            Err(error) => {
                let error_code = PipelineError::Rewrite(error).error_code();
                if let Err(state_error) = show_safe_preview_error(&app, preview_id, error_code) {
                    println!(
                        "[pipeline] Safe Mode preview {preview_id} was dismissed before regeneration failed: {state_error}"
                    );
                }
                return;
            }
        };

        let next_preview = SafePreview {
            id: preview.id,
            original_text: preview.original_text,
            replacement_text: rewrite.text,
            persona_id,
        };

        if let Err(error) = current_safe_preview(next_preview.id) {
            println!(
                "[pipeline] Safe Mode preview {} was dismissed before regeneration completed: {error}",
                next_preview.id
            );
            return;
        }

        if let Err(error) = replace_safe_preview_state(next_preview.clone()) {
            hud::error(&app, PipelineError::Replacement(error).error_code());
            return;
        }

        hud::preview(
            &app,
            next_preview.id,
            next_preview.replacement_text.clone(),
            next_preview.persona_id.clone(),
        );
        println!(
            "[pipeline] Safe Mode preview {} regenerated to {} characters via {} in {}ms with persona={}",
            next_preview.id,
            next_preview.replacement_text.chars().count(),
            rewrite.provider,
            started_at.elapsed().as_millis(),
            next_preview.persona_id
        );
    });
}

pub fn dismiss_safe_preview(app: &tauri::AppHandle, preview_id: Option<u64>) {
    if let Some(preview_id) = preview_id {
        if let Err(error) = clear_safe_preview(preview_id) {
            println!("[pipeline] failed to clear Safe Mode preview {preview_id}: {error}");
        }
    } else if let Err(error) = clear_any_safe_preview() {
        println!("[pipeline] failed to clear Safe Mode preview: {error}");
    }

    crate::window::hide_hud(app);
}

fn run_safe_preview(app: &tauri::AppHandle) -> Result<PipelineOutcome, PipelineError> {
    let persona_id = config::load_or_create(app)
        .map(|config| persona::normalize_config(&config.personas).default_safe_persona_id)
        .unwrap_or_else(|error| {
            println!(
                "[pipeline] failed to load persona config; using built-in default persona: {error}"
            );
            rewrite::default_safe_persona_id().to_string()
        });
    let selection = capture_and_rewrite(app, SelectionMode::Safe, Some(&persona_id))?;
    let preview_id = next_safe_preview_id();
    replace_safe_preview_state(SafePreview {
        id: preview_id,
        original_text: selection.original_text.clone(),
        replacement_text: selection.replacement_text.clone(),
        persona_id: persona_id.clone(),
    })
    .map_err(PipelineError::Replacement)?;

    hud::preview(
        app,
        preview_id,
        selection.replacement_text.clone(),
        persona_id.clone(),
    );
    println!(
        "[pipeline] Safe Mode preview {} ready with {} characters and persona={}",
        preview_id,
        selection.replacement_text.chars().count(),
        persona_id
    );

    Ok(selection.into_outcome(Duration::ZERO))
}

fn run_magic_replacement(app: &tauri::AppHandle) -> Result<PipelineOutcome, PipelineError> {
    let selection = capture_and_rewrite(app, SelectionMode::Magic, None)?;
    let replaced = replace_or_save_to_clipboard(app, &selection.replacement_text)?;
    if replaced {
        remember_last_replacement(&selection.original_text);
        println!(
            "[pipeline] Magic Mode replaced {} characters",
            selection.replacement_text.chars().count()
        );
    } else {
        println!(
            "[pipeline] Magic Mode saved {} characters to clipboard after PASTE_BLOCKED",
            selection.replacement_text.chars().count()
        );
    }
    record_history(
        app,
        history::RewriteHistoryRecord {
            mode: "magic",
            original_text: &selection.original_text,
            result_text: &selection.replacement_text,
            persona_id: None,
            action: if replaced {
                "replaced"
            } else {
                "saved_to_clipboard"
            },
            replaced,
        },
    );

    Ok(selection.into_outcome(Duration::ZERO))
}

pub fn copy_last_replacement_original(app: &tauri::AppHandle) -> Result<(), PipelineError> {
    let replacement = LAST_REPLACEMENT
        .lock()
        .map_err(|error| {
            PipelineError::Replacement(SelectionError::Clipboard(format!(
                "last replacement lock failed: {error}"
            )))
        })?
        .clone()
        .ok_or_else(|| {
            PipelineError::Replacement(SelectionError::Clipboard(
                "last replacement is no longer available".to_string(),
            ))
        })?;

    clipboard::copy_text_to_clipboard(&replacement.original_text)
        .map_err(PipelineError::Replacement)?;
    hud::saved_to_clipboard(app);
    println!(
        "[pipeline] copied previous original text to clipboard for undo: {} characters",
        replacement.original_text.chars().count()
    );
    Ok(())
}

fn remember_last_replacement(original_text: &str) {
    match LAST_REPLACEMENT.lock() {
        Ok(mut replacement) => {
            *replacement = Some(LastReplacement {
                original_text: original_text.to_string(),
            });
        }
        Err(error) => {
            println!("[pipeline] failed to remember last replacement for undo: {error}");
        }
    }
}

fn record_history(app: &tauri::AppHandle, record: history::RewriteHistoryRecord<'_>) {
    if let Err(error) = history::record(app, record) {
        println!("[history] failed to record rewrite history: {error}");
    }
}

fn replace_or_save_to_clipboard(
    app: &tauri::AppHandle,
    replacement_text: &str,
) -> Result<bool, PipelineError> {
    if crate::platform::paste_target_requires_elevation() {
        clipboard::copy_text_to_clipboard(replacement_text).map_err(PipelineError::Replacement)?;
        hud::saved_to_clipboard(app);
        println!(
            "[pipeline] PASTE_BLOCKED: target window appears elevated; saved replacement to clipboard"
        );
        return Ok(false);
    }

    match clipboard::replace_selected_text(replacement_text) {
        Ok(()) => {
            hud::undo_available(app);
            Ok(true)
        }
        Err(paste_error) => {
            println!(
                "[pipeline] PASTE_BLOCKED: paste failed; saving replacement to clipboard: {paste_error}"
            );
            clipboard::copy_text_to_clipboard(replacement_text)
                .map_err(PipelineError::Replacement)?;
            hud::saved_to_clipboard(app);
            Ok(false)
        }
    }
}

struct RewritePipelineResult {
    original_text: String,
    replacement_text: String,
    selection_provider: &'static str,
    rewrite_provider: &'static str,
    rewrite_duration: Duration,
}

impl RewritePipelineResult {
    fn into_outcome(self, total_duration: Duration) -> PipelineOutcome {
        PipelineOutcome {
            selected_characters: self.original_text.chars().count(),
            replacement_characters: self.replacement_text.chars().count(),
            selection_provider: self.selection_provider,
            rewrite_provider: self.rewrite_provider,
            rewrite_duration: self.rewrite_duration,
            total_duration,
        }
    }
}

fn capture_and_rewrite(
    app: &tauri::AppHandle,
    mode: SelectionMode,
    persona_id: Option<&str>,
) -> Result<RewritePipelineResult, PipelineError> {
    let selection = selection::read_selected_text(mode).map_err(PipelineError::Selection)?;
    println!(
        "[pipeline] {} captured {} characters via {}",
        mode.label(),
        selection.text.chars().count(),
        selection.provider
    );

    let rewrite = if let Some(persona_id) = persona_id {
        rewrite::rewrite_selected_text_with_persona(app, mode, &selection.text, Some(persona_id))
    } else {
        rewrite::rewrite_selected_text(app, mode, &selection.text)
    }
    .map_err(PipelineError::Rewrite)?;
    println!(
        "[pipeline] {} rewritten to {} characters via {} in {}ms",
        mode.label(),
        rewrite.text.chars().count(),
        rewrite.provider,
        rewrite.duration.as_millis()
    );

    Ok(RewritePipelineResult {
        original_text: selection.text,
        replacement_text: rewrite.text,
        selection_provider: selection.provider,
        rewrite_provider: rewrite.provider,
        rewrite_duration: rewrite.duration,
    })
}

fn next_safe_preview_id() -> u64 {
    SAFE_PREVIEW_ID.fetch_add(1, Ordering::AcqRel) + 1
}

fn current_safe_preview(preview_id: u64) -> Result<SafePreview, SelectionError> {
    let preview = SAFE_PREVIEW
        .lock()
        .map_err(|error| SelectionError::Clipboard(format!("safe preview lock failed: {error}")))?
        .clone()
        .ok_or_else(|| {
            SelectionError::Clipboard("safe preview is no longer available".to_string())
        })?;

    if preview.id != preview_id {
        return Err(SelectionError::Clipboard(
            "safe preview has already changed".to_string(),
        ));
    }

    Ok(preview)
}

fn clear_safe_preview(preview_id: u64) -> Result<(), SelectionError> {
    let mut preview_slot = SAFE_PREVIEW
        .lock()
        .map_err(|error| SelectionError::Clipboard(format!("safe preview lock failed: {error}")))?;
    let Some(preview) = preview_slot.as_ref() else {
        return Ok(());
    };

    if preview.id != preview_id {
        return Err(SelectionError::Clipboard(
            "safe preview has already changed".to_string(),
        ));
    }

    *preview_slot = None;
    Ok(())
}

fn clear_any_safe_preview() -> Result<(), SelectionError> {
    let mut preview_slot = SAFE_PREVIEW
        .lock()
        .map_err(|error| SelectionError::Clipboard(format!("safe preview lock failed: {error}")))?;
    *preview_slot = None;
    Ok(())
}

fn replace_safe_preview_state(preview: SafePreview) -> Result<(), SelectionError> {
    let mut preview_slot = SAFE_PREVIEW
        .lock()
        .map_err(|error| SelectionError::Clipboard(format!("safe preview lock failed: {error}")))?;
    *preview_slot = Some(preview);
    Ok(())
}

fn show_safe_preview_error(
    app: &tauri::AppHandle,
    preview_id: u64,
    error_code: &'static str,
) -> Result<(), SelectionError> {
    let preview = current_safe_preview(preview_id)?;
    hud::preview_error(
        app,
        preview.id,
        preview.replacement_text,
        preview.persona_id,
        error_code,
    );
    Ok(())
}
