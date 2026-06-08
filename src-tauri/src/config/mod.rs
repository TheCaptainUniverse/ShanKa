use global_hotkey::hotkey::HotKey;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::Manager;

use crate::persona;

const CONFIG_FILE_NAME: &str = "config.json";
const CONFIG_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub schema_version: u16,
    pub hotkeys: HotkeyConfig,
    pub settings: AppSettingsConfig,
    pub personas: persona::PersonaConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            hotkeys: HotkeyConfig::default(),
            settings: AppSettingsConfig::default(),
            personas: persona::PersonaConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub safe_mode: String,
    pub magic_mode: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            safe_mode: default_safe_mode_hotkey(),
            magic_mode: default_magic_mode_hotkey(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedHotkeyConfig {
    pub safe_mode: HotKey,
    pub magic_mode: HotKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettingsConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout_ms: u64,
}

impl Default for AppSettingsConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: String::new(),
            timeout_ms: 8_000,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(String),
    Json(String),
    InvalidHotkey {
        field: &'static str,
        message: String,
    },
    DuplicateHotkeys(String),
    InvalidSettings(String),
    InvalidPersonas(String),
    Path(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "config I/O error: {message}"),
            Self::Json(message) => write!(formatter, "config JSON error: {message}"),
            Self::InvalidHotkey { field, message } => {
                write!(formatter, "invalid {field} hotkey: {message}")
            }
            Self::DuplicateHotkeys(hotkey) => {
                write!(
                    formatter,
                    "safe and magic hotkeys must be different: {hotkey}"
                )
            }
            Self::InvalidSettings(message) => write!(formatter, "invalid app settings: {message}"),
            Self::InvalidPersonas(message) => write!(formatter, "invalid personas: {message}"),
            Self::Path(message) => write!(formatter, "config path error: {message}"),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    let config = load_or_create(app).map_err(to_tauri_error)?;
    let path = config_path(app).map_err(to_tauri_error)?;
    println!(
        "[config] app config ready at {}; Safe Mode={}, Magic Mode={}, Base URL={}, Model={}, Default Persona={}",
        path.display(),
        config.hotkeys.safe_mode,
        config.hotkeys.magic_mode,
        config.settings.base_url,
        if config.settings.model.is_empty() {
            "<mock>"
        } else {
            &config.settings.model
        },
        config.personas.default_safe_persona_id
    );
    Ok(())
}

pub fn load_or_create(app: &tauri::AppHandle) -> Result<AppConfig, ConfigError> {
    let path = config_path(app)?;

    if !path.exists() {
        let config = AppConfig::default();
        save_app_config_at(&path, &config)?;
        return Ok(config);
    }

    match load_app_config_at(&path) {
        Ok(mut config) => {
            let normalized_personas = persona::normalize_config(&config.personas);
            if normalized_personas != config.personas {
                config.personas = normalized_personas;
                save_app_config_at(&path, &config)?;
            }
            Ok(config)
        }
        Err(error) => {
            println!("[config] failed to read config; rewriting defaults: {error}");
            let config = AppConfig::default();
            save_app_config_at(&path, &config)?;
            Ok(config)
        }
    }
}

pub fn save_hotkeys(
    app: &tauri::AppHandle,
    hotkeys: HotkeyConfig,
) -> Result<HotkeyConfig, ConfigError> {
    hotkeys.resolve()?;

    let mut config = load_or_create(app)?;
    config.hotkeys = hotkeys;
    save_app_config_at(&config_path(app)?, &config)?;
    Ok(config.hotkeys)
}

pub fn save_settings(
    app: &tauri::AppHandle,
    settings: AppSettingsConfig,
) -> Result<AppSettingsConfig, ConfigError> {
    let settings = settings.normalized()?;
    let mut config = load_or_create(app)?;
    config.settings = settings;
    save_app_config_at(&config_path(app)?, &config)?;
    Ok(config.settings)
}

