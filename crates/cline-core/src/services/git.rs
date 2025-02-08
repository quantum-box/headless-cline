use anyhow::Result;
use std::path::Path;
use std::process::Output;
use tokio::process::Command;

pub struct GitService;

impl GitService {
    pub fn new() -> Self {
        Self
    }

    /// Gitコマンドを実行する
    async fn execute_git(&self, args: &[&str], cwd: &Path) -> Result<Output> {
        Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to execute git command: {}", e))
    }

    /// ワーキングディレクトリの変更状態を取得
    pub async fn get_working_state(&self, workspace_path: &Path) -> Result<String> {
        // Gitリポジトリかどうかを確認
        let git_dir = workspace_path.join(".git");
        if !git_dir.exists() {
            return Err(anyhow::anyhow!("Not a git repository"));
        }

        // git statusで変更状態を取得
        let status_output = self
            .execute_git(&["status", "--porcelain"], workspace_path)
            .await?;
        let status = String::from_utf8_lossy(&status_output.stdout);

        if status.is_empty() {
            return Ok("No git changes".to_string());
        }

        // git diffで詳細な差分を取得
        let diff_output = self.execute_git(&["diff"], workspace_path).await?;
        let diff = String::from_utf8_lossy(&diff_output.stdout);

        let mut result = String::new();
        result.push_str("git changes:\n\n");

        // 変更ファイルの一覧
        result.push_str("# Changed files\n");
        for line in status.lines() {
            if line.is_empty() {
                continue;
            }
            let status_code = &line[0..2];
            let file_path = &line[3..];

            let status_text = match status_code.trim() {
                "M" => "Modified",
                "A" => "Added",
                "D" => "Deleted",
                "R" => "Renamed",
                "C" => "Copied",
                "U" => "Updated but unmerged",
                "??" => "Untracked",
                _ => "Unknown status",
            };

            result.push_str(&format!("- {} ({})\n", file_path, status_text));
        }

        // 詳細な差分
        if !diff.is_empty() {
            result.push_str("\n# Detailed changes\n");
            result.push_str(&diff);
        }

        Ok(result)
    }

    /// コミット情報を取得
    pub async fn get_commit_info(
        &self,
        workspace_path: &Path,
        commit_hash: &str,
    ) -> Result<String> {
        // Gitリポジトリかどうかを確認
        let git_dir = workspace_path.join(".git");
        if !git_dir.exists() {
            return Err(anyhow::anyhow!("Not a git repository"));
        }

        // コミット情報を取得
        let show_output = self
            .execute_git(
                &[
                    "show",
                    "--format=%H%n%h%n%s%n%an%n%ad%n%b",
                    "--no-patch",
                    commit_hash,
                ],
                workspace_path,
            )
            .await?;
        let show = String::from_utf8_lossy(&show_output.stdout);
        let mut lines = show.lines();

        let full_hash = lines.next().unwrap_or_default();
        let short_hash = lines.next().unwrap_or_default();
        let subject = lines.next().unwrap_or_default();
        let author = lines.next().unwrap_or_default();
        let date = lines.next().unwrap_or_default();
        let body = lines.collect::<Vec<_>>().join("\n");

        // 変更統計を取得
        let stat_output = self
            .execute_git(
                &["show", "--stat", "--format=", commit_hash],
                workspace_path,
            )
            .await?;
        let stat = String::from_utf8_lossy(&stat_output.stdout);

        // 詳細な差分を取得
        let diff_output = self
            .execute_git(&["show", "--format=", commit_hash], workspace_path)
            .await?;
        let diff = String::from_utf8_lossy(&diff_output.stdout);

        let mut result = String::new();
        result.push_str(&format!("Commit: {} ({})\n", short_hash, full_hash));
        result.push_str(&format!("Author: {}\n", author));
        result.push_str(&format!("Date: {}\n\n", date));
        result.push_str(&format!("Message: {}\n", subject));
        if !body.is_empty() {
            result.push_str(&format!("\nDescription:\n{}\n", body));
        }
        result.push_str("\nFiles Changed:\n");
        result.push_str(&stat);
        result.push_str("\nFull Changes:\n");
        result.push_str(&diff);

        Ok(result)
    }
}
