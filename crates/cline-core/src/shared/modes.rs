use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub type Mode = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    pub slug: String,
    pub name: String,
    pub role_definition: String,
    pub custom_instructions: Option<String>,
}

// Mode-specific prompts only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptComponent {
    pub role_definition: Option<String>,
    pub custom_instructions: Option<String>,
}

pub type CustomModePrompts = std::collections::HashMap<String, PromptComponent>;

pub const default_mode_slug: &str = "code";

pub static modes: Lazy<Vec<ModeConfig>> = Lazy::new(|| {
    vec![
        ModeConfig {
            slug: "code".to_string(),
            name: "Code".to_string(),
            role_definition: "A general-purpose coding assistant".to_string(),
            custom_instructions: None,
        },
        ModeConfig {
            slug: "architect".to_string(),
            name: "Architect".to_string(),
            role_definition: "A software architect focused on high-level design".to_string(),
            custom_instructions: None,
        },
        ModeConfig {
            slug: "security".to_string(),
            name: "Security".to_string(),
            role_definition: "A security expert focused on identifying and fixing vulnerabilities"
                .to_string(),
            custom_instructions: None,
        },
    ]
});

pub fn get_mode_by_slug(mode: Mode, custom_modes: Option<&[ModeConfig]>) -> Option<&ModeConfig> {
    if let Some(custom_modes) = custom_modes {
        if let Some(mode_config) = custom_modes.iter().find(|m| m.slug == mode) {
            return Some(mode_config);
        }
    }
    modes.iter().find(|m| m.slug == mode)
}

pub fn get_role_definition(mode: &str, custom_modes: Option<&[ModeConfig]>) -> String {
    get_mode_by_slug(mode.to_string(), custom_modes)
        .map(|m| m.role_definition.clone())
        .unwrap_or_default()
}
