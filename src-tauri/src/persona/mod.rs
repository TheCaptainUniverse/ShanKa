use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::OnceLock};

const PERSONA_CATALOG_JSON: &str = include_str!("../../../shared/personas.json");
const FALLBACK_SAFE_PERSONA_ID: &str = "clean-correction";

static PERSONA_CATALOG: OnceLock<PersonaCatalog> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PersonaConfig {
    #[serde(alias = "default_safe_persona_id")]
    pub default_safe_persona_id: String,
    pub items: Vec<PersonaConfigItem>,
}

impl Default for PersonaConfig {
    fn default() -> Self {
        let catalog = persona_catalog();
        Self {
            default_safe_persona_id: catalog.default_safe_persona_id.clone(),
            items: catalog.personas.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PersonaConfigItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub name_key: String,
    pub description_key: String,
    pub system_prompt: String,
    pub built_in: bool,
    pub enabled: bool,
}

impl Default for PersonaConfigItem {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            name_key: String::new(),
            description_key: String::new(),
            system_prompt: String::new(),
            built_in: false,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedPersona {
    pub name: String,
    pub system_prompt: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersonaCatalog {
    default_safe_persona_id: String,
    personas: Vec<PersonaConfigItem>,
}

pub fn default_safe_persona_id() -> &'static str {
    persona_catalog().default_safe_persona_id.as_str()
}

pub fn normalize_config(config: &PersonaConfig) -> PersonaConfig {
    let mut normalized = config.clone();
    normalized.items = merge_built_in_personas(&normalized.items);
    ensure_enabled_persona(&mut normalized.items);

    if !has_enabled_persona(&normalized.items, &normalized.default_safe_persona_id) {
        normalized.default_safe_persona_id = default_safe_persona_id().to_string();
    }
    if !has_enabled_persona(&normalized.items, &normalized.default_safe_persona_id) {
        normalized.default_safe_persona_id = normalized
            .items
            .iter()
            .find(|persona| persona.enabled)
            .map(|persona| persona.id.clone())
            .unwrap_or_else(|| FALLBACK_SAFE_PERSONA_ID.to_string());
    }

    normalized
}

pub fn resolve_persona(
    config: &PersonaConfig,
    persona_id: Option<&str>,
) -> Option<ResolvedPersona> {
    let normalized = normalize_config(config);
    let target_id = persona_id.unwrap_or(normalized.default_safe_persona_id.as_str());

    normalized
        .items
        .iter()
        .filter(|persona| persona.enabled)
        .find(|persona| persona.id == target_id)
        .or_else(|| {
            normalized
                .items
                .iter()
                .filter(|persona| persona.enabled)
                .find(|persona| persona.id == normalized.default_safe_persona_id)
        })
        .or_else(|| normalized.items.iter().find(|persona| persona.enabled))
        .map(|persona| ResolvedPersona {
            name: persona.name.clone(),
            system_prompt: persona.system_prompt.clone(),
        })
}

fn merge_built_in_personas(items: &[PersonaConfigItem]) -> Vec<PersonaConfigItem> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for built_in in &persona_catalog().personas {
        let mut persona = built_in.clone();
        if let Some(configured) = items.iter().find(|item| item.id == built_in.id) {
            persona.enabled = configured.enabled;
        }
        seen.insert(persona.id.clone());
        merged.push(persona);
    }

    for persona in items {
        if !is_valid_persona(persona) || seen.contains(&persona.id) {
            continue;
        }

        let mut custom_persona = persona.clone();
        custom_persona.built_in = false;
        seen.insert(custom_persona.id.clone());
        merged.push(custom_persona);
    }

    merged
}

fn is_valid_persona(persona: &PersonaConfigItem) -> bool {
    !persona.id.trim().is_empty()
        && !persona.name.trim().is_empty()
        && !persona.system_prompt.trim().is_empty()
}

fn ensure_enabled_persona(items: &mut [PersonaConfigItem]) {
    if items.iter().any(|persona| persona.enabled) {
        return;
    }

    let fallback_index = items
        .iter()
        .position(|persona| persona.id == FALLBACK_SAFE_PERSONA_ID)
        .or_else(|| (!items.is_empty()).then_some(0));

    if let Some(index) = fallback_index {
        items[index].enabled = true;
    }
}

fn has_enabled_persona(items: &[PersonaConfigItem], persona_id: &str) -> bool {
    items
        .iter()
        .any(|persona| persona.enabled && persona.id == persona_id)
}

fn persona_catalog() -> &'static PersonaCatalog {
    PERSONA_CATALOG.get_or_init(|| {
        serde_json::from_str(PERSONA_CATALOG_JSON)
            .expect("shared/personas.json must be valid persona catalog JSON")
    })
}

#[cfg(test)]
mod tests {
    use super::{normalize_config, resolve_persona, PersonaConfig, PersonaConfigItem};

    #[test]
    fn normalize_config_keeps_built_in_prompt_authoritative() {
        let config = PersonaConfig {
            items: vec![PersonaConfigItem {
                id: "clean-correction".to_string(),
                name: "Changed".to_string(),
                system_prompt: "Changed prompt".to_string(),
                enabled: false,
                ..PersonaConfigItem::default()
            }],
            ..PersonaConfig::default()
        };

        let normalized = normalize_config(&config);
        let clean = normalized
            .items
            .iter()
            .find(|persona| persona.id == "clean-correction")
            .expect("built-in clean correction persona should exist");

        assert_eq!(clean.name, "Clean Correction");
        assert_ne!(clean.system_prompt, "Changed prompt");
        assert!(!clean.enabled);
    }

    #[test]
    fn normalize_config_falls_back_when_default_is_disabled() {
        let config = PersonaConfig {
            default_safe_persona_id: "clean-correction".to_string(),
            items: vec![PersonaConfigItem {
                id: "clean-correction".to_string(),
                name: "Clean Correction".to_string(),
                system_prompt: "Prompt".to_string(),
                enabled: false,
                built_in: true,
                ..PersonaConfigItem::default()
            }],
        };

        let normalized = normalize_config(&config);

        assert_ne!(normalized.default_safe_persona_id, "clean-correction");
        assert!(normalized
            .items
            .iter()
            .any(|persona| persona.id == normalized.default_safe_persona_id && persona.enabled));
    }

    #[test]
    fn resolve_persona_can_use_custom_persona() {
        let config = PersonaConfig {
            items: vec![PersonaConfigItem {
                id: "custom-friendly".to_string(),
                name: "Friendly".to_string(),
                system_prompt: "Make it friendlier.".to_string(),
                enabled: true,
                ..PersonaConfigItem::default()
            }],
            ..PersonaConfig::default()
        };

        let resolved = resolve_persona(&config, Some("custom-friendly"))
            .expect("custom persona should resolve");

        assert_eq!(resolved.name, "Friendly");
        assert_eq!(resolved.system_prompt, "Make it friendlier.");
    }
}
