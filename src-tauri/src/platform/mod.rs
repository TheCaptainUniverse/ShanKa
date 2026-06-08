use serde::Serialize;

pub fn setup(_app: &tauri::AppHandle) -> tauri::Result<()> {
    println!("[platform] platform adapter ready");
    Ok(())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformStatus {
    pub os: &'static str,
    pub accessibility: PlatformCapability,
    pub global_hotkey: PlatformCapability,
    pub clipboard: PlatformCapability,
    pub input_simulation: PlatformCapability,
    pub linux_session: Option<String>,
    pub notes: Vec<&'static str>,
    pub settings_action_available: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapability {
    pub status: &'static str,
    pub message_key: &'static str,
}

#[allow(dead_code)]
impl PlatformCapability {
    fn ok(message_key: &'static str) -> Self {
        Self {
            status: "ok",
            message_key,
        }
    }

    fn warning(message_key: &'static str) -> Self {
        Self {
            status: "warning",
            message_key,
        }
    }

    fn blocked(message_key: &'static str) -> Self {
        Self {
            status: "blocked",
            message_key,
        }
    }
}

pub fn status() -> PlatformStatus {
    platform_status()
}

pub fn open_permission_settings() -> Result<(), String> {
    open_platform_permission_settings()
}

pub fn text_operation_permission_issue() -> Option<&'static str> {
    platform_text_operation_permission_issue()
}

#[cfg(target_os = "windows")]
static PREVIEW_TARGET_WINDOW: std::sync::atomic::AtomicIsize =
    std::sync::atomic::AtomicIsize::new(0);

const HOTKEY_KEY_RELEASE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(900);
const HOTKEY_KEY_RELEASE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(20);
const MENU_CANCEL_SETTLE_DELAY: std::time::Duration = std::time::Duration::from_millis(90);
const FOCUS_RESTORE_SETTLE_DELAY: std::time::Duration = std::time::Duration::from_millis(80);

#[cfg(target_os = "macos")]
pub const PRIMARY_MODIFIER: &str = "Meta";

#[cfg(not(target_os = "macos"))]
pub const PRIMARY_MODIFIER: &str = "Control";

#[cfg(target_os = "macos")]
pub fn primary_modifier_key() -> enigo::Key {
    enigo::Key::Meta
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn primary_modifier_key() -> enigo::Key {
    enigo::Key::Control
}

#[cfg(target_os = "windows")]
fn platform_status() -> PlatformStatus {
    PlatformStatus {
        os: "windows",
        accessibility: PlatformCapability::ok("settings.platform.accessibility.notRequired"),
        global_hotkey: PlatformCapability::ok("settings.platform.globalHotkey.ok"),
        clipboard: PlatformCapability::ok("settings.platform.clipboard.ok"),
        input_simulation: PlatformCapability::ok("settings.platform.input.ok"),
        linux_session: None,
        notes: vec!["settings.platform.note.windowsAdminFallback"],
        settings_action_available: false,
    }
}

#[cfg(target_os = "macos")]
fn platform_status() -> PlatformStatus {
    let accessibility_trusted = macos_accessibility_trusted();
    PlatformStatus {
        os: "macos",
        accessibility: if accessibility_trusted {
            PlatformCapability::ok("settings.platform.accessibility.granted")
        } else {
            PlatformCapability::blocked("settings.platform.accessibility.missing")
        },
        global_hotkey: PlatformCapability::ok("settings.platform.globalHotkey.ok"),
        clipboard: PlatformCapability::ok("settings.platform.clipboard.ok"),
        input_simulation: if accessibility_trusted {
            PlatformCapability::ok("settings.platform.input.ok")
        } else {
            PlatformCapability::blocked("settings.platform.input.needsAccessibility")
        },
        linux_session: None,
        notes: vec!["settings.platform.note.macosAccessibility"],
        settings_action_available: true,
    }
}

#[cfg(target_os = "linux")]
fn platform_status() -> PlatformStatus {
    let session = std::env::var("XDG_SESSION_TYPE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let wayland = session.eq_ignore_ascii_case("wayland");

    PlatformStatus {
        os: "linux",
        accessibility: PlatformCapability::warning("settings.platform.accessibility.linuxLimited"),
        global_hotkey: if wayland {
            PlatformCapability::warning("settings.platform.globalHotkey.waylandLimited")
        } else {
            PlatformCapability::ok("settings.platform.globalHotkey.x11Likely")
        },
        clipboard: if wayland {
            PlatformCapability::warning("settings.platform.clipboard.waylandLimited")
        } else {
            PlatformCapability::ok("settings.platform.clipboard.x11Likely")
        },
        input_simulation: if wayland {
            PlatformCapability::warning("settings.platform.input.waylandLimited")
        } else {
            PlatformCapability::ok("settings.platform.input.x11Likely")
        },
        linux_session: Some(session),
        notes: if wayland {
            vec!["settings.platform.note.linuxWayland"]
        } else {
            vec!["settings.platform.note.linuxX11"]
        },
        settings_action_available: false,
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn platform_status() -> PlatformStatus {
    PlatformStatus {
        os: "unknown",
        accessibility: PlatformCapability::warning("settings.platform.accessibility.unknown"),
        global_hotkey: PlatformCapability::warning("settings.platform.globalHotkey.unknown"),
        clipboard: PlatformCapability::warning("settings.platform.clipboard.unknown"),
        input_simulation: PlatformCapability::warning("settings.platform.input.unknown"),
        linux_session: None,
        notes: vec!["settings.platform.note.unknown"],
        settings_action_available: false,
    }
}

#[cfg(target_os = "macos")]
fn platform_text_operation_permission_issue() -> Option<&'static str> {
    (!macos_accessibility_trusted()).then_some("macOS Accessibility permission is required")
}

#[cfg(not(target_os = "macos"))]
fn platform_text_operation_permission_issue() -> Option<&'static str> {
    None
}

#[cfg(target_os = "macos")]
fn open_platform_permission_settings() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open macOS Accessibility settings: {error}"))
}

#[cfg(not(target_os = "macos"))]
fn open_platform_permission_settings() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn macos_accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> std::ffi::c_uchar;
}

#[cfg(target_os = "windows")]
pub fn wait_for_hotkey_keys_released(hotkey: global_hotkey::hotkey::HotKey) -> bool {
    use global_hotkey::hotkey::Modifiers;
    use std::time::Instant;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU,
        VK_RSHIFT, VK_RWIN, VK_SHIFT,
    };

    let mut virtual_keys = Vec::new();
    if hotkey.mods.contains(Modifiers::ALT) {
        virtual_keys.extend([VK_MENU, VK_LMENU, VK_RMENU]);
    }
    if hotkey.mods.contains(Modifiers::SHIFT) {
        virtual_keys.extend([VK_SHIFT, VK_LSHIFT, VK_RSHIFT]);
    }
    if hotkey.mods.contains(Modifiers::CONTROL) {
        virtual_keys.extend([VK_CONTROL, VK_LCONTROL, VK_RCONTROL]);
    }
    if hotkey.mods.contains(Modifiers::SUPER) {
        virtual_keys.extend([VK_LWIN, VK_RWIN]);
    }
    if let Some(main_key) = windows_virtual_key_for_code(hotkey.key) {
        virtual_keys.push(main_key);
    } else {
        println!(
            "[platform] unsupported Windows release tracking for hotkey key {}; waiting for modifiers only",
            hotkey.key
        );
    }

    let started_at = Instant::now();
    while started_at.elapsed() < HOTKEY_KEY_RELEASE_TIMEOUT {
        if virtual_keys
            .iter()
            .all(|virtual_key| !is_windows_key_down(*virtual_key))
        {
            return true;
        }

        std::thread::sleep(HOTKEY_KEY_RELEASE_INTERVAL);
    }

    false
}

