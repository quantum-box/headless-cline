use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use futures_util::future::join_all;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use uuid::Uuid;

use crate::mentions::{parse_mentions, should_process_mentions};
use crate::services::anthropic::{AnthropicClient, Message};
use crate::services::browser::BrowserSession;
use crate::services::terminal::TerminalManager;
use crate::shared::message::{ClineAsk, ClineMessage, ClineSay};

// グローバル定数
struct GlobalFileNames {
    ui_messages: &'static str,
    api_conversation_history: &'static str,
}

const GLOBAL_FILE_NAMES: GlobalFileNames = GlobalFileNames {
    ui_messages: "ui_messages.json",
    api_conversation_history: "api_conversation_history.json",
};

// APIメトリクス関連の型
#[derive(Debug)]
struct ApiMetrics {
    total_tokens_in: u32,
    total_tokens_out: u32,
    total_cache_writes: u32,
    total_cache_reads: u32,
    total_cost: f64,
}

#[derive(Debug, Serialize)]
struct TaskHistory {
    id: String,
    ts: i64,
    task: String,
    tokens_in: u32,
    tokens_out: u32,
    cache_writes: u32,
    cache_reads: u32,
    total_cost: f64,
}

// ツール関連の型
#[derive(Debug)]
pub enum ToolResponse {
    Success(String),
    Error(String),
}

impl From<&str> for ToolResponse {
    fn from(s: &str) -> Self {
        ToolResponse::Success(s.to_string())
    }
}

#[derive(Debug)]
pub enum ToolUseName {
    ExecuteCommand,
    WriteToFile,
    ReadFile,
}

impl std::fmt::Display for ToolUseName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolUseName::ExecuteCommand => write!(f, "execute command"),
            ToolUseName::WriteToFile => write!(f, "write to file"),
            ToolUseName::ReadFile => write!(f, "read file"),
        }
    }
}

// ユーティリティ関数
async fn file_exists(path: &PathBuf) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

fn get_api_metrics(messages: &[ClineMessage]) -> ApiMetrics {
    // 実際のメトリクス計算ロジックを実装
    ApiMetrics {
        total_tokens_in: 0,
        total_tokens_out: 0,
        total_cache_writes: 0,
        total_cache_reads: 0,
        total_cost: 0.0,
    }
}

// フォーマットレスポンス用のモジュール
mod format_response {
    pub fn tool_error(msg: String) -> String {
        format!("Tool execution error: {}", msg)
    }

