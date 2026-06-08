use crate::selection::{SelectionError, SelectionMode};
use arboard::{Clipboard, Error as ClipboardError, ImageData};
use std::{
    borrow::Cow,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

const CLIPBOARD_PREPARE_TIMEOUT: Duration = Duration::from_millis(300);
const COPY_ATTEMPT_SETTLE_DELAY: Duration = Duration::from_millis(70);
const COPY_READ_TIMEOUT: Duration = Duration::from_millis(1_200);
const COPY_READ_INTERVAL: Duration = Duration::from_millis(25);
const COPY_RETRY_SETTLE_DELAY: Duration = Duration::from_millis(90);
const COPY_ATTEMPTS: usize = 3;
const PASTE_CLIPBOARD_PREPARE_TIMEOUT: Duration = Duration::from_millis(500);
const PASTE_SETTLE_DELAY: Duration = Duration::from_millis(180);

enum ClipboardBackup {
    Text(String),
    Html {
        html: String,
        alt_text: Option<String>,
    },
    Image(ImageData<'static>),
    FileList(Vec<PathBuf>),
    EmptyOrUnsupported,
}

struct ClipboardCaptureMachine {
    mode: SelectionMode,
    sentinel: String,
}

#[derive(Debug, Clone, Copy)]
enum ClipboardCaptureStep {
    BackupOriginal,
    PrepareSentinel,
    TriggerCopy,
    RestoreOriginal,
    ValidateSelection,
    Complete,
}

impl ClipboardBackup {
    fn read() -> Result<Self, SelectionError> {
        let mut clipboard = Clipboard::new().map_err(|error| {
            SelectionError::Clipboard(format!("failed to open clipboard: {error}"))
        })?;

        let text = clipboard.get_text().ok();

        if let Ok(html) = clipboard.get().html() {
            return Ok(Self::Html {
                html,
                alt_text: text,
            });
        }

        if let Some(text) = text {
            return Ok(Self::Text(text));
        }

        if let Ok(image) = clipboard.get_image() {
            return Ok(Self::Image(image));
        }

        if let Ok(file_list) = clipboard.get().file_list() {
            return Ok(Self::FileList(file_list));
        }

        Ok(Self::EmptyOrUnsupported)
    }

    fn restore(self) -> Result<(), SelectionError> {
        let mut clipboard = Clipboard::new().map_err(|error| {
            SelectionError::Clipboard(format!("failed to reopen clipboard: {error}"))
        })?;

        match self {
            Self::Text(text) => clipboard.set_text(text).map_err(|error| {
                SelectionError::Clipboard(format!("failed to restore text clipboard: {error}"))
            }),
            Self::Html { html, alt_text } => clipboard.set_html(html, alt_text).map_err(|error| {
                SelectionError::Clipboard(format!("failed to restore HTML clipboard: {error}"))
            }),
            Self::Image(image) => clipboard.set_image(image).map_err(|error| {
                SelectionError::Clipboard(format!("failed to restore image clipboard: {error}"))
            }),
            Self::FileList(file_list) => clipboard.set().file_list(&file_list).map_err(|error| {
                SelectionError::Clipboard(format!("failed to restore file list clipboard: {error}"))
            }),
            Self::EmptyOrUnsupported => clipboard.clear().map_err(|error| {
                SelectionError::Clipboard(format!(
                    "failed to clear clipboard after unsupported backup: {error}"
                ))
            }),
        }
    }
}

pub fn setup(_app: &tauri::AppHandle) -> tauri::Result<()> {
    println!("[clipboard] ghost clipboard state machine ready");
    Ok(())
}

pub fn capture_selected_text(mode: SelectionMode) -> Result<String, SelectionError> {
    ClipboardCaptureMachine::new(mode).run()
}

pub fn replace_selected_text(text: &str) -> Result<(), SelectionError> {
    println!(
        "[clipboard] replacement paste started with {} characters",
        text.chars().count()
    );

    let backup = ClipboardBackup::read()?;
    let paste_result = paste_text_from_clipboard(text);
    let restore_result = backup.restore();

    match (paste_result, restore_result) {
        (Ok(()), Ok(())) => {
            println!("[clipboard] replacement paste completed; original clipboard restored");
            Ok(())
        }
        (Err(paste_error), Ok(())) => Err(paste_error),
        (Ok(()), Err(restore_error)) => Err(restore_error),
        (Err(paste_error), Err(restore_error)) => {
            println!(
                "[clipboard] also failed to restore clipboard after paste error: {restore_error}"
            );
            Err(paste_error)
        }
    }
}

pub fn copy_text_to_clipboard(text: &str) -> Result<(), SelectionError> {
    write_clipboard_text(text)
}

impl ClipboardCaptureMachine {
    fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            sentinel: format!("__SHANKA_COPY_SENTINEL_{}__", current_time_nanos()),
        }
    }

    fn run(self) -> Result<String, SelectionError> {
        println!("[clipboard] {} capture started", self.mode.label());
        crate::platform::prepare_for_selection_capture();

        self.log_step(ClipboardCaptureStep::BackupOriginal);
        let backup = ClipboardBackup::read()?;

        self.log_step(ClipboardCaptureStep::PrepareSentinel);
        write_clipboard_text(&self.sentinel)?;
        if let Err(error) = wait_for_clipboard_to_contain(&self.sentinel, CLIPBOARD_PREPARE_TIMEOUT)
        {
            println!("[clipboard] clipboard sentinel was not observable before copy: {error}");
        }

        self.log_step(ClipboardCaptureStep::TriggerCopy);
        let selected_text = copy_and_wait_for_text(&self.sentinel);

        self.log_step(ClipboardCaptureStep::RestoreOriginal);
        if let Err(error) = backup.restore() {
            println!("[clipboard] failed to restore original clipboard: {error}");
            return Err(error);
        }

        let selected_text = selected_text?;

        self.log_step(ClipboardCaptureStep::ValidateSelection);
        if selected_text.is_empty() || selected_text == self.sentinel {
            return Err(SelectionError::NoTextSelected);
        }

        self.log_step(ClipboardCaptureStep::Complete);
        if crate::config::debug_logging_enabled() {
            println!("[clipboard] captured selected text:\n{selected_text}");
        } else {
            println!(
                "[clipboard] captured selected text: {} characters",
                selected_text.chars().count()
            );
        }
        Ok(selected_text)
    }

    fn log_step(&self, step: ClipboardCaptureStep) {
        println!("[clipboard] state: {step:?}");
    }
}

