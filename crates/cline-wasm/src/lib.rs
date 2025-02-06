use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use futures_util::StreamExt;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

const API_KEY: &str = "";

#[wasm_bindgen]
pub async fn hellp_world() -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
    tracing_wasm::try_set_as_global_default()
        .unwrap_or_else(|e| tracing::warn!("failed to set tracing: {}", e));

    Ok(claude_api("こんにちは！")
        .await
        .map_err(|e| JsValue::from_str(e.to_string().as_str()))?)
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<Content>,
}

#[derive(Deserialize)]
struct Content {
    text: String,
}

#[derive(Deserialize)]
struct StreamResponse {
    #[serde(rename = "type")]
    response_type: String,
    index: Option<i32>,
    delta: Option<Delta>,
}

#[derive(Deserialize)]
struct Delta {
    #[serde(rename = "type")]
    delta_type: String,
    text: String,
}

pub async fn claude_api(message: &str) -> anyhow::Result<String> {
    tracing::info!("start claude api process");
    let client = reqwest::Client::new();

    let request_body = ClaudeRequest {
        model: "claude-3-5-sonnet-latest".to_string(),
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
        .header("x-api-key", API_KEY)
        .header("anthropic-version", "2023-06-01")
        .json(&request_body)
        .send()
        .await?;

    if response.status() != StatusCode::OK {
        tracing::error!("error {}", &response.text().await?);
        anyhow::bail!("http status")
    }

    let claude_response: ClaudeResponse = response
        .json()
        .await
        .inspect_err(|x| tracing::error!("{:#?}", x))?;

    Ok(claude_response.content[0].text.clone())
}

#[wasm_bindgen]
pub async fn stream_response(message: String, callback: js_sys::Function) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    tracing_wasm::try_set_as_global_default()
        .unwrap_or_else(|e| tracing::warn!("failed to set tracing: {}", e));

    let client = reqwest::Client::new();

    let request_body = ClaudeRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: message,
        }],
        max_tokens: 1000,
        stream: true,
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("x-api-key", API_KEY)
        .header("anthropic-version", "2023-06-01")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut stream = response.bytes_stream();
    let this = JsValue::null();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| JsValue::from_str(&e.to_string()))?;
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            tracing::debug!("Received line: {}", line);
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    tracing::debug!("Stream completed");
                    continue;
                }

                tracing::debug!("Parsing data: {}", data);
                if let Ok(response) = serde_json::from_str::<StreamResponse>(data) {
                    if let Some(delta) = response.delta {
                        if delta.delta_type == "text_delta" {
                            tracing::debug!("Received content: {}", delta.text);
                            callback
                                .call1(&this, &JsValue::from_str(&delta.text))
                                .map_err(|e| {
                                    JsValue::from_str(&format!("Callback error: {:?}", e))
                                })?;
                        }
                    }
                } else {
                    tracing::warn!("Failed to parse response JSON");
                }
            }
        }
    }

    Ok(())
}