    pub fn missing_tool_parameter_error(param_name: &str) -> String {
        format!("Missing required parameter: {}", param_name)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AskResponse {
    YesButtonClicked,
    NoButtonClicked,
    MessageResponse,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait EditorInfoProvider: Debug + Send + Sync {
    async fn get_visible_files(&self) -> Result<Vec<String>>;
    async fn get_open_tabs(&self) -> Result<Vec<String>>;
}

#[derive(Debug, Clone)]
pub struct Cline {
    task_id: String,
    anthropic_client: AnthropicClient,
    workspace_path: PathBuf,
    did_edit_file: bool,
    custom_instructions: Option<String>,
    diff_enabled: bool,
    fuzzy_match_threshold: f64,
    api_conversation_history: Vec<Message>,
    cline_messages: Vec<ClineMessage>,
    did_complete_reading_stream: bool,
    did_reject_tool: bool,
    did_already_use_tool: bool,
    terminal_manager: Option<Arc<Mutex<dyn TerminalManager + Send + Sync>>>,
    editor_info_provider: Option<Arc<dyn EditorInfoProvider>>,
    browser_session: Option<Arc<Mutex<BrowserSession>>>,
    abort: bool,
    provider: Option<Arc<dyn Provider + Send + Sync>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    #[allow(dead_code)]
    content: Vec<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    #[allow(dead_code)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct StreamResponse {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    response_type: String,
    #[allow(dead_code)]
    index: Option<i32>,
    #[allow(dead_code)]
    delta: Option<Delta>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: String,
    #[allow(dead_code)]
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BrowserAction {
    action: String,
    url: Option<String>,
    coordinate: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BrowserActionResult {
    logs: Option<String>,
    screenshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
    images: Option<Vec<String>>,
}

lazy_static! {
    static ref MENTION_REGEX: Regex = Regex::new(
        r"@((?:/|\w+://)[^\s]+?|[a-f0-9]{7,40}\b|problems\b|git-changes\b)(?=[.,;:!?]?(?=[\s\r\n]|$))"
    ).unwrap();
}

impl Cline {
    pub fn new(
        workspace_path: PathBuf,
        custom_instructions: Option<String>,
        enable_diff: Option<bool>,
        fuzzy_match_threshold: Option<f64>,
    ) -> Result<Self> {
        Ok(Self {
            task_id: Uuid::new_v4().to_string(),
            anthropic_client: AnthropicClient::new()?,
            workspace_path,
            did_edit_file: false,
            custom_instructions,
            diff_enabled: enable_diff.unwrap_or(false),
            fuzzy_match_threshold: fuzzy_match_threshold.unwrap_or(1.0),
            api_conversation_history: Vec::new(),
            cline_messages: Vec::new(),
            did_complete_reading_stream: false,
            did_reject_tool: false,
            did_already_use_tool: false,
            terminal_manager: None,
            editor_info_provider: None,
            browser_session: Some(Arc::new(Mutex::new(BrowserSession::new()))),
            abort: false,
            provider: None,
        })
    }

    pub fn set_editor_info_provider(&mut self, provider: Arc<dyn EditorInfoProvider>) {
        self.editor_info_provider = Some(provider);
    }

    pub async fn send_message(&self, message: &str) -> Result<String> {
        self.anthropic_client.send_message(message).await
    }

    pub fn add_message(&mut self, message: Message) {
        self.api_conversation_history.push(message);
    }

    pub fn add_cline_message(&mut self, message: ClineMessage) {
        self.cline_messages.push(message);
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn workspace_path(&self) -> &PathBuf {
        &self.workspace_path
    }

    pub fn did_edit_file(&self) -> bool {
        self.did_edit_file
    }

    pub fn custom_instructions(&self) -> Option<&str> {
        self.custom_instructions.as_deref()
    }

    pub fn diff_enabled(&self) -> bool {
        self.diff_enabled
    }

    pub fn fuzzy_match_threshold(&self) -> f64 {
        self.fuzzy_match_threshold
    }

    pub fn conversation_history(&self) -> &[Message] {
        &self.api_conversation_history
    }

    pub fn cline_messages(&self) -> &[ClineMessage] {
        &self.cline_messages
    }

    pub async fn recursively_make_cline_requests(
        &mut self,
        user_content: String,
        include_file_details: bool,
    ) -> Result<bool> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // APIリクエスト開始メッセージを追加
        self.add_cline_message(ClineMessage::Say {
            ts: current_time,
            text: Some("API request started...".to_string()),
            say: ClineSay::ApiReqStarted,
            images: None,
            partial: None,
            reasoning: None,
        });

        let mut last_chunk = String::new();
        let mut this = self.clone();
        let assistant_message = self
            .anthropic_client
            .attempt_api_request(
                user_content,
                include_file_details,
                Box::new(move |chunk| {
                    if chunk != last_chunk {
                        let current_time = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as i64;
                        last_chunk = chunk.clone();
                        this.add_cline_message(ClineMessage::Say {
                            ts: current_time,
                            text: Some(chunk),
                            say: ClineSay::Text,
                            images: None,
                            partial: Some(true),
                            reasoning: None,
                        });
                    }
                }),
            )
            .await?;

        // 完了したメッセージを追加
        self.add_cline_message(ClineMessage::Say {
            ts: current_time,
            text: Some(assistant_message.clone()),
            say: ClineSay::Text,
            images: None,
            partial: None,
            reasoning: None,
        });

        // 会話履歴に追加
        self.add_message(Message {
            role: "assistant".to_string(),
            content: assistant_message.clone(),
        });

        // メッセージにツール使用が含まれているかチェック
        let contains_tool_use = assistant_message.contains("<tool>");
        if !contains_tool_use {
            // ツール使用がない場合は、次のリクエストのためのコンテンツを準備
            let next_content = "No tools were used in the response. Please either use a tool or attempt completion.".to_string();
            return Box::pin(self.recursively_make_cline_requests(next_content, false)).await;
        }

        Ok(false)
    }

    pub async fn start_task(
        &mut self,
        task: Option<String>,
        images: Option<Vec<String>>,
    ) -> Result<()> {
        // 会話履歴とメッセージをクリア
        self.cline_messages.clear();
        self.api_conversation_history.clear();

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // 初期メッセージを追加
        self.add_cline_message(ClineMessage::Say {
            ts: current_time,
            text: task.clone(),
            say: ClineSay::Task,
            images: images.clone(),
            partial: None,
            reasoning: None,
        });

        // タスク内容を構築
        let mut task_content = String::new();
        if let Some(task_text) = task {
            task_content.push_str(&format!("<task>\n{}\n</task>", task_text));
        }

        // 環境情報を追加
        task_content.push_str("\n\n<environment_details>\n");
        task_content.push_str(&format!("Workspace: {}\n", self.workspace_path.display()));
        if let Some(instructions) = &self.custom_instructions {
            task_content.push_str(&format!("Custom Instructions: {}\n", instructions));
        }
        task_content.push_str("</environment_details>");

        // 画像情報を追加（もし存在する場合）
        if let Some(img) = images {
            for (i, image_data) in img.iter().enumerate() {
                task_content.push_str(&format!("\n\n<image_{}>", i + 1));
                task_content.push_str(image_data);
                task_content.push_str(&format!("</image_{}>", i + 1));
            }
        }

        // タスクを開始
        self.recursively_make_cline_requests(task_content, true)
            .await?;

        Ok(())
    }

    pub async fn ask(
        &mut self,
        ask_type: String,
        text: Option<String>,
        partial: Option<bool>,
    ) -> Result<(AskResponse, Option<String>, Option<Vec<String>>)> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // 部分的な更新の場合
        if let Some(is_partial) = partial {
            if is_partial {
                let last_message = self.cline_messages.last().cloned();
                let is_updating_previous_partial = last_message.as_ref().is_some_and(|msg| {
                    matches!(
                        msg,
                        ClineMessage::Ask {
                            partial: Some(true),
                            ..
                        }
                    )
                });

                if is_updating_previous_partial {
                    // 既存の部分メッセージを更新
                    if let Some(ClineMessage::Ask { ts, .. }) = last_message {
                        self.cline_messages.pop();
                        self.add_cline_message(ClineMessage::Ask {
                            ts,
                            text,
                            ask: ClineAsk::Followup,
                            partial: Some(true),
                            reasoning: None,
                        });
                    }
                } else {
                    // 新しい部分メッセージを追加
                    self.add_cline_message(ClineMessage::Ask {
                        ts: current_time,
                        text,
                        ask: ClineAsk::Followup,
                        partial: Some(true),
                        reasoning: None,
                    });
                }
                // 部分的な更新の場合は処理を中断
                anyhow::bail!("Current ask promise was ignored");
            } else {
                let last_message = self.cline_messages.last().cloned();
                let is_updating_previous_partial = last_message.as_ref().is_some_and(|msg| {
                    matches!(
                        msg,
                        ClineMessage::Ask {
                            partial: Some(true),
                            ..
                        }
                    )
                });

                // 完了メッセージの処理
                if is_updating_previous_partial {
                    if let Some(ClineMessage::Ask { ts, .. }) = last_message {
                        self.cline_messages.pop();
                        self.add_cline_message(ClineMessage::Ask {
                            ts,
                            text,
                            ask: ClineAsk::Followup,
                            partial: Some(false),
                            reasoning: None,
                        });
                    }
                } else {
                    self.add_cline_message(ClineMessage::Ask {
                        ts: current_time,
                        text,
                        ask: ClineAsk::Followup,
                        partial: Some(false),
                        reasoning: None,
                    });
                }
            }
        } else {
            // 通常のメッセージ
            self.add_cline_message(ClineMessage::Ask {
                ts: current_time,
                text,
                ask: ClineAsk::Followup,
                partial: None,
                reasoning: None,
            });
        }

        Ok((AskResponse::YesButtonClicked, None, None))
    }

    pub async fn say(
        &mut self,
        say_type: String,
        text: Option<String>,
        images: Option<Vec<String>>,
        partial: Option<bool>,
    ) -> Result<()> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        if let Some(is_partial) = partial {
            let last_message = self.cline_messages.last().cloned();
            let is_updating_previous_partial = last_message.as_ref().is_some_and(|msg| {
                matches!(
                    msg,
                    ClineMessage::Say {
                        partial: Some(true),
                        ..
                    }
                )
            });

            if is_partial {
                if is_updating_previous_partial {
                    // 既存の部分メッセージを更新
                    if let Some(ClineMessage::Say { ts, .. }) = last_message {
                        self.cline_messages.pop();
                        self.add_cline_message(ClineMessage::Say {
                            ts,
                            text,
                            say: ClineSay::Text,
                            images,
                            partial: Some(true),
                            reasoning: None,
                        });
                    }
                } else {
                    // 新しい部分メッセージを追加
                    self.add_cline_message(ClineMessage::Say {
                        ts: current_time,
                        text,
                        say: ClineSay::Text,
                        images,
                        partial: Some(true),
                        reasoning: None,
                    });
                }
            } else {
                // 完了メッセージの処理
                if is_updating_previous_partial {
                    if let Some(ClineMessage::Say { ts, .. }) = last_message {
                        self.cline_messages.pop();
                        self.add_cline_message(ClineMessage::Say {
                            ts,
                            text,
                            say: ClineSay::Text,
                            images,
                            partial: Some(false),
                            reasoning: None,
                        });
                    }
                } else {
                    self.add_cline_message(ClineMessage::Say {
                        ts: current_time,
                        text,
                        say: ClineSay::Text,
                        images,
                        partial: Some(false),
                        reasoning: None,
                    });
                }
            }
        } else {
            // 通常のメッセージ
            self.add_cline_message(ClineMessage::Say {
                ts: current_time,
                text,
                say: ClineSay::Text,
                images,
                partial: None,
                reasoning: None,
            });
        }

        Ok(())
    }

    pub async fn initiate_task_loop(
        &mut self,
        initial_task: Option<String>,
        images: Option<Vec<String>>,
    ) -> Result<()> {
        // タスクの初期化
        self.start_task(initial_task, images).await?;

        // タスクループの開始
        loop {
            // ストリームの読み込みが完了しているか確認
            if !self.did_complete_reading_stream {
                continue;
            }

            // ツールが拒否されたか確認
            if self.did_reject_tool {
                self.did_reject_tool = false;
                continue;
            }

            // ツールが使用済みか確認
            if self.did_already_use_tool {
                self.did_already_use_tool = false;
                // ツール使用後の処理を実行
                let next_content =
                    "Tool execution completed. Please proceed with the next step.".to_string();
                self.recursively_make_cline_requests(next_content, false)
                    .await?;
                continue;
            }

            // タスクが完了したかどうかを確認
            // TODO: タスク完了の条件を実装
            break;
        }

        Ok(())
    }

    pub async fn overwrite_cline_messages(&mut self, messages: Vec<ClineMessage>) -> Result<()> {
        self.cline_messages = messages;
        self.save_cline_messages().await
    }

    pub async fn get_saved_cline_messages(&self) -> Result<Vec<ClineMessage>> {
        let task_dir = self.ensure_task_directory_exists().await?;
        let file_path = task_dir.join(GLOBAL_FILE_NAMES.ui_messages);

        if file_exists(&file_path).await {
            let content = fs::read_to_string(&file_path).await?;
            Ok(serde_json::from_str(&content)?)
        } else {
            // 古いパスをチェック
            let old_path = task_dir.join("claude_messages.json");
            if file_exists(&old_path).await {
                let content = fs::read_to_string(&old_path).await?;
                fs::remove_file(&old_path).await?; // 古いファイルを削除
                Ok(serde_json::from_str(&content)?)
            } else {
                Ok(Vec::new())
            }
        }
    }

    pub async fn save_cline_messages(&self) -> Result<()> {
        let task_dir = self.ensure_task_directory_exists().await?;
        let file_path = task_dir.join(GLOBAL_FILE_NAMES.ui_messages);

        fs::write(&file_path, serde_json::to_string(&self.cline_messages)?).await?;

        // APIメトリクスの計算と保存
        let api_metrics = get_api_metrics(&self.cline_messages);
        let task_message = &self.cline_messages[0]; // 最初のメッセージは常にタスクのsay
        let last_relevant_message = self.cline_messages.iter().rev().find(|m| match m {
            ClineMessage::Ask { text, .. } => !matches!(
                text.as_deref(),
                Some("resume_task" | "resume_completed_task")
            ),
            _ => true,
        });

        if let Some(provider) = &self.provider {
            provider
                .update_task_history(TaskHistory {
                    id: self.task_id.clone(),
                    ts: match last_relevant_message {
                        Some(ClineMessage::Ask { ts, .. }) | Some(ClineMessage::Say { ts, .. }) => {
                            *ts
                        }
                        None => 0,
                    },
                    task: match task_message {
                        ClineMessage::Say { text, .. } => text.clone().unwrap_or_default(),
                        _ => String::new(),
                    },
                    tokens_in: api_metrics.total_tokens_in,
                    tokens_out: api_metrics.total_tokens_out,
                    cache_writes: api_metrics.total_cache_writes,
                    cache_reads: api_metrics.total_cache_reads,
                    total_cost: api_metrics.total_cost,
                })
                .await?;
        }

        Ok(())
    }

    pub async fn overwrite_api_conversation_history(
        &mut self,
        new_history: Vec<Message>,
    ) -> Result<()> {
        self.api_conversation_history = new_history;
        self.save_api_conversation_history().await
    }

    pub async fn get_saved_api_conversation_history(&self) -> Result<Vec<Message>> {
        let task_dir = self.ensure_task_directory_exists().await?;
        let file_path = task_dir.join(GLOBAL_FILE_NAMES.api_conversation_history);

        if file_exists(&file_path).await {
            let content = fs::read_to_string(&file_path).await?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn save_api_conversation_history(&self) -> Result<()> {
        let task_dir = self.ensure_task_directory_exists().await?;
        let file_path = task_dir.join(GLOBAL_FILE_NAMES.api_conversation_history);

        fs::write(
            &file_path,
            serde_json::to_string(&self.api_conversation_history)?,
        )
        .await?;

        Ok(())
    }

    pub async fn abort_task(&mut self) {
        self.abort = true;
        if let Some(terminal_manager) = &mut self.terminal_manager {
            terminal_manager.lock().unwrap().dispose_all();
        }
        if let Some(browser_session) = &mut self.browser_session {
            browser_session.lock().unwrap().close_browser().await.ok();
        }
    }

    pub async fn execute_command_tool(&mut self, command: String) -> Result<(bool, ToolResponse)> {
        let terminal_info = self
            .terminal_manager
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Terminal manager not initialized"))?
            .lock()
            .unwrap()
            .get_or_create_terminal(self.workspace_path.to_string_lossy().to_string())?;

        let process = self
            .terminal_manager
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Terminal manager not initialized"))?
            .lock()
            .unwrap()
            .run_command(terminal_info, command.clone())?;

        // コマンド実行の結果を処理
        // TypeScriptコードの実装に合わせて、出力の収集とユーザーフィードバックの処理を実装

        Ok((false, "Command executed successfully".into()))
    }

    pub async fn say_and_create_missing_param_error(
        &mut self,
        tool_name: ToolUseName,
        param_name: String,
        rel_path: Option<String>,
    ) -> Result<String> {
        let error_message = format!(
            "Roo tried to use {}{} without value for required parameter '{}'. Retrying...",
            tool_name,
            rel_path
                .map(|p| format!(" for '{}'", p))
                .unwrap_or_default(),
            param_name
        );

        self.say("error".to_string(), Some(error_message.clone()), None, None)
            .await?;

        Ok(format_response::tool_error(
            format_response::missing_tool_parameter_error(&param_name),
        ))
    }

    pub async fn present_assistant_message(&mut self) -> Result<()> {
        if self.abort {
            return Err(anyhow::anyhow!("Roo Code instance aborted"));
        }

        // TypeScriptコードの実装に合わせて、
        // アシスタントメッセージの表示とツール実行の処理を実装

        Ok(())
    }

    pub async fn get_environment_details(&self, include_file_details: bool) -> Result<String> {
        let mut details = String::new();

        // Editor Visible Files
        details.push_str("\n\n# Editor Visible Files\n");
        if let Some(provider) = &self.editor_info_provider {
            let visible_files = provider.get_visible_files().await?;
            if visible_files.is_empty() {
                details.push_str("(No visible files)");
            } else {
                details.push_str(&visible_files.join("\n"));
            }
        } else {
            details.push_str("(Editor information not available)");
        }

        // Editor Open Tabs
        details.push_str("\n\n# Editor Open Tabs\n");
        if let Some(provider) = &self.editor_info_provider {
            let open_tabs = provider.get_open_tabs().await?;
            if open_tabs.is_empty() {
                details.push_str("(No open tabs)");
            } else {
                details.push_str(&open_tabs.join("\n"));
            }
        } else {
            details.push_str("(Editor information not available)");
        }

        // Terminal Details
        if let Some(terminal_manager) = &self.terminal_manager {
            let mut terminal_manager = terminal_manager.lock().unwrap();
            let busy_terminals = terminal_manager.get_terminals(true);
            let inactive_terminals = terminal_manager.get_terminals(false);

            if !busy_terminals.is_empty() && self.did_edit_file {
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }

            let mut terminal_details = String::new();

            // Actively Running Terminals
            if !busy_terminals.is_empty() {
                terminal_details.push_str("\n\n# Actively Running Terminals");
                for terminal in busy_terminals {
                    terminal_details.push_str(&format!(
                        "\n## Original command: `{}`",
                        terminal.last_command
                    ));
                    if let Some(output) = terminal_manager.get_unretrieved_output(terminal.id) {
                        terminal_details.push_str(&format!("\n### New Output\n{}", output));
                    }
                }
            }

            // Inactive Terminals
            if !inactive_terminals.is_empty() {
                let mut inactive_terminal_outputs = HashMap::new();
                for terminal in &inactive_terminals {
                    if let Some(output) = terminal_manager.get_unretrieved_output(terminal.id) {
                        inactive_terminal_outputs.insert(terminal.id, output);
                    }
                }

                if !inactive_terminal_outputs.is_empty() {
                    terminal_details.push_str("\n\n# Inactive Terminals");
                    for terminal in inactive_terminals {
                        if let Some(output) = inactive_terminal_outputs.get(&terminal.id) {
                            terminal_details.push_str(&format!("\n## {}", terminal.last_command));
                            terminal_details.push_str(&format!("\n### New Output\n{}", output));
                        }
                    }
                }
            }

            if !terminal_details.is_empty() {
                details.push_str(&terminal_details);
            }
        }

        // Current Time
        let now: DateTime<Local> = SystemTime::now().into();
        let timezone_offset = now.offset().local_minus_utc() as f32 / 3600.0;
        let timezone_offset_str = format!("{:+}:00", timezone_offset);

        details.push_str("\n\n# Current Time\n");
        details.push_str(&format!(
            "{} ({}, UTC{})",
            now.format("%Y-%m-%d %I:%M:%S %p"),
            Local::now().format("%Z"),
            timezone_offset_str
        ));

        // Context Size
        let api_metrics = get_api_metrics(&self.cline_messages);
        let context_tokens = api_metrics.total_tokens_in + api_metrics.total_tokens_out;
        let context_window = 128_000; // Claude 3.5 Sonnetのコンテキストウィンドウサイズ
        let context_percentage = (context_tokens as f64 / context_window as f64 * 100.0).round();

        details.push_str("\n\n# Current Context Size (Tokens)\n");
        details.push_str(&format!(
            "{} ({}%)",
            context_tokens.to_string(),
            context_percentage
        ));

        // Current Mode
        details.push_str("\n\n# Current Mode\n");
        details.push_str("default"); // モード機能は別途実装が必要

        // Current Working Directory Files
        if include_file_details {
            details.push_str(&format!(
                "\n\n# Current Working Directory ({}) Files\n",
                self.workspace_path.display()
            ));

            // TODO: list_filesの実装が必要
            details.push_str("(File listing not implemented)");
        }

        Ok(format!(
            "<environment_details>\n{}\n</environment_details>",
            details.trim()
        ))
    }

    /// コンテキストを読み込む
    pub async fn load_context(&self, text: String) -> Result<UserContent> {
        if should_process_mentions(&text) {
            if let Some(browser_session) = &self.browser_session {
                let mut browser = browser_session.lock().unwrap();
                let parsed_text = parse_mentions(&text, &mut browser, &self.workspace_path).await?;
                Ok(UserContent {
                    content_type: "text".to_string(),
                    text: Some(parsed_text),
                    images: None,
                })
            } else {
                Ok(UserContent {
                    content_type: "text".to_string(),
                    text: Some(text),
                    images: None,
                })
            }
        } else {
            Ok(UserContent {
                content_type: "text".to_string(),
                text: Some(text),
                images: None,
            })
        }
    }

    async fn ensure_task_directory_exists(&self) -> Result<PathBuf> {
        let task_dir = self.workspace_path.join(".cline").join(&self.task_id);
        if !task_dir.exists() {
            tokio::fs::create_dir_all(&task_dir).await?;
        }
        Ok(task_dir)
    }

    async fn get_file_or_folder_content(&self, mention_path: &str) -> Result<String> {
        let abs_path = self.workspace_path.join(mention_path);

        let metadata = tokio::fs::metadata(&abs_path).await?;
        if metadata.is_dir() {
            // TODO: ディレクトリ内容の取得を実装
            Ok("Directory listing not implemented".to_string())
        } else {
            tokio::fs::read_to_string(&abs_path)
                .await
                .map_err(Into::into)
        }
    }
}

#[async_trait]
pub trait Provider: std::fmt::Debug + Send + Sync {
    async fn update_task_history(&self, history: TaskHistory) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use regex::Regex;

    async fn create_test_cline(mock_provider: MockEditorInfoProvider) -> Result<Cline> {
        let mut cline = Cline::new(
            PathBuf::from("/test/workspace"),
            None,
            Some(false),
            Some(1.0),
        )?;
        cline.set_editor_info_provider(Arc::new(mock_provider));
        Ok(cline)
    }

    #[tokio::test]
    async fn test_get_environment_details() {
        let mut mock = MockEditorInfoProvider::new();
        mock.expect_get_visible_files()
            .returning(|| Ok(vec!["file1.rs".to_string(), "file2.rs".to_string()]));
        mock.expect_get_open_tabs()
            .returning(|| Ok(vec!["tab1.rs".to_string(), "tab2.rs".to_string()]));

        let cline = create_test_cline(mock).await.unwrap();
        let details = cline.get_environment_details(true).await.unwrap();
        println!("Generated details:\n{}", details);

        let time_regex = Regex::new(
            r"\d{4}-\d{2}-\d{2} \d{1,2}:\d{2}:\d{2} (?:AM|PM) \([+-]\d{1,2}:00, UTC[+-]\d{1,2}:00\)"
        ).unwrap();
        let normalized_details = time_regex.replace(&details, "TIME_PLACEHOLDER");

        let expected = r#"<environment_details>
# Editor Visible Files
file1.rs
file2.rs

# Editor Open Tabs
tab1.rs
tab2.rs

# Current Time
TIME_PLACEHOLDER

# Current Context Size (Tokens)
0 (0%)

# Current Mode
default

# Current Working Directory (/test/workspace) Files
(File listing not implemented)
</environment_details>"#;

        assert_eq!(normalized_details, expected);
    }

    #[tokio::test]
    async fn test_get_environment_details_with_empty_files() {
        let mut mock = MockEditorInfoProvider::new();
        mock.expect_get_visible_files().returning(|| Ok(vec![]));
        mock.expect_get_open_tabs().returning(|| Ok(vec![]));

        let cline = create_test_cline(mock).await.unwrap();
        let details = cline.get_environment_details(true).await.unwrap();

        let time_regex = Regex::new(
            r"\d{4}-\d{2}-\d{2} \d{1,2}:\d{2}:\d{2} (?:AM|PM) \([+-]\d{1,2}:00, UTC[+-]\d{1,2}:00\)"
        ).unwrap();
        let normalized_details = time_regex.replace(&details, "TIME_PLACEHOLDER");

        let expected = r#"<environment_details>
# Editor Visible Files
(No visible files)

# Editor Open Tabs
(No open tabs)

# Current Time
TIME_PLACEHOLDER

# Current Context Size (Tokens)
0 (0%)

# Current Mode
default

# Current Working Directory (/test/workspace) Files
(File listing not implemented)
</environment_details>"#;

        assert_eq!(normalized_details, expected);
    }

    #[tokio::test]
    async fn test_get_environment_details_without_file_details() {
        let mut mock = MockEditorInfoProvider::new();
        mock.expect_get_visible_files().returning(|| Ok(vec![]));
        mock.expect_get_open_tabs().returning(|| Ok(vec![]));

        let cline = create_test_cline(mock).await.unwrap();
        let details = cline.get_environment_details(false).await.unwrap();

        let time_regex = Regex::new(
            r"\d{4}-\d{2}-\d{2} \d{1,2}:\d{2}:\d{2} (?:AM|PM) \([+-]\d{1,2}:00, UTC[+-]\d{1,2}:00\)"
        ).unwrap();
        let normalized_details = time_regex.replace(&details, "TIME_PLACEHOLDER");

        let expected = r#"<environment_details>
# Editor Visible Files
(No visible files)

# Editor Open Tabs
(No open tabs)

# Current Time
TIME_PLACEHOLDER

# Current Context Size (Tokens)
0 (0%)

# Current Mode
default
</environment_details>"#;

        assert_eq!(normalized_details, expected);
    }
}