fn copy_and_wait_for_text(sentinel: &str) -> Result<String, SelectionError> {
    let mut last_error = SelectionError::NoTextSelected;

    for attempt in 1..=COPY_ATTEMPTS {
        thread::sleep(COPY_ATTEMPT_SETTLE_DELAY);
        crate::input::copy_selection().map_err(SelectionError::Input)?;
        println!(
            "[clipboard] copy shortcut sent; waiting for clipboard update (attempt {attempt}/{COPY_ATTEMPTS})"
        );

        match wait_for_copied_text(sentinel) {
            Ok(text) => return Ok(text),
            Err(error) => {
                println!(
                    "[clipboard] copy attempt {attempt} did not produce selected text: {error}"
                );
                last_error = error;
            }
        }

        if attempt < COPY_ATTEMPTS {
            thread::sleep(COPY_RETRY_SETTLE_DELAY);
        }
    }

    Err(last_error)
}

fn paste_text_from_clipboard(text: &str) -> Result<(), SelectionError> {
    write_clipboard_text(text)?;
    wait_for_clipboard_to_contain(text, PASTE_CLIPBOARD_PREPARE_TIMEOUT)?;
    crate::input::paste_clipboard().map_err(SelectionError::Input)?;
    thread::sleep(PASTE_SETTLE_DELAY);
    Ok(())
}

fn write_clipboard_text(text: &str) -> Result<(), SelectionError> {
    let mut clipboard = Clipboard::new().map_err(|error| {
        SelectionError::Clipboard(format!("failed to open clipboard before copy: {error}"))
    })?;

    clipboard.set_text(text).map_err(|error| {
        SelectionError::Clipboard(format!("failed to prepare clipboard before copy: {error}"))
    })
}

fn wait_for_copied_text(sentinel: &str) -> Result<String, SelectionError> {
    let started_at = Instant::now();
    let mut last_error = SelectionError::NoTextSelected;

    while started_at.elapsed() < COPY_READ_TIMEOUT {
        match read_clipboard_text() {
            Ok(text) if !text.is_empty() && text != sentinel => return Ok(text),
            Ok(text) if text == sentinel => {
                last_error = SelectionError::NoTextSelected;
            }
            Ok(_) => {
                last_error = SelectionError::NoTextSelected;
            }
            Err(error) => {
                last_error = error;
            }
        }

        thread::sleep(COPY_READ_INTERVAL);
    }

    Err(last_error)
}

fn wait_for_clipboard_to_contain(
    expected_text: &str,
    timeout: Duration,
) -> Result<(), SelectionError> {
    let started_at = Instant::now();
    let mut last_error = SelectionError::NoTextSelected;

    while started_at.elapsed() < timeout {
        match read_clipboard_text() {
            Ok(text) if text == expected_text => return Ok(()),
            Ok(_) => {
                last_error = SelectionError::NoTextSelected;
            }
            Err(error) => {
                last_error = error;
            }
        }

        thread::sleep(COPY_READ_INTERVAL);
    }

    Err(last_error)
}

fn read_clipboard_text() -> Result<String, SelectionError> {
    let mut clipboard = Clipboard::new().map_err(|error| {
        SelectionError::Clipboard(format!("failed to open clipboard after copy: {error}"))
    })?;

    clipboard
        .get_text()
        .map(normalize_clipboard_text)
        .map_err(|error| match error {
            ClipboardError::ContentNotAvailable => SelectionError::NoTextSelected,
            other => SelectionError::Clipboard(format!(
                "failed to read copied selection as text: {other}"
            )),
        })
}

fn normalize_clipboard_text(text: String) -> String {
    Cow::from(text).trim_matches('\0').to_string()
}

fn current_time_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}
