use crate::diff::DiffStrategy;
use crate::services::mcp::McpHub;

#[derive(Debug, Default)]
pub struct ToolArgs<'a> {
    pub cwd: String,
    pub supports_computer_use: bool,
    pub diff_strategy: Option<&'a Box<dyn DiffStrategy>>,
    pub browser_viewport_size: Option<String>,
    pub mcp_hub: Option<&'a Box<McpHub>>,
    pub tool_options: Option<serde_json::Value>,
}