#[cfg(target_os = "windows")]
pub fn prepare_for_selection_capture() {
    if !is_windows_menu_mode_active() {
        return;
    }

    match send_windows_escape_key() {
        Ok(()) => {
            println!(
                "[platform] Windows menu mode detected after Alt hotkey; sent Escape before capture"
            );
            std::thread::sleep(MENU_CANCEL_SETTLE_DELAY);
        }
        Err(error) => {
            println!("[platform] failed to cancel Windows menu mode before capture: {error}");
        }
    }
}

#[cfg(target_os = "windows")]
pub fn remember_preview_target_window() {
    use std::sync::atomic::Ordering;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let foreground_window = unsafe { GetForegroundWindow() };
    PREVIEW_TARGET_WINDOW.store(foreground_window as isize, Ordering::Release);
}

#[cfg(target_os = "windows")]
pub fn restore_preview_target_window() {
    use std::sync::atomic::Ordering;
    use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindow, SetForegroundWindow};

    let window = PREVIEW_TARGET_WINDOW.load(Ordering::Acquire) as isize;
    if window == 0 {
        return;
    }

    let window = window as windows_sys::Win32::Foundation::HWND;
    let restored =
        unsafe { !window.is_null() && IsWindow(window) != 0 && SetForegroundWindow(window) != 0 };
    if !restored {
        println!("[platform] failed to restore focus to preview target window");
    }

    std::thread::sleep(FOCUS_RESTORE_SETTLE_DELAY);
}

