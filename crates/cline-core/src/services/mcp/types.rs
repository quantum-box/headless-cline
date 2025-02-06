use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub name: String,
    pub config: String,
    pub status: McpServerStatus,
    pub error: Option<String>,
    pub disabled: Option<bool>,
    pub tools: Option<Vec<McpTool>>,
    pub resources: Option<Vec<McpResource>>,
    pub resource_templates: Option<Vec<McpResourceTemplate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpServerStatus {
    #[serde(rename = "connecting")]
    Connecting,
    #[serde(rename = "connected")]
    Connected,
    #[serde(rename = "disconnected")]
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub always_allow: bool,
    pub input_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceTemplate {
    pub uri_template: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallResponse {
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceResponse {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioConfig {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub always_allow: Option<Vec<String>>,
    pub disabled: Option<bool>,
    pub timeout: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSettings {
    pub mcp_servers: HashMap<String, StdioConfig>,
}
