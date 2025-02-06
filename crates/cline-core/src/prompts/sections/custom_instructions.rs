use std::path::Path;
use tokio::fs;

#[allow(dead_code)]
pub struct PreferredLanguage {
    pub preferred_language: Option<String>,
}

#[allow(dead_code)]
pub async fn add_custom_instructions(
    mode_custom_instructions: &str,
    global_custom_instructions: &str,
    cwd: &str,
    mode: &str,
    options: PreferredLanguage,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut sections = Vec::new();

    // Load mode-specific rules if mode is provided
    let mut mode_rule_content = String::new();
    if !mode.is_empty() {
        let mode_rule_file = format!(".clinerules-{}", mode);
        let mode_rule_path = Path::new(cwd).join(&mode_rule_file);
        if let Ok(content) = fs::read_to_string(&mode_rule_path).await {
            if !content.trim().is_empty() {
                mode_rule_content = content.trim().to_string();
            }
        }
    }

    // Add language preference if provided
    if let Some(lang) = &options.preferred_language {
        sections.push(format!(
            "Language Preference:\nYou should always speak and think in the {} language.",
            lang
        ));
    }

    // Add global instructions first
    if !global_custom_instructions.trim().is_empty() {
        sections.push(format!(
            "Global Instructions:\n{}",
            global_custom_instructions.trim()
        ));
    }

    // Add mode-specific instructions after
    if !mode_custom_instructions.trim().is_empty() {
        sections.push(format!(
            "Mode-specific Instructions:\n{}",
            mode_custom_instructions.trim()
        ));
    }

    // Add rules - include both mode-specific and generic rules if they exist
    let mut rules = Vec::new();

    // Add mode-specific rules first if they exist
    if !mode_rule_content.is_empty() {
        let mode_rule_file = format!(".clinerules-{}", mode);
        rules.push(format!(
            "# Rules from {}:\n{}",
            mode_rule_file, mode_rule_content
        ));
    }

    // Add generic rules
    let generic_rule_content = load_rule_files(cwd).await?;
    if !generic_rule_content.trim().is_empty() {
        rules.push(generic_rule_content.trim().to_string());
    }

    if !rules.is_empty() {
        sections.push(format!("Rules:\n\n{}", rules.join("\n\n")));
    }

    let joined_sections = sections.join("\n\n");

    Ok(if !joined_sections.is_empty() {
        format!(
            "\n====\n\nUSER'S CUSTOM INSTRUCTIONS\n\nThe following additional instructions are provided by the user, and should be followed to the best of your ability without interfering with the TOOL USE guidelines.\n\n{}",
            joined_sections
        )
    } else {
        String::new()
    })
}

#[allow(dead_code)]
async fn load_rule_files(cwd: &str) -> Result<String, Box<dyn std::error::Error>> {
    let rule_files = vec![".clinerules", ".cursorrules", ".windsurfrules"];
    let mut combined_rules = String::new();

    for file in rule_files {
        let rule_path = Path::new(cwd).join(file);
        if let Ok(content) = fs::read_to_string(&rule_path).await {
            if !content.trim().is_empty() {
                combined_rules.push_str(&format!("\n# Rules from {}:\n{}\n", file, content.trim()));
            }
        }
    }

    Ok(combined_rules)
}
