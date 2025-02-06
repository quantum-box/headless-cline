use crate::services::diff::DiffStrategy;
use crate::services::mcp::McpHub;
use std::fmt;

#[derive(Default)]
pub struct ToolArgs<'a> {
    pub cwd: String,
    pub supports_computer_use: bool,
    pub diff_strategy: Option<&'a dyn DiffStrategy>,
    pub browser_viewport_size: Option<String>,
    pub mcp_hub: Option<&'a McpHub>,
    pub tool_options: Option<serde_json::Value>,
}

impl fmt::Debug for ToolArgs<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolArgs")
            .field("cwd", &self.cwd)
            .field("supports_computer_use", &self.supports_computer_use)
            .field("diff_strategy", &"<DiffStrategy>")
            .field("browser_viewport_size", &self.browser_viewport_size)
            .field("mcp_hub", &self.mcp_hub)
            .field("tool_options", &self.tool_options)
            .finish()
    }
}
