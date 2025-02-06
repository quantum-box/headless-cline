use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::services::anthropic::{AnthropicClient, Message};

const API_KEY: &str = "";

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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ClineMessage {
    Ask {
        ts: i64,
        text: Option<String>,
        partial: bool,
    },
    Say {
        ts: i64,
        text: Option<String>,
        images: Option<Vec<String>>,
        partial: bool,
    },
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
    content: Vec<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    text: String,
}

#[derive(Debug, Deserialize)]
struct StreamResponse {
    #[serde(rename = "type")]
    response_type: String,
    index: Option<i32>,
    delta: Option<Delta>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(rename = "type")]
    delta_type: String,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AskResponse {
    YesButtonClicked,
    NoButtonClicked,
    MessageResponse,
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
        })
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
            images: None,
            partial: true,
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
                            images: None,
                            partial: true,
                        });
                    }
                }),
            )
            .await?;

        // 完了したメッセージを追加
        self.add_cline_message(ClineMessage::Say {
            ts: current_time,
            text: Some(assistant_message.clone()),
            images: None,
            partial: false,
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
            images: images.clone(),
            partial: false,
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
        _ask_type: String,
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
                    matches!(msg, ClineMessage::Ask { partial: true, .. })
                });

                if is_updating_previous_partial {
                    // 既存の部分メッセージを更新
                    if let Some(ClineMessage::Ask { ts, .. }) = last_message {
                        self.cline_messages.pop();
                        self.add_cline_message(ClineMessage::Ask {
                            ts,
                            text,
                            partial: true,
                        });
                    }
                } else {
                    // 新しい部分メッセージを追加
                    self.add_cline_message(ClineMessage::Ask {
                        ts: current_time,
                        text,
                        partial: true,
                    });
                }
                // 部分的な更新の場合は処理を中断
                anyhow::bail!("Current ask promise was ignored");
            } else {
                let last_message = self.cline_messages.last().cloned();
                let is_updating_previous_partial = last_message.as_ref().is_some_and(|msg| {
                    matches!(msg, ClineMessage::Ask { partial: true, .. })
                });

                // 完了メッセージの処理
                if is_updating_previous_partial {
                    if let Some(ClineMessage::Ask { ts, .. }) = last_message {
                        self.cline_messages.pop();
                        self.add_cline_message(ClineMessage::Ask {
                            ts,
                            text,
                            partial: false,
                        });
                    }
                } else {
                    self.add_cline_message(ClineMessage::Ask {
                        ts: current_time,
                        text,
                        partial: false,
                    });
                }
            }
        } else {
            // 通常のメッセージ
            self.add_cline_message(ClineMessage::Ask {
                ts: current_time,
                text,
                partial: false,
            });
        }

        Ok((AskResponse::YesButtonClicked, None, None))
    }

    pub async fn say(
        &mut self,
        _say_type: String,
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
                matches!(msg, ClineMessage::Say { partial: true, .. })
            });

            if is_partial {
                if is_updating_previous_partial {
                    // 既存の部分メッセージを更新
                    if let Some(ClineMessage::Say { ts, .. }) = last_message {
                        self.cline_messages.pop();
                        self.add_cline_message(ClineMessage::Say {
                            ts,
                            text,
                            images,
                            partial: true,
                        });
                    }
                } else {
                    // 新しい部分メッセージを追加
                    self.add_cline_message(ClineMessage::Say {
                        ts: current_time,
                        text,
                        images,
                        partial: true,
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
                            images,
                            partial: false,
                        });
                    }
                } else {
                    self.add_cline_message(ClineMessage::Say {
                        ts: current_time,
                        text,
                        images,
                        partial: false,
                    });
                }
            }
        } else {
            // 通常のメッセージ
            self.add_cline_message(ClineMessage::Say {
                ts: current_time,
                text,
                images,
                partial: false,
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
}
