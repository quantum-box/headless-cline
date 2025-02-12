use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResponse {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

pub type UserContent = Vec<ContentBlock>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text(TextBlock),
    #[serde(rename = "image")]
    Image(ImageBlock),
    #[serde(rename = "tool_use")]
    ToolUse(ToolUseBlock),
    #[serde(rename = "tool_result")]
    ToolResult(ToolResultBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBlock {
    pub source: ImageSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String, // "base64"
    pub media_type: String, // "image/jpeg" | "image/png" | "image/gif" | "image/webp"
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ContentBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl From<String> for ToolResponse {
    fn from(s: String) -> Self {
        ToolResponse::Text(s)
    }
}

impl From<&str> for ToolResponse {
    fn from(s: &str) -> Self {
        ToolResponse::Text(s.to_string())
    }
}
