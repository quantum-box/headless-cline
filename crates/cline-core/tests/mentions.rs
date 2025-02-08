use anyhow::Result;
use cline_core::mentions::{parse_mentions, should_process_mentions};
use cline_core::services::browser::BrowserSession;
use std::fs;
use tempfile::TempDir;

// テスト用のヘルパー関数
async fn setup_test_browser() -> Result<BrowserSession> {
    let mut browser_session = BrowserSession::new();
    // CIでは--no-sandboxオプションが必要
    if std::env::var("CI").is_ok() {
        browser_session.set_chrome_args(vec!["--no-sandbox", "--headless"]);
    } else {
        // ローカルでのテスト用
        browser_session.set_chrome_args(vec!["--headless"]);
    }
    // Chromeのパスを設定
    std::env::set_var("CHROME_PATH", "/usr/bin/google-chrome");
    browser_session.launch_browser().await?;
    Ok(browser_session)
}

fn setup_test_workspace() -> TempDir {
    let temp_dir = tempfile::tempdir().unwrap();

    // テスト用のファイルを作成
    fs::write(temp_dir.path().join("test.txt"), "This is a test file").unwrap();

    // テスト用のディレクトリを作成
    fs::create_dir(temp_dir.path().join("test_dir")).unwrap();
    fs::write(
        temp_dir.path().join("test_dir").join("nested.txt"),
        "This is a nested file",
    )
    .unwrap();

    // Gitリポジトリを初期化
    let repo = git2::Repository::init(temp_dir.path()).unwrap();
    let mut index = repo.index().unwrap();

    // 全てのファイルをステージングに追加
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();

    // 初期コミットを作成
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .unwrap();

    // Gitの設定を追加（CIで必要）
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    temp_dir
}

#[tokio::test]
async fn test_parse_mentions_integration() -> Result<()> {
    let temp_dir = setup_test_workspace();
    let workspace_path = temp_dir.path();
    let mut browser_session = setup_test_browser().await?;

    // 単一のファイルメンションのテスト
    let text = "Check #test.txt";
    let result = parse_mentions(text, &mut browser_session, workspace_path).await?;
    assert!(
        result.contains("This is a test file"),
        "File content should be included in the result"
    );

    Ok(())
}

#[tokio::test]
async fn test_parse_mentions_with_invalid_paths() -> Result<()> {
    let temp_dir = setup_test_workspace();
    let workspace_path = temp_dir.path();
    let mut browser_session = setup_test_browser().await?;

    // 存在しないファイルのテスト
    let text = "Check #nonexistent.txt";
    let result = parse_mentions(text, &mut browser_session, workspace_path).await;
    assert!(result.is_err(), "Should fail with non-existent file");

    Ok(())
}

#[tokio::test]
async fn test_parse_mentions_with_problems() -> Result<()> {
    let temp_dir = setup_test_workspace();
    let workspace_path = temp_dir.path();
    let mut browser_session = setup_test_browser().await?;

    let text = "Check #problems";
    let result = parse_mentions(text, &mut browser_session, workspace_path).await?;
    assert!(
        result.contains("#problems"),
        "Original mention should be included in the result"
    );
    assert!(
        result.contains("(see below for content)"),
        "Mention should be marked as having content"
    );

    Ok(())
}

#[tokio::test]
async fn test_parse_mentions_with_urls() -> Result<()> {
    let temp_dir = setup_test_workspace();
    let workspace_path = temp_dir.path();
    let mut browser_session = setup_test_browser().await?;

    // 単一のURLメンションのテスト
    let text = "Check https://example.com";
    let result = parse_mentions(text, &mut browser_session, workspace_path).await?;
    assert!(
        result.contains("Example Domain"),
        "URL content should include the page title"
    );

    // 複数のURLメンションのテスト
    let text = "Check https://example.com and https://www.rust-lang.org";
    let result = parse_mentions(text, &mut browser_session, workspace_path).await?;
    assert!(
        result.contains("Example Domain") && result.contains("Rust Programming Language"),
        "Both URL contents should be included"
    );

    // 無効なURLのテスト
    let text = "Check https://invalid.example.com";
    let result = parse_mentions(text, &mut browser_session, workspace_path).await;
    assert!(result.is_err(), "Should fail with invalid URL");

    Ok(())
}

