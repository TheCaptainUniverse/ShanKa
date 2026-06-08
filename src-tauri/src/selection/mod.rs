pub trait SelectionProvider {
    fn name(&self) -> &'static str;
    fn read_selected_text(&self, mode: SelectionMode) -> Result<String, SelectionError>;
}

#[derive(Debug, Clone, Copy)]
pub enum SelectionMode {
    Safe,
    Magic,
}

impl SelectionMode {
    pub fn label(self) -> &'static str {
        match self {
            SelectionMode::Safe => "Safe Mode",
            SelectionMode::Magic => "Magic Mode",
        }
    }
}

#[derive(Debug)]
pub struct SelectedText {
    pub text: String,
    pub provider: &'static str,
}

#[derive(Debug)]
pub enum SelectionError {
    Clipboard(String),
    Input(String),
    #[allow(dead_code)]
    NativeProvider {
        provider: &'static str,
        message: String,
    },
    NoTextSelected,
    #[allow(dead_code)]
    UnsupportedPlatformNativeProvider(&'static str),
    PermissionDenied(String),
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectionError::Clipboard(message) => {
                write!(formatter, "clipboard error: {message}")
            }
            SelectionError::Input(message) => write!(formatter, "input error: {message}"),
            SelectionError::NativeProvider { provider, message } => {
                write!(formatter, "{provider} error: {message}")
            }
            SelectionError::NoTextSelected => write!(formatter, "no text selected"),
            SelectionError::UnsupportedPlatformNativeProvider(provider) => {
                write!(formatter, "{provider} is not implemented yet")
            }
            SelectionError::PermissionDenied(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for SelectionError {}

struct ClipboardSelectionProvider;

impl SelectionProvider for ClipboardSelectionProvider {
    fn name(&self) -> &'static str {
        "clipboard-copy"
    }

    fn read_selected_text(&self, mode: SelectionMode) -> Result<String, SelectionError> {
        crate::clipboard::capture_selected_text(mode)
    }
}

pub fn setup(_app: &tauri::AppHandle) -> tauri::Result<()> {
    println!(
        "[selection] provider pipeline ready: primary=clipboard-copy, native-slot={}",
        native::platform_provider().name()
    );
    Ok(())
}

pub fn read_selected_text(mode: SelectionMode) -> Result<SelectedText, SelectionError> {
    let primary_provider = ClipboardSelectionProvider;
    match primary_provider.read_selected_text(mode) {
        Ok(text) => {
            return Ok(SelectedText {
                text,
                provider: primary_provider.name(),
            });
        }
        Err(primary_error) => {
            println!(
                "[selection] {} failed: {primary_error}; trying native provider",
                primary_provider.name()
            );

            let native_provider = native::platform_provider();
            match native_provider.read_selected_text(mode) {
                Ok(text) => {
                    return Ok(SelectedText {
                        text,
                        provider: native_provider.name(),
                    });
                }
                Err(SelectionError::UnsupportedPlatformNativeProvider(provider)) => {
                    println!("[selection] {provider} unavailable after clipboard-copy failure");
                }
                Err(error) => {
                    println!(
                        "[selection] {} also failed after clipboard-copy: {error}",
                        native_provider.name()
                    );
                }
            }

            Err(primary_error)
        }
    }
}

#[allow(dead_code)]
pub fn read_selected_text_native_first(
    mode: SelectionMode,
) -> Result<SelectedText, SelectionError> {
    let native_provider = native::platform_provider();
    match native_provider.read_selected_text(mode) {
        Ok(text) => Ok(SelectedText {
            text,
            provider: native_provider.name(),
        }),
        Err(SelectionError::UnsupportedPlatformNativeProvider(provider)) => {
            println!("[selection] {provider} unavailable; falling back to clipboard-copy");
            read_selected_text_with_clipboard_only(mode)
        }
        Err(error) => {
            println!(
                "[selection] {} failed: {error}; falling back to clipboard-copy",
                native_provider.name()
            );
            read_selected_text_with_clipboard_only(mode)
        }
    }
}

fn read_selected_text_with_clipboard_only(
    mode: SelectionMode,
) -> Result<SelectedText, SelectionError> {
    let provider = ClipboardSelectionProvider;
    provider.read_selected_text(mode).map(|text| SelectedText {
        text,
        provider: provider.name(),
    })
}

mod native {
    use super::{SelectionError, SelectionMode, SelectionProvider};

    pub struct PlatformNativeSelectionProvider;

    impl SelectionProvider for PlatformNativeSelectionProvider {
        fn name(&self) -> &'static str {
            platform_provider_name()
        }

        fn read_selected_text(&self, _mode: SelectionMode) -> Result<String, SelectionError> {
            Err(SelectionError::UnsupportedPlatformNativeProvider(
                self.name(),
            ))
        }
    }

    pub fn platform_provider() -> PlatformNativeSelectionProvider {
        PlatformNativeSelectionProvider
    }

    #[cfg(target_os = "windows")]
    fn platform_provider_name() -> &'static str {
        "windows-native-selection"
    }

    #[cfg(target_os = "macos")]
    fn platform_provider_name() -> &'static str {
        "macos-accessibility"
    }

    #[cfg(target_os = "linux")]
    fn platform_provider_name() -> &'static str {
        "linux-at-spi"
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    fn platform_provider_name() -> &'static str {
        "native-selection"
    }
}
