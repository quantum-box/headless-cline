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
            let path = mention.trim_start_matches('#');
            let content = get_file_or_folder_content(workspace_path, path).await?;
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

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let c = chars[i];
        match c {
            '#' => {
                // ハッシュが単語の先頭にある場合のみメンションとして扱う
                if i == 0 || chars[i - 1].is_whitespace() {
                    in_mention = true;
                    current_mention.push(c);
                }
            }
            'h' if text[i..].starts_with("http") => {
                in_mention = true;
                current_mention.push(c);
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

    // 最後のメンションを処理
    if in_mention && !current_mention.is_empty() {
        mentions.push(current_mention);
    }

    mentions
}

/// メンションを処理する必要があるかどうかを判定する
pub fn should_process_mentions(text: &str) -> bool {
    text.contains('#') && text.split_whitespace().any(|word| word.starts_with('#'))
        || text.contains("http")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // テスト用のヘルパー関数
    fn setup_test_browser() -> BrowserSession {
        BrowserSession::new()
    }

    #[test]
    fn test_extract_mentions() {
        let test_cases = vec![
            (
                "Check #src/main.rs and #tests/test.rs\nAlso #git and #git:abc123\nAnd https://example.com",
                vec!["#src/main.rs", "#tests/test.rs", "#git", "#git:abc123", "https://example.com"],
            ),
            (
                "No mentions here",
                vec![],
            ),
            (
                "#git #git:1234567 #problems",
                vec!["#git", "#git:1234567", "#problems"],
            ),
            (
                "Multiple #urls https://example1.com https://example2.com",
                vec!["#urls", "https://example1.com", "https://example2.com"],
            ),
            (
                "#folder/with/trailing/slash/ #file/without/slash",
                vec!["#folder/with/trailing/slash/", "#file/without/slash"],
            ),
            (
                "Text with hash# but not mention",
                vec![],
            ),
        ];

        for (input, expected) in test_cases {
            let mentions = extract_mentions(input);
            assert_eq!(mentions, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_should_process_mentions() {
        let test_cases = vec![
            ("Check #src/main.rs", true),
            ("Visit https://example.com", true),
            ("No mentions here", false),
            ("#git changes", true),
            ("Multiple #mentions #here", true),
            ("Text with hash# but not mention", false),
            ("https://multiple.com http://urls.com", true),
            ("", false),
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                should_process_mentions(input),
                expected,
                "Failed for input: {}",
                input
            );
        }
    }

    #[tokio::test]
    async fn test_parse_mentions_with_git() {
        let workspace_path = PathBuf::from("/test/workspace");
        let mut browser_session = setup_test_browser();
        let text = "Check #git and #git:abc123";

        let result = parse_mentions(text, &mut browser_session, &workspace_path).await;
        assert!(
            result.is_err(),
            "Should fail with non-existent git repository"
        );
    }

    #[tokio::test]
    async fn test_parse_mentions_with_problems() {
        let workspace_path = PathBuf::from("/test/workspace");
        let mut browser_session = setup_test_browser();
        let text = "Check #problems";

        let result = parse_mentions(text, &mut browser_session, &workspace_path).await;
        assert!(result.is_ok(), "Should succeed with empty diagnostics");

        let content = result.unwrap();
        assert!(
            content.contains("Check #problems"),
            "Should contain original text"
        );
    }

    #[tokio::test]
    async fn test_parse_mentions_with_url() {
        let workspace_path = PathBuf::from("/test/workspace");
        let mut browser_session = setup_test_browser();
        let text = "Check https://example.com";

        let result = parse_mentions(text, &mut browser_session, &workspace_path).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Browser not initialized"),
            "Expected 'Browser not initialized' error, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_parse_mentions_without_mentions() {
        let workspace_path = PathBuf::from("/test/workspace");
        let mut browser_session = setup_test_browser();
        let text = "No mentions in this text";

        let result = parse_mentions(text, &mut browser_session, &workspace_path).await;
        assert!(result.is_ok(), "Should succeed with no mentions");
        assert_eq!(
            result.unwrap(),
            text,
            "Should return original text unchanged"
        );
    }
}