#[tokio::test]
async fn test_git_functionality() -> Result<()> {
    let temp_dir = setup_test_workspace();
    let workspace_path = temp_dir.path();
    let mut browser_session = setup_test_browser().await?;

    // 1. 初期状態のテスト（変更なし）
    let text = "Check #git";
    let result = parse_mentions(text, &mut browser_session, workspace_path).await?;
    println!("\n=== 初期状態のGit結果 ===\n{}\n", result);
    assert!(
        result.contains("No git changes"),
        "Should show no changes initially"
    );

    // 2. ファイル変更のテスト
    fs::write(workspace_path.join("test.txt"), "Modified content").unwrap();

    let result = parse_mentions(text, &mut browser_session, workspace_path).await?;
    println!("\n=== ファイル変更後のGit結果 ===\n{}\n", result);
    assert!(
        result.contains("git changes:"),
        "Should show git changes header"
    );
    assert!(
        result.contains("test.txt (Modified)"),
        "Should show modified file"
    );

    // 3. 新規ファイル追加のテスト
    fs::write(workspace_path.join("new_file.txt"), "New file content").unwrap();

    let result = parse_mentions(text, &mut browser_session, workspace_path).await?;
    println!("\n=== 新規ファイル追加後のGit結果 ===\n{}\n", result);
    assert!(
        result.contains("new_file.txt (Untracked)"),
        "Should show untracked file"
    );

    // 4. コミットハッシュのテスト
    let repo = git2::Repository::open(workspace_path)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let commit_hash = commit.id().to_string();

    let text = format!("Check #git:{}", &commit_hash[..7]);
    let result = parse_mentions(&text, &mut browser_session, workspace_path).await?;
    println!("\n=== コミット情報の結果 ===\n{}\n", result);
    assert!(
        result.contains("Initial commit"),
        "Should show commit message"
    );
    assert!(result.contains("test.txt"), "Should show committed file");

    // 5. 無効なGitリポジトリのテスト
    let invalid_dir = tempfile::tempdir()?;
    let text = "Check #git";
    let result = parse_mentions(text, &mut browser_session, invalid_dir.path()).await;
    println!("\n=== 無効なGitリポジトリのエラー ===\n{:?}\n", result);
    assert!(result.is_err(), "Should fail with non-git directory");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Not a git repository"),
        "Should show appropriate error message"
    );

    Ok(())
}

#[tokio::test]
async fn test_parse_mentions_with_mixed_content() -> Result<()> {
    let temp_dir = setup_test_workspace();
    let workspace_path = temp_dir.path();
    let mut browser_session = setup_test_browser().await?;

    // ファイル、URL、Git変更の組み合わせテスト
    let text = "Check #test.txt and https://example.com";
    let result = parse_mentions(text, &mut browser_session, workspace_path).await?;
    println!("\n=== ファイルとURLの組み合わせ結果 ===\n{}\n", result);

    assert!(
        result.contains("This is a test file"),
        "File content should be included"
    );
    assert!(
        result.contains("Example Domain"),
        "URL content should be included"
    );

    // Gitの変更を追加
    fs::write(workspace_path.join("test.txt"), "Modified content").unwrap();
    let text = "Check #git";
    let result = parse_mentions(text, &mut browser_session, workspace_path).await?;
    println!("\n=== Git変更の結果 ===\n{}\n", result);
    assert!(
        result.contains("git changes:") && result.contains("test.txt (Modified)"),
        "Git changes should show modified file"
    );

    Ok(())
}

#[test]
fn test_should_process_mentions_integration() {
    assert!(should_process_mentions("Check #test.txt"));
    assert!(should_process_mentions("Check https://example.com"));
    assert!(!should_process_mentions("No mentions here"));
    assert!(should_process_mentions("#git changes"));
    assert!(should_process_mentions("Multiple #mentions #here"));
    assert!(!should_process_mentions("Text with hash# but not mention"));
}
