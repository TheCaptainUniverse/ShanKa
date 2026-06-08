const API_KEY_ACCOUNT: &str = "openai-compatible-api-key";
const API_KEY_REF: &str = "keychain://shanka/openai-compatible-api-key";
const SERVICE: &str = "Shanka";

pub fn default_api_key_ref() -> String {
    API_KEY_REF.to_string()
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub fn store_api_key(api_key: &str) -> Result<String, String> {
    api_key_entry()?
        .set_password(api_key)
        .map_err(|error| format!("failed to write API key to system keychain: {error}"))?;
    Ok(default_api_key_ref())
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub fn load_api_key(api_key_ref: &str) -> Result<Option<String>, String> {
    if api_key_ref.trim().is_empty() {
        return Ok(None);
    }

    match api_key_entry()?.get_password() {
        Ok(api_key) => Ok(Some(api_key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!(
            "failed to read API key from system keychain: {error}"
        )),
    }
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn api_key_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, API_KEY_ACCOUNT)
        .map_err(|error| format!("failed to open system keychain: {error}"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn store_api_key(_api_key: &str) -> Result<String, String> {
    Err("system keychain is not supported on this platform".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn load_api_key(_api_key_ref: &str) -> Result<Option<String>, String> {
    Err("system keychain is not supported on this platform".to_string())
}
