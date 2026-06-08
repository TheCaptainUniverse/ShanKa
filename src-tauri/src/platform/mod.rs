pub fn setup(_app: &tauri::AppHandle) -> tauri::Result<()> {
    println!("[platform] platform adapter ready");
    Ok(())
}

#[cfg(target_os = "macos")]
pub const PRIMARY_MODIFIER: &str = "Meta";

#[cfg(not(target_os = "macos"))]
pub const PRIMARY_MODIFIER: &str = "Control";
