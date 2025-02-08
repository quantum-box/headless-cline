use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::sync::Arc;
use std::{env, fmt::Debug};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
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
#[allow(dead_code)]
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

pub type MessageCallback = Box<dyn FnMut(String) + Send + 'static>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AnthropicClientTrait: Send + Sync + std::fmt::Debug {
    async fn send_message(&self, message: &str) -> Result<String>;
    async fn attempt_api_request(
        &self,
        user_content: String,
        include_file_details: bool,
        on_chunk: MessageCallback,
    ) -> Result<String>;
}

#[derive(Debug, Clone)]
pub enum AnthropicClient {
    Real {
        client: Client,
        api_key: String,
    },
    #[cfg(test)]
    Mock(Arc<MockAnthropicClientTrait>),
}

impl AnthropicClient {
    pub fn new() -> Result<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY environment variable not set"))?;

        Ok(Self::Real {
            client: Client::new(),
            api_key,
        })
    }

    #[cfg(test)]
    pub fn mock(mock: MockAnthropicClientTrait) -> Self {
        Self::Mock(Arc::new(mock))
    }
}

#[async_trait]
impl AnthropicClientTrait for AnthropicClient {
    async fn send_message(&self, message: &str) -> Result<String> {
        match self {
            Self::Real { client, api_key } => {
                let request_body = ClaudeRequest {
                    model: "claude-3-sonnet-20240229".to_string(),
                    messages: vec![Message {
                        role: "user".to_string(),
                        content: message.to_string(),
                    }],
                    max_tokens: 1000,
                    stream: false,
                };

                let response = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("accept", "application/json")
                    .header("content-type", "application/json")
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&request_body)
                    .send()
                    .await?;

                if response.status() != StatusCode::OK {
                    anyhow::bail!("API request failed: {}", response.text().await?);
                }

                let claude_response: ClaudeResponse = response.json().await?;
                Ok(claude_response.content[0].text.clone())
            }
            #[cfg(test)]
            Self::Mock(mock) => mock.as_ref().send_message(message).await,
        }
    }

    async fn attempt_api_request(
        &self,
        user_content: String,
        _include_file_details: bool,
        mut on_chunk: MessageCallback,
    ) -> Result<String> {
        match self {
            Self::Real { client, api_key } => {
                let request_body = ClaudeRequest {
                    model: "claude-3-sonnet-20240229".to_string(),
                    messages: vec![Message {
                        role: "user".to_string(),
                        content: user_content,
                    }],
                    max_tokens: 1000,
                    stream: true,
                };

                let response = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("accept", "application/json")
                    .header("content-type", "application/json")
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&request_body)
                    .send()
                    .await?;

                if response.status() != StatusCode::OK {
                    let error_text = response.text().await?;
                    tracing::error!("API request failed: {}", error_text);
                    anyhow::bail!("API request failed: {}", error_text);
                }

                let mut stream = response.bytes_stream();
                let mut assistant_message = String::new();

                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?.to_vec();
                    let text = String::from_utf8_lossy(&chunk);

                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                continue;
                            }

                            if let Ok(response) = serde_json::from_str::<StreamResponse>(data) {
                                if let Some(delta) = response.delta {
                                    if delta.delta_type == "text_delta" {
                                        assistant_message.push_str(&delta.text);
                                        on_chunk(assistant_message.clone());
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(assistant_message)
            }
            #[cfg(test)]
            Self::Mock(mock) => {
                mock.as_ref()
                    .attempt_api_request(user_content, _include_file_details, on_chunk)
                    .await
            }
        }
    }
}
