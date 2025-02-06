use crate::mcp::McpHub;
use crate::modes::{get_mode_by_slug, CustomModePrompts, Mode, ModeConfig, PromptComponent, MODES};
use crate::prompts::tools::get_tool_descriptions_for_mode;
use crate::sections::{
    add_custom_instructions, get_capabilities_section, get_mcp_servers_section, get_modes_section,
    get_objective_section, get_rules_section, get_shared_tool_use_section, get_system_info_section,
    get_tool_use_guidelines_section,
};
use crate::services::diff::DiffStrategy;
use std::collections::HashMap;

use std::path::Path;

use super::PreferredLanguage;

#[allow(clippy::too_many_arguments)]
pub async fn generate_prompt(
    context: &Path,
    cwd: &str,
    supports_computer_use: bool,
    mode: Mode,
    mcp_hub: Option<&McpHub>,
    diff_strategy: Option<&dyn DiffStrategy>,
    browser_viewport_size: Option<&str>,
    prompt_component: Option<&PromptComponent>,
    custom_mode_configs: Option<&[ModeConfig]>,
    global_custom_instructions: Option<&str>,
    preferred_language: Option<&str>,
    diff_enabled: Option<bool>,
    experiments: Option<&HashMap<String, bool>>,
    enable_mcp_server_creation: Option<bool>,
) -> Result<String, Box<dyn std::error::Error>> {
    if !context.exists() {
        return Err("Extension context is required for generating system prompt".into());
    }

    let effective_diff_strategy = if diff_enabled.unwrap_or(false) {
        diff_strategy
    } else {
        None
    };

    let (mcp_servers_section, modes_section): (String, String) = tokio::join!(
        get_mcp_servers_section(mcp_hub, effective_diff_strategy, enable_mcp_server_creation),
        get_modes_section(context)
    );

    let mode_config = get_mode_by_slug(mode.clone(), custom_mode_configs)
        .or_else(|| MODES.iter().find(|m| m.slug == mode))
        .unwrap_or(&MODES[0]);

    let role_definition = prompt_component
        .and_then(|pc| pc.role_definition.as_ref())
        .unwrap_or(&mode_config.role_definition);

    let base_prompt = format!(
        "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
        role_definition,
        get_shared_tool_use_section(),
        get_tool_descriptions_for_mode(
            mode.clone(),
            cwd.to_string(),
            supports_computer_use,
            effective_diff_strategy,
            browser_viewport_size.map(|s| s.to_string()),
            mcp_hub,
            custom_mode_configs,
            experiments
        ),
        get_tool_use_guidelines_section(),
        mcp_servers_section,
        get_capabilities_section(cwd, supports_computer_use, mcp_hub, effective_diff_strategy),
        modes_section,
        get_rules_section(
            cwd,
            supports_computer_use,
            effective_diff_strategy,
            experiments
        ),
        get_system_info_section(cwd, mode.clone(), custom_mode_configs),
        get_objective_section()
    );

    let empty_string = String::new();
    let custom_instructions = prompt_component
        .and_then(|pc| pc.custom_instructions.as_ref())
        .or(mode_config.custom_instructions.as_ref())
        .unwrap_or(&empty_string);

    let final_prompt = add_custom_instructions(
        custom_instructions,
        global_custom_instructions.unwrap_or(""),
        cwd,
        &mode,
        PreferredLanguage {
            preferred_language: preferred_language.map(|s| s.to_string()),
        },
    )
    .await?;

    Ok(format!("{}\n\n{}", base_prompt, final_prompt))
}

#[allow(clippy::too_many_arguments)]
pub async fn system_prompt(
    context: &Path,
    cwd: &str,
    supports_computer_use: bool,
    mcp_hub: Option<&McpHub>,
    diff_strategy: Option<&dyn DiffStrategy>,
    browser_viewport_size: Option<&str>,
    mode: Option<Mode>,
    custom_mode_prompts: Option<&CustomModePrompts>,
    custom_modes: Option<&[ModeConfig]>,
    global_custom_instructions: Option<&str>,
    preferred_language: Option<&str>,
    diff_enabled: Option<bool>,
    experiments: Option<&HashMap<String, bool>>,
    enable_mcp_server_creation: Option<bool>,
) -> Result<String, Box<dyn std::error::Error>> {
    if !context.exists() {
        return Err("Extension context is required for generating system prompt".into());
    }

    fn get_prompt_component<'a>(
        value: &'a Option<&'a CustomModePrompts>,
        mode: &'a Mode,
    ) -> Option<&'a PromptComponent> {
        value.and_then(|prompts| prompts.get(mode))
    }

    let mode_str = mode.as_deref().unwrap_or("");
    let current_mode = get_mode_by_slug(mode_str.to_string(), custom_modes)
        .or_else(|| MODES.iter().find(|m| m.slug == mode_str))
        .unwrap_or(&MODES[0]);

    let prompt_component = get_prompt_component(&custom_mode_prompts, &current_mode.slug);

    let effective_diff_strategy = if diff_enabled.unwrap_or(false) {
        diff_strategy
    } else {
        None
    };

    generate_prompt(
        context,
        cwd,
        supports_computer_use,
        current_mode.slug.clone(),
        mcp_hub,
        effective_diff_strategy,
        browser_viewport_size,
        prompt_component,
        custom_modes,
        global_custom_instructions,
        preferred_language,
        diff_enabled,
        experiments,
        enable_mcp_server_creation,
    )
    .await
}
