use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageModelChatSelector {
    pub vendor: Option<String>,
    pub family: Option<String>,
    pub version: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionMessage {
    #[serde(rename = "type")]
    pub message_type: ExtensionMessageType,
    pub text: Option<String>,
    pub action: Option<ExtensionAction>,
    pub invoke: Option<ExtensionInvoke>,
    pub state: Option<ExtensionState>,
    pub images: Option<Vec<String>>,
    pub ollama_models: Option<Vec<String>>,
    pub lm_studio_models: Option<Vec<String>>,
    pub vs_code_lm_models: Option<Vec<LanguageModelChatSelector>>,
    pub file_paths: Option<Vec<String>>,
    pub opened_tabs: Option<Vec<OpenedTab>>,
    pub partial_message: Option<ClineMessage>,
    pub glama_models: Option<HashMap<String, ModelInfo>>,
    pub open_router_models: Option<HashMap<String, ModelInfo>>,
    pub open_ai_models: Option<Vec<String>>,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub commits: Option<Vec<GitCommit>>,
    pub list_api_config: Option<Vec<ApiConfigMeta>>,
    pub mode: Option<Mode>,
    pub custom_mode: Option<ModeConfig>,
    pub slug: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionMessageType {
    Action,
    State,
    SelectedImages,
    OllamaModels,
    LmStudioModels,
    Theme,
    WorkspaceUpdated,
    Invoke,
    PartialMessage,
    GlamaModels,
    OpenRouterModels,
    OpenAiModels,
    McpServers,
    EnhancedPrompt,
    CommitSearchResults,
    ListApiConfig,
    VsCodeLmModels,
    VsCodeLmApiAvailable,
    RequestVsCodeLmModels,
    UpdatePrompt,
    SystemPrompt,
    AutoApprovalEnabled,
    UpdateCustomMode,
    DeleteCustomMode,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionAction {
    ChatButtonClicked,
    McpButtonClicked,
    SettingsButtonClicked,
    HistoryButtonClicked,
    PromptsButtonClicked,
    DidBecomeVisible,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionInvoke {
    SendMessage,
    PrimaryButtonClick,
    SecondaryButtonClick,
    SetChatBoxMessage,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedTab {
    pub label: String,
    pub is_active: bool,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    // ModelInfoの具体的なフィールドは必要に応じて追加
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub name: String,
    pub config: String,
    #[serde(rename = "status")]
    pub server_status: McpServerStatus,
    pub error: Option<String>,
    pub tools: Option<Vec<McpTool>>,
    pub resources: Option<Vec<McpResource>>,
    pub resource_templates: Option<Vec<McpResourceTemplate>>,
    pub disabled: Option<bool>,
    pub timeout: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServerStatus {
    Connected,
    Connecting,
    Disconnected,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
    pub always_allow: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceTemplate {
    pub uri_template: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceResponse {
    pub _meta: Option<HashMap<String, serde_json::Value>>,
    pub contents: Vec<McpResourceContent>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallResponse {
    pub _meta: Option<HashMap<String, serde_json::Value>>,
    pub content: Vec<McpToolCallResponseContent>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum McpToolCallResponseContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: McpResourceContent },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommit {
    // GitCommitの具体的なフィールドは必要に応じて追加
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfigMeta {
    pub id: String,
    pub name: String,
    pub api_provider: Option<ApiProvider>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiProvider {
    // ApiProviderの具体的なフィールドは必要に応じて追加
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    // Modeの具体的な列挙値は必要に応じて追加
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeConfig {
    // ModeConfigの具体的なフィールドは必要に応じて追加
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineMessage {
    pub ts: i64,
    #[serde(rename = "type")]
    pub message_type: ClineMessageType,
    pub ask: Option<ClineAsk>,
    pub say: Option<ClineSay>,
    pub text: Option<String>,
    pub images: Option<Vec<String>>,
    pub partial: Option<bool>,
    pub reasoning: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineMessageType {
    Ask,
    Say,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineAsk {
    Followup,
    Command,
    CommandOutput,
    CompletionResult,
    Tool,
    ApiReqFailed,
    ResumeTask,
    ResumeCompletedTask,
    MistakeLimitReached,
    BrowserActionLaunch,
    UseMcpServer,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineSay {
    Task,
    Error,
    ApiReqStarted,
    ApiReqFinished,
    Text,
    Reasoning,
    CompletionResult,
    UserFeedback,
    UserFeedbackDiff,
    ApiReqRetried,
    ApiReqRetryDelayed,
    CommandOutput,
    Tool,
    ShellIntegrationWarning,
    BrowserAction,
    BrowserActionResult,
    Command,
    McpServerRequestStarted,
    McpServerResponse,
    NewTaskStarted,
    NewTask,
}

#[allow(dead_code)]
pub const BROWSER_ACTIONS: [&str; 6] = [
    "launch",
    "click",
    "type",
    "scroll_down",
    "scroll_up",
    "close",
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    Launch,
    Click,
    Type,
    ScrollDown,
    ScrollUp,
    Close,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineSayBrowserAction {
    pub action: BrowserAction,
    pub coordinate: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserActionResult {
    pub screenshot: Option<String>,
    pub logs: Option<String>,
    pub current_url: Option<String>,
    pub current_mouse_position: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineAskUseMcpServer {
    pub server_name: String,
    #[serde(rename = "type")]
    pub action_type: ClineAskUseMcpServerType,
    pub tool_name: Option<String>,
    pub arguments: Option<String>,
    pub uri: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineAskUseMcpServerType {
    UseMcpTool,
    AccessMcpResource,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineApiReqInfo {
    pub request: Option<String>,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub cache_writes: Option<i32>,
    pub cache_reads: Option<i32>,
    pub cost: Option<f64>,
    pub cancel_reason: Option<ClineApiReqCancelReason>,
    pub streaming_failed_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineApiReqCancelReason {
    StreamingFailed,
    UserCancelled,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionState {
    pub version: String,
    pub cline_messages: Vec<ClineMessage>,
    pub task_history: Vec<HistoryItem>,
    pub should_show_announcement: bool,
    pub api_configuration: Option<ApiConfiguration>,
    pub current_api_config_name: Option<String>,
    pub list_api_config_meta: Option<Vec<ApiConfigMeta>>,
    pub custom_instructions: Option<String>,
    pub custom_mode_prompts: Option<CustomModePrompts>,
    pub custom_support_prompts: Option<CustomSupportPrompts>,
    pub always_allow_read_only: Option<bool>,
    pub always_allow_write: Option<bool>,
    pub always_allow_execute: Option<bool>,
    pub always_allow_browser: Option<bool>,
    pub always_allow_mcp: Option<bool>,
    pub always_approve_resubmit: Option<bool>,
    pub always_allow_mode_switch: Option<bool>,
    pub request_delay_seconds: i32,
    pub rate_limit_seconds: i32,
    pub uri_scheme: Option<String>,
    pub allowed_commands: Option<Vec<String>>,
    pub sound_enabled: Option<bool>,
    pub sound_volume: Option<f32>,
    pub diff_enabled: Option<bool>,
    pub browser_viewport_size: Option<String>,
    pub screenshot_quality: Option<i32>,
    pub fuzzy_match_threshold: Option<f32>,
    pub preferred_language: String,
    pub write_delay_ms: i32,
    pub terminal_output_line_limit: Option<i32>,
    pub mcp_enabled: bool,
    pub enable_mcp_server_creation: bool,
    pub mode: Mode,
    pub mode_api_configs: Option<HashMap<Mode, String>>,
    pub enhancement_api_config_id: Option<String>,
    pub experiments: HashMap<String, bool>,
    pub auto_approval_enabled: Option<bool>,
    pub custom_modes: Vec<ModeConfig>,
    pub tool_requirements: Option<HashMap<String, bool>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub ts: i64,
    pub task: String,
    pub tokens_in: i32,
    pub tokens_out: i32,
    pub cache_writes: Option<i32>,
    pub cache_reads: Option<i32>,
    pub total_cost: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfiguration {
    // ApiConfigurationの具体的なフィールドは必要に応じて追加
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModePrompts {
    // CustomModePromptsの具体的なフィールドは必要に応じて追加
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomSupportPrompts {
    // CustomSupportPromptsの具体的なフィールドは必要に応じて追加
}
