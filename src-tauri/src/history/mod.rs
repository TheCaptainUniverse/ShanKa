use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, time::SystemTime};

const HISTORY_FILE_NAME: &str = "history.json";
const HISTORY_LIMIT: usize = 50;
const HISTORY_TEXT_LIMIT: usize = 4_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteHistoryItem {
    pub id: u64,
    pub mode: String,
    pub original_text: String,
    pub result_text: String,
    pub persona_id: Option<String>,
    pub action: String,
    pub replaced: bool,
    pub created_at_ms: u64,
}

pub struct RewriteHistoryRecord<'a> {
    pub mode: &'static str,
    pub original_text: &'a str,
    pub result_text: &'a str,
    pub persona_id: Option<&'a str>,
    pub action: &'static str,
    pub replaced: bool,
}

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    let path = history_path(app).map_err(to_tauri_error)?;
    println!(
        "[history] local rewrite history ready at {}",
        path.display()
    );
    Ok(())
}

pub fn record(app: &tauri::AppHandle, record: RewriteHistoryRecord<'_>) -> Result<(), String> {
    let config = crate::config::load_or_create(app).map_err(|error| error.to_string())?;
    if !config.settings.history_enabled {
        return Ok(());
    }

    let mut history = load(app)?;
    let created_at_ms = current_time_millis();
    insert_history_item(&mut history, build_history_item(record, created_at_ms));
    save(app, &history)
}

pub fn load(app: &tauri::AppHandle) -> Result<Vec<RewriteHistoryItem>, String> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

pub fn clear(app: &tauri::AppHandle) -> Result<(), String> {
    save(app, &[])
}

fn save(app: &tauri::AppHandle, history: &[RewriteHistoryItem]) -> Result<(), String> {
    let path = history_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create history directory: {error}"))?;
    }

    let contents = serde_json::to_string_pretty(history)
        .map_err(|error| format!("failed to serialize rewrite history: {error}"))?;
    fs::write(&path, format!("{contents}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn history_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    crate::config::app_config_dir(app)
        .map(|directory| directory.join(HISTORY_FILE_NAME))
        .map_err(|error| error.to_string())
}

fn build_history_item(record: RewriteHistoryRecord<'_>, created_at_ms: u64) -> RewriteHistoryItem {
    RewriteHistoryItem {
        id: created_at_ms,
        mode: record.mode.to_string(),
        original_text: truncate_history_text(record.original_text),
        result_text: truncate_history_text(record.result_text),
        persona_id: record.persona_id.map(ToString::to_string),
        action: record.action.to_string(),
        replaced: record.replaced,
        created_at_ms,
    }
}

fn insert_history_item(history: &mut Vec<RewriteHistoryItem>, item: RewriteHistoryItem) {
    history.insert(0, item);
    history.truncate(HISTORY_LIMIT);
}

fn truncate_history_text(text: &str) -> String {
    text.chars().take(HISTORY_TEXT_LIMIT).collect()
}

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn to_tauri_error(error: String) -> tauri::Error {
    tauri::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_history_item_truncates_text_and_preserves_metadata() {
        let original_text = "a".repeat(HISTORY_TEXT_LIMIT + 10);
        let result_text = "b".repeat(HISTORY_TEXT_LIMIT + 20);

        let item = build_history_item(
            RewriteHistoryRecord {
                mode: "safe",
                original_text: &original_text,
                result_text: &result_text,
                persona_id: Some("clean-correction"),
                action: "copied",
                replaced: false,
            },
            42,
        );

        assert_eq!(item.id, 42);
        assert_eq!(item.created_at_ms, 42);
        assert_eq!(item.mode, "safe");
        assert_eq!(item.persona_id.as_deref(), Some("clean-correction"));
        assert_eq!(item.action, "copied");
        assert!(!item.replaced);
        assert_eq!(item.original_text.chars().count(), HISTORY_TEXT_LIMIT);
        assert_eq!(item.result_text.chars().count(), HISTORY_TEXT_LIMIT);
    }

    #[test]
    fn insert_history_item_keeps_newest_first_and_enforces_limit() {
        let mut history = Vec::new();

        for id in 0..(HISTORY_LIMIT as u64 + 5) {
            insert_history_item(
                &mut history,
                RewriteHistoryItem {
                    id,
                    mode: "magic".to_string(),
                    original_text: "original".to_string(),
                    result_text: "result".to_string(),
                    persona_id: None,
                    action: "replaced".to_string(),
                    replaced: true,
                    created_at_ms: id,
                },
            );
        }

        assert_eq!(history.len(), HISTORY_LIMIT);
        assert_eq!(
            history.first().map(|item| item.id),
            Some(HISTORY_LIMIT as u64 + 4)
        );
        assert_eq!(history.last().map(|item| item.id), Some(5));
    }
}
