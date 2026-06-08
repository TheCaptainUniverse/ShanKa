pub fn setup(_app: &tauri::AppHandle) -> tauri::Result<()> {
    println!("[bridge] local service bridge ready");
    Ok(())
}
