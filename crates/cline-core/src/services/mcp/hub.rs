use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::json;
use tokio::sync::mpsc;

use super::types::*;

#[derive(Debug)]
pub struct McpConnection {
    #[allow(dead_code)]
    pub server: McpServer,
    #[allow(dead_code)]
    pub client: reqwest::Client,
}

#[derive(Debug)]
pub struct McpHub {
    connections: Arc<Mutex<Vec<McpConnection>>>,
    settings_path: PathBuf,
    workspace_path: PathBuf,
    is_connecting: bool,
    #[allow(dead_code)]
    file_watchers: HashMap<String, RecommendedWatcher>,
}

#[allow(dead_code)]
impl McpHub {
    pub fn new(workspace_path: PathBuf, settings_path: PathBuf) -> Result<Self> {
        let hub = Self {
            connections: Arc::new(Mutex::new(Vec::new())),
            settings_path,
            workspace_path,
            is_connecting: false,
            file_watchers: HashMap::new(),
        };

        // 設定ファイルが存在しない場合は作成
        if !hub.settings_path.exists() {
            fs::write(
                &hub.settings_path,
                serde_json::to_string_pretty(&json!({
                    "mcpServers": {}
                }))?,
            )?;
        }

        // 設定ファイルの監視を開始
        hub.watch_mcp_settings_file()?;
        hub.initialize_mcp_servers()?;

        Ok(hub)
    }

