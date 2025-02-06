use crate::modes::{Mode, ModeConfig, default_mode_slug, get_mode_by_slug};
use std::path::Path;

pub async fn get_modes_section(context: &Path) -> String {
    let settings_dir = context.join("settings");
    let custom_modes_path = settings_dir.join("cline_custom_modes.json");

    format!(
        "====\n\nMODES\n\n- When referring to modes, always use their display names. The built-in modes are:\n  * \"Code\" mode - A general-purpose coding assistant\n  * \"Architect\" mode - A software architect focused on high-level design\n  * \"Security\" mode - A security expert focused on identifying and fixing vulnerabilities\n  Custom modes will be referred to by their configured name property.\n\n- Custom modes can be configured by editing the custom modes file at '{}'. The file gets created automatically on startup and should always exist. Make sure to read the latest contents before writing to it to avoid overwriting existing modes.\n\n- The following fields are required and must not be empty:\n  * slug: A valid slug (lowercase letters, numbers, and hyphens). Must be unique, and shorter is better.\n  * name: The display name for the mode\n  * roleDefinition: A detailed description of the mode's role and capabilities\n  * groups: Array of allowed tool groups (can be empty). Each group can be specified either as a string (e.g., \"edit\" to allow editing any file) or with file restrictions (e.g., [\"edit\", {{ fileRegex: \"\\.md$\", description: \"Markdown files only\" }}] to only allow editing markdown files)\n\n- The customInstructions field is optional.\n\n- For multi-line text, include newline characters in the string like \"This is the first line.\\nThis is the next line.\\n\\nThis is a double line break.\"\n\nThe file should follow this structure:\n{{\n \"customModes\": [\n   {{\n     \"slug\": \"designer\", // Required: unique slug with lowercase letters, numbers, and hyphens\n     \"name\": \"Designer\", // Required: mode display name\n     \"roleDefinition\": \"You are Roo, a UI/UX expert specializing in design systems and frontend development. Your expertise includes:\\n- Creating and maintaining design systems\\n- Implementing responsive and accessible web interfaces\\n- Working with CSS, HTML, and modern frontend frameworks\\n- Ensuring consistent user experiences across platforms\", // Required: non-empty\n     \"groups\": [ // Required: array of tool groups (can be empty)\n       \"read\",    // Read files group (read_file, search_files, list_files, list_code_definition_names)\n       \"edit\",    // Edit files group (write_to_file, apply_diff) - allows editing any file\n       // Or with file restrictions:\n       // [\"edit\", {{ fileRegex: \"\\.md$\", description: \"Markdown files only\" }}],  // Edit group that only allows editing markdown files\n       \"browser\", // Browser group (browser_action)\n       \"command\", // Command group (execute_command)\n       \"mcp\"     // MCP group (use_mcp_tool, access_mcp_resource)\n     ],\n     \"customInstructions\": \"Additional instructions for the Designer mode\" // Optional\n    }}\n  ]\n}}",
        custom_modes_path.display()
    )
}
