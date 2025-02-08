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
        // git statusで変更状態を取得
        let status_output = self
            .execute_git(&["status", "--porcelain"], workspace_path)
            .await?;
        let status = String::from_utf8_lossy(&status_output.stdout);

        if status.is_empty() {
            return Ok("No changes in working directory".to_string());
        }

        // git diffで詳細な差分を取得
        let diff_output = self.execute_git(&["diff", "HEAD"], workspace_path).await?;
        let diff = String::from_utf8_lossy(&diff_output.stdout);

        let mut result = String::new();

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
            result.push_str("```diff\n");
            result.push_str(&diff);
            result.push_str("```");
        }

        Ok(result)
    }

    /// コミット情報を取得
    pub async fn get_commit_info(
        &self,
        commit_hash: &str,
        workspace_path: &Path,
    ) -> Result<String> {
        let args = &[
            "show",
            "--no-patch",
            "--format=%H %s%n%nAuthor: %an%nDate: %aD%n%n%b",
            commit_hash,
        ];

        let output = self.execute_git(args, workspace_path).await?;
        let commit_info = String::from_utf8_lossy(&output.stdout);

        if commit_info.trim().is_empty() {
            return Err(anyhow::anyhow!("Commit not found: {}", commit_hash));
        }

        Ok(commit_info.to_string())
    }
}