#[cfg(target_os = "windows")]
pub fn paste_target_requires_elevation() -> bool {
    use std::sync::atomic::Ordering;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    if is_current_process_elevated() {
        return false;
    }

    let remembered_window = PREVIEW_TARGET_WINDOW.load(Ordering::Acquire) as isize;
    let target_window = if remembered_window == 0 {
        unsafe { GetForegroundWindow() }
    } else {
        remembered_window as windows_sys::Win32::Foundation::HWND
    };

    if target_window.is_null() {
        return false;
    }

    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(target_window, &mut process_id) };
    if process_id == 0 || process_id == std::process::id() {
        return false;
    }

    match is_windows_process_elevated(process_id) {
        Ok(elevated) => elevated,
        Err(error) => {
            println!(
                "[platform] could not inspect paste target integrity; using clipboard fallback: {error}"
            );
            true
        }
    }
}

#[cfg(target_os = "windows")]
fn is_windows_key_down(virtual_key: u16) -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    unsafe { GetAsyncKeyState(virtual_key as i32) & 0x8000u16 as i16 != 0 }
}

#[cfg(target_os = "windows")]
fn is_current_process_elevated() -> bool {
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;

    is_windows_process_elevated(unsafe { GetCurrentProcessId() }).unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn is_windows_process_elevated(process_id: u32) -> Result<bool, String> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::Threading::{OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION},
    };

    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        return Err(format!(
            "failed to open process {process_id}: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut token = std::ptr::null_mut();
    let opened_token = unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } != 0;
    unsafe { CloseHandle(process) };
    if !opened_token {
        return Err(format!(
            "failed to open process token for {process_id}: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut return_length = 0;
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        )
    } != 0;
    unsafe { CloseHandle(token) };

    if ok {
        Ok(elevation.TokenIsElevated != 0)
    } else {
        Err(format!(
            "failed to query process token elevation for {process_id}: {}",
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(target_os = "windows")]
fn windows_virtual_key_for_code(code: global_hotkey::hotkey::Code) -> Option<u16> {
    use global_hotkey::hotkey::Code;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        VK_0, VK_A, VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_HOME, VK_INSERT,
        VK_LEFT, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
    };

    match code {
        Code::KeyA => Some(VK_A),
        Code::KeyB => Some(VK_A + 1),
        Code::KeyC => Some(VK_A + 2),
        Code::KeyD => Some(VK_A + 3),
        Code::KeyE => Some(VK_A + 4),
        Code::KeyF => Some(VK_A + 5),
        Code::KeyG => Some(VK_A + 6),
        Code::KeyH => Some(VK_A + 7),
        Code::KeyI => Some(VK_A + 8),
        Code::KeyJ => Some(VK_A + 9),
        Code::KeyK => Some(VK_A + 10),
        Code::KeyL => Some(VK_A + 11),
        Code::KeyM => Some(VK_A + 12),
        Code::KeyN => Some(VK_A + 13),
        Code::KeyO => Some(VK_A + 14),
        Code::KeyP => Some(VK_A + 15),
        Code::KeyQ => Some(VK_A + 16),
        Code::KeyR => Some(VK_A + 17),
        Code::KeyS => Some(VK_A + 18),
        Code::KeyT => Some(VK_A + 19),
        Code::KeyU => Some(VK_A + 20),
        Code::KeyV => Some(VK_A + 21),
        Code::KeyW => Some(VK_A + 22),
        Code::KeyX => Some(VK_A + 23),
        Code::KeyY => Some(VK_A + 24),
        Code::KeyZ => Some(VK_A + 25),
        Code::Digit0 => Some(VK_0),
        Code::Digit1 => Some(VK_0 + 1),
        Code::Digit2 => Some(VK_0 + 2),
        Code::Digit3 => Some(VK_0 + 3),
        Code::Digit4 => Some(VK_0 + 4),
        Code::Digit5 => Some(VK_0 + 5),
        Code::Digit6 => Some(VK_0 + 6),
        Code::Digit7 => Some(VK_0 + 7),
        Code::Digit8 => Some(VK_0 + 8),
        Code::Digit9 => Some(VK_0 + 9),
        Code::Space => Some(VK_SPACE),
        Code::Enter | Code::NumpadEnter => Some(VK_RETURN),
        Code::Tab => Some(VK_TAB),
        Code::Escape => Some(VK_ESCAPE),
        Code::Backspace => Some(VK_BACK),
        Code::Delete => Some(VK_DELETE),
        Code::Insert => Some(VK_INSERT),
        Code::Home => Some(VK_HOME),
        Code::End => Some(VK_END),
        Code::PageUp => Some(VK_PRIOR),
        Code::PageDown => Some(VK_NEXT),
        Code::ArrowLeft => Some(VK_LEFT),
        Code::ArrowRight => Some(VK_RIGHT),
        Code::ArrowUp => Some(VK_UP),
        Code::ArrowDown => Some(VK_DOWN),
        Code::F1 => Some(VK_F1),
        Code::F2 => Some(VK_F1 + 1),
        Code::F3 => Some(VK_F1 + 2),
        Code::F4 => Some(VK_F1 + 3),
        Code::F5 => Some(VK_F1 + 4),
        Code::F6 => Some(VK_F1 + 5),
        Code::F7 => Some(VK_F1 + 6),
        Code::F8 => Some(VK_F1 + 7),
        Code::F9 => Some(VK_F1 + 8),
        Code::F10 => Some(VK_F1 + 9),
        Code::F11 => Some(VK_F1 + 10),
        Code::F12 => Some(VK_F1 + 11),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn is_windows_menu_mode_active() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
        GUI_INMENUMODE, GUI_POPUPMENUMODE, GUI_SYSTEMMENUMODE,
    };

    let foreground_window = unsafe { GetForegroundWindow() };
    if foreground_window.is_null() {
        return false;
    }

    let thread_id = unsafe { GetWindowThreadProcessId(foreground_window, std::ptr::null_mut()) };
    if thread_id == 0 {
        return false;
    }

    let mut info = GUITHREADINFO::default();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;

    let ok = unsafe { GetGUIThreadInfo(thread_id, &mut info) } != 0;
    if !ok {
        return false;
    }

    let menu_flags = GUI_INMENUMODE | GUI_SYSTEMMENUMODE | GUI_POPUPMENUMODE;
    info.flags & menu_flags != 0
}

#[cfg(target_os = "windows")]
fn send_windows_escape_key() -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, KEYEVENTF_KEYUP, VK_ESCAPE,
    };

    let mut inputs = [
        windows_keyboard_input(VK_ESCAPE, 0),
        windows_keyboard_input(VK_ESCAPE, KEYEVENTF_KEYUP),
    ];

    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };

    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err(format!(
            "Windows SendInput sent {sent}/{} Escape events; last OS error: {}",
            inputs.len(),
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(target_os = "windows")]
fn windows_keyboard_input(
    vk: u16,
    flags: u32,
) -> windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
    };

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(not(target_os = "windows"))]
pub fn wait_for_hotkey_keys_released(_hotkey: global_hotkey::hotkey::HotKey) -> bool {
    true
}

#[cfg(not(target_os = "windows"))]
pub fn prepare_for_selection_capture() {}

#[cfg(not(target_os = "windows"))]
pub fn remember_preview_target_window() {}

#[cfg(not(target_os = "windows"))]
pub fn restore_preview_target_window() {}

#[cfg(not(target_os = "windows"))]
pub fn paste_target_requires_elevation() -> bool {
    false
}