    fn watch_mcp_settings_file(&self) -> Result<()> {
        let settings_path = self.settings_path.clone();
        let _connections = Arc::clone(&self.connections);

        let (tx, mut rx) = mpsc::channel(32);

        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;

        watcher.watch(&settings_path, RecursiveMode::NonRecursive)?;

        let mut hub = self.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let notify::Event {
                    kind: notify::EventKind::Modify(_),
                    ..
                } = event
                {
                    if let Err(e) = hub.reload_settings().await {
                        eprintln!("Failed to reload settings: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    async fn reload_settings(&mut self) -> Result<()> {
        let content = fs::read_to_string(&self.settings_path)?;
        let settings: McpSettings = serde_json::from_str(&content)?;
        self.update_server_connections(settings.mcp_servers).await?;
        Ok(())
    }

    fn initialize_mcp_servers(&self) -> Result<()> {
        let content = fs::read_to_string(&self.settings_path)?;
        let settings: McpSettings = serde_json::from_str(&content)?;

        let mut connections = self.connections.lock().unwrap();
        for (name, config) in settings.mcp_servers {
            let connection = self.create_connection(&name, &config)?;
            connections.push(connection);
        }

        Ok(())
    }

    fn create_connection(&self, name: &str, config: &StdioConfig) -> Result<McpConnection> {
        let client = reqwest::Client::new();

        let server = McpServer {
            name: name.to_string(),
            config: serde_json::to_string(config)?,
            status: McpServerStatus::Connecting,
            error: None,
            disabled: config.disabled,
            tools: None,
            resources: None,
            resource_templates: None,
        };

        Ok(McpConnection { server, client })
    }

    async fn update_server_connections(
        &mut self,
        new_servers: HashMap<String, StdioConfig>,
    ) -> Result<()> {
        self.is_connecting = true;

        let mut connections = self.connections.lock().unwrap();
        let _current_names: Vec<String> =
            connections.iter().map(|c| c.server.name.clone()).collect();

        // 削除されたサーバーを削除
        connections.retain(|conn| new_servers.contains_key(&conn.server.name));

        // 新規サーバーを追加または更新
        for (name, config) in new_servers {
            if let Some(conn) = connections.iter_mut().find(|c| c.server.name == name) {
                // 設定が変更された場合は更新
                let new_config = serde_json::to_string(&config)?;
                if conn.server.config != new_config {
                    *conn = self.create_connection(&name, &config)?;
                }
            } else {
                // 新規サーバーを追加
                connections.push(self.create_connection(&name, &config)?);
            }
        }

        self.is_connecting = false;
        Ok(())
    }

    pub fn get_servers(&self) -> Vec<McpServer> {
        let connections = self.connections.lock().unwrap();
        connections
            .iter()
            .filter(|conn| !conn.server.disabled.unwrap_or(false))
            .map(|conn| conn.server.clone())
            .collect()
    }

    pub async fn call_tool(
        &self,
        server_name: &str,
        _tool_name: &str,
        tool_arguments: Option<serde_json::Value>,
    ) -> Result<McpToolCallResponse> {
        let connections = self.connections.lock().unwrap();
        let connection = connections
            .iter()
            .find(|c| c.server.name == server_name)
            .context("Server not found")?;

        if connection.server.disabled.unwrap_or(false) {
            anyhow::bail!("Server is disabled");
        }

        // ツール呼び出しの実装
        // 実際のAPIエンドポイントやプロトコルに合わせて実装する必要があります
        Ok(McpToolCallResponse {
            result: tool_arguments.unwrap_or(json!({})),
        })
    }

    pub async fn read_resource(
        &self,
        server_name: &str,
        _uri: &str,
    ) -> Result<McpResourceResponse> {
        let connections = self.connections.lock().unwrap();
        let connection = connections
            .iter()
            .find(|c| c.server.name == server_name)
            .context("Server not found")?;

        if connection.server.disabled.unwrap_or(false) {
            anyhow::bail!("Server is disabled");
        }

        // リソース読み込みの実装
        // 実際のAPIエンドポイントやプロトコルに合わせて実装する必要があります
        Ok(McpResourceResponse {
            content: String::new(),
        })
    }

    pub async fn toggle_tool_always_allow(
        &self,
        server_name: &str,
        tool_name: &str,
        should_allow: bool,
    ) -> Result<()> {
        let content = fs::read_to_string(&self.settings_path)?;
        let mut settings: McpSettings = serde_json::from_str(&content)?;

        if let Some(server_config) = settings.mcp_servers.get_mut(server_name) {
            let always_allow = server_config.always_allow.get_or_insert_with(Vec::new);

            if should_allow {
                if !always_allow.contains(&tool_name.to_string()) {
                    always_allow.push(tool_name.to_string());
                }
            } else {
                always_allow.retain(|t| t != tool_name);
            }

            fs::write(
                &self.settings_path,
                serde_json::to_string_pretty(&settings)?,
            )?;
        }

        Ok(())
    }

    pub async fn toggle_server_disabled(&self, server_name: &str, disabled: bool) -> Result<()> {
        let content = fs::read_to_string(&self.settings_path)?;
        let mut settings: McpSettings = serde_json::from_str(&content)?;

        if let Some(server_config) = settings.mcp_servers.get_mut(server_name) {
            server_config.disabled = Some(disabled);

            fs::write(
                &self.settings_path,
                serde_json::to_string_pretty(&settings)?,
            )?;

            let mut connections = self.connections.lock().unwrap();
            if let Some(conn) = connections
                .iter_mut()
                .find(|c| c.server.name == server_name)
            {
                conn.server.disabled = Some(disabled);
            }
        }

        Ok(())
    }

    pub async fn update_server_timeout(&self, server_name: &str, timeout: u32) -> Result<()> {
        let content = fs::read_to_string(&self.settings_path)?;
        let mut settings: McpSettings = serde_json::from_str(&content)?;

        if let Some(server_config) = settings.mcp_servers.get_mut(server_name) {
            server_config.timeout = Some(timeout);

            fs::write(
                &self.settings_path,
                serde_json::to_string_pretty(&settings)?,
            )?;
        }

        Ok(())
    }

    pub async fn get_mcp_servers_path(&self) -> String {
        self.workspace_path
            .join("mcp-servers")
            .to_string_lossy()
            .to_string()
    }

    pub async fn get_mcp_settings_file_path(&self) -> String {
        self.settings_path.to_string_lossy().to_string()
    }
}

impl Clone for McpHub {
    fn clone(&self) -> Self {
        Self {
            connections: Arc::clone(&self.connections),
            settings_path: self.settings_path.clone(),
            workspace_path: self.workspace_path.clone(),
            is_connecting: self.is_connecting,
            file_watchers: HashMap::new(),
        }
    }
}
