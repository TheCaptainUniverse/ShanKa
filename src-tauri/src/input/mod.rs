use crate::platform::PRIMARY_MODIFIER;

#[cfg(not(target_os = "windows"))]
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};

pub fn setup(_app: &tauri::AppHandle) -> tauri::Result<()> {
    println!("[input] input simulator ready with {PRIMARY_MODIFIER} as primary modifier");
    Ok(())
}

pub fn copy_selection() -> Result<(), String> {
    send_primary_shortcut('c')
}

#[allow(dead_code)]
pub fn paste_clipboard() -> Result<(), String> {
    send_primary_shortcut('v')
}

#[cfg(target_os = "windows")]
fn send_primary_shortcut(key: char) -> Result<(), String> {
    println!(
        "[input] sending Windows SendInput shortcut: Ctrl+{}",
        key.to_ascii_uppercase()
    );
    windows_send_ctrl_shortcut(key)
}

#[cfg(not(target_os = "windows"))]
fn send_primary_shortcut(key: char) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|error| format!("failed to initialize input simulator: {error}"))?;
    let modifier = crate::platform::primary_modifier_key();

    enigo
        .key(modifier, Press)
        .map_err(|error| format!("failed to press {PRIMARY_MODIFIER}: {error}"))?;

    let key_result = enigo
        .key(Key::Unicode(key), Click)
        .map_err(|error| format!("failed to click shortcut key '{key}': {error}"));
    let release_result = enigo
        .key(modifier, Release)
        .map_err(|error| format!("failed to release {PRIMARY_MODIFIER}: {error}"));

    key_result.and(release_result)
}

#[cfg(target_os = "windows")]
const WINDOWS_SHORTCUT_KEY_DELAY: std::time::Duration = std::time::Duration::from_millis(18);

#[cfg(target_os = "windows")]
fn windows_send_ctrl_shortcut(key: char) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{KEYEVENTF_KEYUP, VK_CONTROL};

    let key = key.to_ascii_uppercase();
    if !key.is_ascii_alphabetic() {
        return Err(format!("unsupported Windows shortcut key: {key}"));
    }

    let key_vk = key as u16;
    let press_ctrl = send_windows_keyboard_events(&[keyboard_input(VK_CONTROL, 0)], "press Ctrl");
    if let Err(error) = press_ctrl {
        let _ = send_windows_keyboard_events(
            &[keyboard_input(VK_CONTROL, KEYEVENTF_KEYUP)],
            "release Ctrl after failed press",
        );
        return Err(error);
    }

    std::thread::sleep(WINDOWS_SHORTCUT_KEY_DELAY);

    let key_result = send_windows_keyboard_events(
        &[
            keyboard_input(key_vk, 0),
            keyboard_input(key_vk, KEYEVENTF_KEYUP),
        ],
        "click shortcut key",
    );

    std::thread::sleep(WINDOWS_SHORTCUT_KEY_DELAY);

    let release_result = send_windows_keyboard_events(
        &[keyboard_input(VK_CONTROL, KEYEVENTF_KEYUP)],
        "release Ctrl",
    );

    key_result.and(release_result)
}

#[cfg(target_os = "windows")]
fn send_windows_keyboard_events(
    inputs: &[windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT],
    action: &str,
) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT};

    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };

    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err(format!(
            "Windows SendInput failed to {action}; sent {sent}/{} events; last OS error: {}",
            inputs.len(),
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(target_os = "windows")]
fn keyboard_input(vk: u16, flags: u32) -> windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
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
