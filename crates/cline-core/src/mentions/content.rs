use anyhow::Result;
use std::path::Path;
use tokio::fs;

use super::types::MentionContent;
use crate::services::browser::BrowserSession;
use crate::services::diagnostics::DiagnosticsProvider;
use crate::services::git::GitService;

/// ファイルまたはフォルダの内容を取得
pub async fn get_file_or_folder_content(
    workspace_path: &Path,
    mention_path: &str,
) -> Result<String> {
    let abs_path = workspace_path.join(mention_path);

    let metadata = fs::metadata(&abs_path).await?;
    if metadata.is_dir() {
        let mut entries = fs::read_dir(&abs_path).await?;
        let mut folder_content = String::new();
        let mut file_contents = Vec::new();
        let mut entry_count = 0;

        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // ツリー表示のためのプレフィックス
            let line_prefix = "├── ";

            if file_type.is_file() {
                folder_content.push_str(&format!("{}{}\n", line_prefix, name_str));

                // ファイルの内容を取得（バイナリファイルは除外）
                let file_path = entry.path();
                if let Ok(content) = fs::read_to_string(&file_path).await {
                    let rel_path = file_path.strip_prefix(workspace_path)?.to_string_lossy();
                    file_contents.push(format!(
                        "<file_content path=\"{}\">\n{}\n</file_content>",
                        rel_path, content
                    ));
                }
            } else if file_type.is_dir() {
                folder_content.push_str(&format!("{}{}/\n", line_prefix, name_str));
            } else {
                folder_content.push_str(&format!("{}{}\n", line_prefix, name_str));
            }

            entry_count += 1;
            if entry_count >= 100 {
                folder_content.push_str("\n(Directory listing truncated at 100 entries)");
                break;
            }
        }

        // 最後のエントリのプレフィックスを修正
        if let Some(last_line_pos) = folder_content.rfind("├── ") {
            folder_content.replace_range(last_line_pos..last_line_pos + 4, "└── ");
        }

        // ファイル内容を追加
        if !file_contents.is_empty() {
            folder_content.push_str("\n\n");
            folder_content.push_str(&file_contents.join("\n\n"));
        }

        Ok(folder_content)
    } else {
        // ファイルの場合は内容を直接返す
        fs::read_to_string(&abs_path).await.map_err(Into::into)
    }
}

/// URLの内容を取得
pub async fn get_url_content(url: &str, browser_session: &mut BrowserSession) -> Result<String> {
    if !browser_session.is_initialized() {
        return Err(anyhow::anyhow!("Browser not initialized"));
    }
    browser_session.url_to_markdown(url).await
}

/// ワークスペースの問題を取得
pub async fn get_workspace_problems(diagnostics_provider: &DiagnosticsProvider) -> Result<String> {
    Ok(diagnostics_provider.format_diagnostics())
}

/// Git変更を取得
pub async fn get_git_changes(workspace_path: &Path) -> Result<String> {
    let git_service = GitService::new();
    git_service.get_working_state(workspace_path).await
}

/// Gitのコミット情報を取得
pub async fn get_git_commit_info(commit_hash: &str, workspace_path: &Path) -> Result<String> {
    let git_service = GitService::new();
    git_service
        .get_commit_info(workspace_path, commit_hash)
        .await
}