pub fn save_personas(
    app: &tauri::AppHandle,
    personas: persona::PersonaConfig,
) -> Result<persona::PersonaConfig, ConfigError> {
    let personas = normalized_personas(&personas)?;
    let mut config = load_or_create(app)?;
    config.personas = personas;
    save_app_config_at(&config_path(app)?, &config)?;
    Ok(config.personas)
}

impl HotkeyConfig {
    pub fn resolve(&self) -> Result<ResolvedHotkeyConfig, ConfigError> {
        let safe_mode = parse_hotkey("safe_mode", &self.safe_mode)?;
        let magic_mode = parse_hotkey("magic_mode", &self.magic_mode)?;

        if safe_mode.id() == magic_mode.id() {
            return Err(ConfigError::DuplicateHotkeys(self.safe_mode.clone()));
        }

        Ok(ResolvedHotkeyConfig {
            safe_mode,
            magic_mode,
        })
    }
}

impl AppSettingsConfig {
    pub fn normalized(&self) -> Result<Self, ConfigError> {
        let base_url = self.base_url.trim().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(ConfigError::InvalidSettings(
                "base_url cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            api_key: self.api_key.trim().to_string(),
            base_url,
            model: self.model.trim().to_string(),
            timeout_ms: self.timeout_ms.clamp(1_000, 120_000),
        })
    }

    pub fn can_use_remote_provider(&self) -> bool {
        !self.api_key.trim().is_empty()
            && !self.base_url.trim().is_empty()
            && !self.model.trim().is_empty()
    }
}

fn normalized_personas(
    personas: &persona::PersonaConfig,
) -> Result<persona::PersonaConfig, ConfigError> {
    let normalized = persona::normalize_config(personas);

    if normalized.items.is_empty() {
        return Err(ConfigError::InvalidPersonas(
            "at least one enabled persona is required".to_string(),
        ));
    }

    Ok(normalized)
}

pub fn default_hotkeys() -> HotkeyConfig {
    HotkeyConfig::default()
}

fn parse_hotkey(field: &'static str, hotkey: &str) -> Result<HotKey, ConfigError> {
    hotkey
        .parse::<HotKey>()
        .map_err(|error| ConfigError::InvalidHotkey {
            field,
            message: error.to_string(),
        })
}

fn load_app_config_at(path: &PathBuf) -> Result<AppConfig, ConfigError> {
    let contents = fs::read_to_string(path)
        .map_err(|error| ConfigError::Io(format!("failed to read {}: {error}", path.display())))?;
    serde_json::from_str(&contents)
        .map_err(|error| ConfigError::Json(format!("failed to parse {}: {error}", path.display())))
}

fn save_app_config_at(path: &PathBuf, config: &AppConfig) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ConfigError::Io(format!(
                "failed to create config directory {}: {error}",
                parent.display()
            ))
        })?;
    }

    let mut config = config.clone();
    config.personas = persona::normalize_config(&config.personas);

    let contents = serde_json::to_string_pretty(&config)
        .map_err(|error| ConfigError::Json(format!("failed to serialize config: {error}")))?;
    fs::write(path, format!("{contents}\n"))
        .map_err(|error| ConfigError::Io(format!("failed to write {}: {error}", path.display())))
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, ConfigError> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(CONFIG_FILE_NAME))
        .map_err(|error| ConfigError::Path(error.to_string()))
}

#[cfg(target_os = "macos")]
fn default_safe_mode_hotkey() -> String {
    "Cmd+Shift+KeyC".to_string()
}

#[cfg(not(target_os = "macos"))]
fn default_safe_mode_hotkey() -> String {
    "Ctrl+Shift+KeyC".to_string()
}

#[cfg(target_os = "macos")]
fn default_magic_mode_hotkey() -> String {
    "Cmd+Shift+Space".to_string()
}

#[cfg(not(target_os = "macos"))]
fn default_magic_mode_hotkey() -> String {
    "Ctrl+Shift+Space".to_string()
}

fn to_tauri_error(error: ConfigError) -> tauri::Error {
    tauri::Error::Io(std::io::Error::new(
        std::io::ErrorKind::Other,
        error.to_string(),
    ))
}
