mod access_mcp_resource;
mod ask_followup_question;
mod attempt_completion;
mod browser_action;
mod execute_command;
mod insert_content;
mod list_code_definition_names;
mod list_files;
mod new_task;
mod read_file;
mod search_and_replace;
mod search_files;
mod switch_mode;
pub mod types;
pub mod use_mcp_tool;
pub mod write_to_file;

pub use access_mcp_resource::get_access_mcp_resource_description;
pub use ask_followup_question::get_ask_followup_question_description;
pub use attempt_completion::get_attempt_completion_description;
pub use browser_action::get_browser_action_description;
pub use execute_command::get_execute_command_description;
pub use insert_content::get_insert_content_description;
pub use list_code_definition_names::get_list_code_definition_names_description;
pub use list_files::get_list_files_description;
pub use new_task::get_new_task_description;
pub use read_file::get_read_file_description;
pub use search_and_replace::get_search_and_replace_description;
pub use search_files::get_search_files_description;
pub use switch_mode::get_switch_mode_description;

pub use use_mcp_tool::get_use_mcp_tool_description;
pub use write_to_file::get_write_to_file_description;

use crate::services::mcp::McpHub;
use crate::shared::modes::{Mode, ModeConfig};
use crate::services::diff::{DiffStrategy, types::ToolArgs};

#[allow(clippy::too_many_arguments)]
pub fn get_tool_descriptions_for_mode(
    _mode: Mode,
    cwd: String,
    supports_computer_use: bool,
    diff_strategy: Option<&dyn DiffStrategy>,
    browser_viewport_size: Option<String>,
    mcp_hub: Option<&McpHub>,
    _custom_modes: Option<&[ModeConfig]>,
    _experiments: Option<&std::collections::HashMap<String, bool>>,
) -> String {
    let args = ToolArgs {
        cwd,
        supports_computer_use,
        diff_strategy,
        browser_viewport_size,
        mcp_hub,
        tool_options: None,
    };

    let mut descriptions = Vec::new();

    // Add descriptions for all tools
    if let Some(desc) = get_execute_command_description(&args) {
        descriptions.push(desc);
    }
    descriptions.push(get_read_file_description(&args));
    descriptions.push(get_write_to_file_description(&args));
    descriptions.push(get_search_files_description(&args));
    descriptions.push(get_list_files_description(&args));
    descriptions.push(get_list_code_definition_names_description(&args));
    if let Some(desc) = get_browser_action_description(&args) {
        descriptions.push(desc);
    }
    descriptions.push(get_ask_followup_question_description(&args));
    descriptions.push(get_attempt_completion_description(&args));
    if let Some(desc) = get_use_mcp_tool_description(&args) {
        descriptions.push(desc);
    }
    if let Some(desc) = get_access_mcp_resource_description(&args) {
        descriptions.push(desc);
    }
    descriptions.push(get_switch_mode_description(&args));
    descriptions.push(get_new_task_description(&args));
    descriptions.push(get_insert_content_description(&args));
    descriptions.push(get_search_and_replace_description(&args));

    format!("# Tools\n\n{}", descriptions.join("\n\n"))
}
