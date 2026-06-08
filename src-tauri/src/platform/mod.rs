pub fn setup(_app: &tauri::AppHandle) -> tauri::Result<()> {
    println!("[platform] platform adapter ready");
    Ok(())
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
fn is_windows_key_down(virtual_key: u16) -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    unsafe { GetAsyncKeyState(virtual_key as i32) & 0x8000u16 as i16 != 0 }
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
