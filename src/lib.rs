use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

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

pub async fn claude_api(message: &str) -> anyhow::Result<String> {
    tracing::info!("start claude api process");
    let client = reqwest::Client::new();

    let api_key = "";

    let request_body = ClaudeRequest {
        model: "claude-3-5-sonnet-latest".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: message.to_string(),
        }],
        max_tokens: 1000,
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
        tracing::error!("error {}", &response.text().await?);
        anyhow::bail!("http status")
    }

    let claude_response: ClaudeResponse = response
        .json()
        .await
        .inspect_err(|x| tracing::error!("{:#?}", x))?;

    Ok(claude_response.content[0].text.clone())
}
