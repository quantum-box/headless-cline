use anyhow::Result;
use cline_core::services::mcp::{McpHub, McpSettings, StdioConfig};
use serde_json::json;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let temp_dir = std::env::temp_dir();
    let settings_path = temp_dir.join("mcp_settings.json");
    let servers_dir = temp_dir.join("mcp_servers");

    std::fs::create_dir_all(&servers_dir)?;
    if !settings_path.exists() {
        let settings = McpSettings {
            mcp_servers: std::collections::HashMap::new(),
        };
        std::fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
    }

    let mcp_hub = McpHub::new(servers_dir.clone(), settings_path.clone())?;
    tracing::info!("McpHub initialized");

    let servers = mcp_hub.get_servers();
    tracing::info!("Available servers: {}", servers.len());
    for server in &servers {
        tracing::info!("Server: {}, Status: {:?}", server.name, server.status);
    }

    if servers.is_empty() {
        tracing::info!("No servers found, setting up a sample server");
        setup_sample_server(&settings_path)?;

        let mcp_hub = McpHub::new(servers_dir.clone(), settings_path.clone())?;

        let servers = mcp_hub.get_servers();
        tracing::info!("Available servers after setup: {}", servers.len());
        for server in &servers {
            tracing::info!("Server: {}, Status: {:?}", server.name, server.status);
        }
    }

    for server in mcp_hub.get_servers() {
        if let Some(tools) = &server.tools {
            for tool in tools {
                tracing::info!("Tool: {}, Description: {}", tool.name, tool.description);

                let result = mcp_hub
                    .call_tool(&server.name, &tool.name, Some(json!({})))
                    .await;

                match result {
                    Ok(response) => {
                        tracing::info!("Tool call result: {:?}", response);
                    }
                    Err(e) => {
                        tracing::error!("Error calling tool: {:?}", e);
                    }
                }

                break; // 最初のツールのみテスト
            }
        }
    }

    tracing::info!("Example completed successfully");
    Ok(())
}

fn setup_sample_server(settings_path: &PathBuf) -> Result<()> {
    let mut settings: McpSettings = if settings_path.exists() {
        let content = std::fs::read_to_string(settings_path)?;
        serde_json::from_str(&content)?
    } else {
        McpSettings {
            mcp_servers: std::collections::HashMap::new(),
        }
    };

    let config = StdioConfig {
        command: "echo".to_string(),
        args: Some(vec!["{\"hello\": \"world\"}".to_string()]),
        env: Some(std::collections::HashMap::new()),
        disabled: None,
        always_allow: None,
        timeout: None,
    };

    settings
        .mcp_servers
        .insert("sample-server".to_string(), config);

    std::fs::write(settings_path, serde_json::to_string_pretty(&settings)?)?;

    tracing::info!("Sample server configuration saved");
    Ok(())
}
