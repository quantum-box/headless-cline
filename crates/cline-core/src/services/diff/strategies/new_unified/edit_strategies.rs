use super::search_strategies::validate_edit_result;
use super::types::{Change, ChangeType, EditResult, Hunk};
use std::path::Path;
use tempfile::tempdir;
use tokio::fs;
use tokio::process::Command;

pub async fn apply_context_matching(
    hunk: &Hunk,
    content: &[String],
    match_position: i32,
) -> EditResult {
    if match_position == -1 {
        return EditResult {
            confidence: 0.0,
            result: content.to_vec(),
            strategy: "context".to_string(),
        };
    }

    let mut new_result = content[..match_position as usize].to_vec();
    let mut source_index = match_position as usize;

    for change in &hunk.changes {
        match change.change_type {
            ChangeType::Context => {
                if source_index < content.len() {
                    new_result.push(content[source_index].clone());
                } else {
                    let line = if !change.indent.is_empty() {
                        format!("{}{}", change.indent, change.content)
                    } else {
                        change.content.clone()
                    };
                    new_result.push(line);
                }
                source_index += 1;
            }
            ChangeType::Add => {
                let base_indent = change.indent.clone();
                let lines = change.content.lines().map(|line| {
                    let line_indent_match = line.matches(|c: char| c.is_whitespace()).next();
                    if let Some(line_indent) = line_indent_match {
                        if line_indent.is_empty() {
                            format!("{}{}", base_indent, line)
                        } else {
                            line.to_string()
                        }
                    } else {
                        format!("{}{}", base_indent, line)
                    }
                });
                new_result.extend(lines);
            }
            ChangeType::Remove => {
                let removed_lines = change.content.lines().count();
                source_index += removed_lines;
            }
        }
    }

    new_result.extend_from_slice(&content[source_index..]);

    let after_text = new_result
        [match_position as usize..new_result.len() - (content.len() - source_index)]
        .join("\n");

    let confidence = validate_edit_result(hunk, &after_text);

    EditResult {
        confidence,
        result: new_result,
        strategy: "context".to_string(),
    }
}

pub async fn apply_git_fallback(hunk: &Hunk, content: &[String]) -> EditResult {
    let temp_dir = match tempdir() {
        Ok(dir) => dir,
        Err(_) => {
            return EditResult {
                confidence: 0.0,
                result: content.to_vec(),
                strategy: "git-fallback".to_string(),
            };
        }
    };

    let file_path = temp_dir.path().join("file.txt");

    // Initialize git repository
    if let Err(_) = Command::new("git")
        .arg("init")
        .current_dir(&temp_dir)
        .output()
        .await
    {
        return EditResult {
            confidence: 0.0,
            result: content.to_vec(),
            strategy: "git-fallback".to_string(),
        };
    }

    // Configure git
    for cmd in &[vec!["config", "user.name", "Temp"], vec![
        "config",
        "user.email",
        "temp@example.com",
    ]] {
        if let Err(_) = Command::new("git")
            .args(cmd)
            .current_dir(&temp_dir)
            .output()
            .await
        {
            return EditResult {
                confidence: 0.0,
                result: content.to_vec(),
                strategy: "git-fallback".to_string(),
            };
        }
    }

    // Create search and replace content
    let search_lines: Vec<String> = hunk
        .changes
        .iter()
        .filter(|c| matches!(c.change_type, ChangeType::Context | ChangeType::Remove))
        .map(|c| {
            if let Some(line) = &c.original_line {
                line.clone()
            } else {
                format!("{}{}", c.indent, c.content)
            }
        })
        .collect();

    let replace_lines: Vec<String> = hunk
        .changes
        .iter()
        .filter(|c| matches!(c.change_type, ChangeType::Context | ChangeType::Add))
        .map(|c| {
            if let Some(line) = &c.original_line {
                line.clone()
            } else {
                format!("{}{}", c.indent, c.content)
            }
        })
        .collect();

    let original_text = content.join("\n");
    let search_text = search_lines.join("\n");
    let replace_text = replace_lines.join("\n");

    // Try first strategy
    if let Ok(_) = fs::write(&file_path, &original_text).await {
        if let Ok(_) = Command::new("git")
            .args(&["add", "file.txt"])
            .current_dir(&temp_dir)
            .output()
            .await
        {
            if let Ok(output) = Command::new("git")
                .args(&["commit", "-m", "original"])
                .current_dir(&temp_dir)
                .output()
                .await
            {
                let original_commit = String::from_utf8_lossy(&output.stdout);
                if let Ok(_) = fs::write(&file_path, &search_text).await {
                    if let Ok(_) = Command::new("git")
                        .args(&["add", "file.txt"])
                        .current_dir(&temp_dir)
                        .output()
                        .await
                    {
                        if let Ok(output) = Command::new("git")
                            .args(&["commit", "-m", "search"])
                            .current_dir(&temp_dir)
                            .output()
                            .await
                        {
                            let search_commit = String::from_utf8_lossy(&output.stdout);
                            if let Ok(_) = fs::write(&file_path, &replace_text).await {
                                if let Ok(_) = Command::new("git")
                                    .args(&["add", "file.txt"])
                                    .current_dir(&temp_dir)
                                    .output()
                                    .await
                                {
                                    if let Ok(output) = Command::new("git")
                                        .args(&["commit", "-m", "replace"])
                                        .current_dir(&temp_dir)
                                        .output()
                                        .await
                                    {
                                        let replace_commit =
                                            String::from_utf8_lossy(&output.stdout);
                                        if let Ok(_) = Command::new("git")
                                            .args(&["checkout", &original_commit])
                                            .current_dir(&temp_dir)
                                            .output()
                                            .await
                                        {
                                            if let Ok(_) = Command::new("git")
                                                .args(&[
                                                    "cherry-pick",
                                                    "--minimal",
                                                    &replace_commit,
                                                ])
                                                .current_dir(&temp_dir)
                                                .output()
                                                .await
                                            {
                                                if let Ok(new_text) =
                                                    fs::read_to_string(&file_path).await
                                                {
                                                    return EditResult {
                                                        confidence: 1.0,
                                                        result: new_text
                                                            .lines()
                                                            .map(String::from)
                                                            .collect(),
                                                        strategy: "git-fallback".to_string(),
                                                    };
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    EditResult {
        confidence: 0.0,
        result: content.to_vec(),
        strategy: "git-fallback".to_string(),
    }
}

pub async fn apply_edit(
    hunk: &Hunk,
    content: &[String],
    match_position: i32,
    confidence: f64,
    confidence_threshold: Option<f64>,
) -> EditResult {
    let confidence_threshold = confidence_threshold.unwrap_or(0.97);

    if confidence < confidence_threshold {
        return apply_git_fallback(hunk, content).await;
    }

    let context_result = apply_context_matching(hunk, content, match_position).await;
    if context_result.confidence >= confidence_threshold {
        return context_result;
    }

    apply_git_fallback(hunk, content).await
}
