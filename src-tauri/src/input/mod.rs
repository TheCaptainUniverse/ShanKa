use crate::platform::PRIMARY_MODIFIER;

pub fn setup(_app: &tauri::AppHandle) -> tauri::Result<()> {
    println!("[input] input simulator ready with {PRIMARY_MODIFIER} as primary modifier");
    Ok(())
}
