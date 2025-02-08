mod content;
mod types;

pub use types::*;

use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use std::path::Path;

use crate::services::browser::BrowserSession;
use crate::services::diagnostics::DiagnosticsProvider;

use self::content::{
    get_file_or_folder_content, get_git_changes, get_git_commit_info, get_url_content,
    get_workspace_problems,
};
use self::types::{MentionContent, MentionType};

lazy_static! {
    /// メンション検出用の正規表現
    /// - `@/path/to/file` - ファイルパス
    /// - `@http://...` - URL
    /// - `@problems` - ワークスペースの問題
    /// - `@git-changes` - Git変更
    /// - `@1234567` - Gitコミットハッシュ (7-40文字の16進数)
    pub static ref MENTION_REGEX: Regex = Regex::new(r"@([^\s]+)").unwrap();
}

/// メンションを解析する
pub async fn parse_mentions(
    text: &str,
    browser_session: &mut BrowserSession,
    workspace_path: &Path,
) -> Result<String> {
    let mentions = extract_mentions(text);
    if mentions.is_empty() {
        return Ok(text.to_string());
    }

    let mut result = text.to_string();
    let diagnostics_provider = DiagnosticsProvider::new();

    for mention in mentions {
        let (mention_type, content) = if mention.starts_with("http") {
            let content = get_url_content(&mention, browser_session).await?;
            (MentionType::Url, content)
        } else if mention == "#git" {
            let content = get_git_changes(workspace_path).await?;
            (MentionType::GitChanges, content)
        } else if mention.starts_with("#git:") {
            let commit_hash = mention.trim_start_matches("#git:");
            let content = get_git_commit_info(commit_hash, workspace_path).await?;
            (MentionType::GitCommit, content)
        } else if mention == "#problems" {
            let content = get_workspace_problems(&diagnostics_provider).await?;
            (MentionType::Problems, content)
        } else {
            let content = get_file_or_folder_content(workspace_path, &mention).await?;
            if mention.ends_with('/') {
                (MentionType::Folder, content)
            } else {
                (MentionType::File, content)
            }
        };

        let mention_content = MentionContent {
            mention_type,
            value: mention.clone(),
            description: None,
        };

        result = result.replace(&mention, &format!("{} (see below for content)", mention));
        result.push_str(&format!("\n\n{}\n{}\n", mention_content.value, content));
    }

    Ok(result)
}

/// メンションを抽出する
fn extract_mentions(text: &str) -> Vec<String> {
    let mut mentions = Vec::new();
    let mut current_mention = String::new();
    let mut in_mention = false;

    for c in text.chars() {
        match c {
            '#' | 'h' => {
                if !in_mention {
                    in_mention = true;
                    current_mention.push(c);
                } else {
                    current_mention.push(c);
                }
            }
            ' ' | '\n' | '\t' => {
                if in_mention {
                    if !current_mention.is_empty() {
                        mentions.push(current_mention.clone());
                    }
                    current_mention.clear();
                    in_mention = false;
                }
            }
            _ => {
                if in_mention {
                    current_mention.push(c);
                }
            }
        }
    }

    if in_mention && !current_mention.is_empty() {
        mentions.push(current_mention);
    }

    mentions
}

/// メンションを処理する必要があるかどうかを判定する
pub fn should_process_mentions(text: &str) -> bool {
    text.contains('#') || text.contains("http")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mentions() {
        let text = "Check #src/main.rs and #tests/test.rs\nAlso #git and #git:abc123\nAnd https://example.com";
        let mentions = extract_mentions(text);
        assert_eq!(
            mentions,
            vec![
                "#src/main.rs",
                "#tests/test.rs",
                "#git",
                "#git:abc123",
                "https://example.com"
            ]
        );
    }

    #[test]
    fn test_should_process_mentions() {
        assert!(should_process_mentions("Check #src/main.rs"));
        assert!(should_process_mentions("Visit https://example.com"));
        assert!(!should_process_mentions("No mentions here"));
    }
}
