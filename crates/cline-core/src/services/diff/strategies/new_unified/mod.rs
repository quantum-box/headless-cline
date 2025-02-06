mod edit_strategies;
mod search_strategies;
mod types;

use crate::services::diff::types::{DiffResult, DiffStrategy};
use crate::services::diff::types::ToolArgs;
use async_trait::async_trait;
use edit_strategies::apply_edit;
use search_strategies::{find_best_match, prepare_search_string};
use types::{Change, ChangeType, Diff, Hunk};

#[derive(Debug)]
pub struct NewUnifiedDiffStrategy {
    confidence_threshold: f64,
}

impl NewUnifiedDiffStrategy {
    pub fn new(confidence_threshold: Option<f64>) -> Self {
        Self {
            confidence_threshold: confidence_threshold.unwrap_or(1.0).max(0.8),
        }
    }

    fn parse_unified_diff(&self, diff: &str) -> Diff {
        const MAX_CONTEXT_LINES: usize = 6;
        let mut hunks = Vec::new();
        let mut current_hunk: Option<Hunk> = None;
        let lines: Vec<&str> = diff.lines().collect();

        let mut i = 0;
        while i < lines.len() && !lines[i].starts_with("@@") {
            i += 1;
        }

        for line in lines.iter().skip(i) {
            if line.starts_with("@@") {
                if let Some(hunk) = current_hunk.take() {
                    if hunk
                        .changes
                        .iter()
                        .any(|c| matches!(c.change_type, ChangeType::Add | ChangeType::Remove))
                    {
                        let changes = hunk.changes;
                        let mut start_idx = 0;
                        let mut end_idx = changes.len() - 1;

                        for (j, change) in changes.iter().enumerate() {
                            if !matches!(change.change_type, ChangeType::Context) {
                                start_idx = j.saturating_sub(MAX_CONTEXT_LINES);
                                break;
                            }
                        }

                        for (j, change) in changes.iter().enumerate().rev() {
                            if !matches!(change.change_type, ChangeType::Context) {
                                end_idx = (j + MAX_CONTEXT_LINES).min(changes.len() - 1);
                                break;
                            }
                        }

                        hunks.push(Hunk {
                            changes: changes[start_idx..=end_idx].to_vec(),
                        });
                    }
                }
                current_hunk = Some(Hunk {
                    changes: Vec::new(),
                });
                continue;
            }

            if let Some(ref mut hunk) = current_hunk {
                let content = &line[1..];
                let indent_match = content.matches(|c: char| c.is_whitespace()).next();
                let indent = indent_match.unwrap_or("").to_string();
                let trimmed_content = &content[indent.len()..];

                let change = match line.chars().next() {
                    Some(' ') => Change {
                        change_type: ChangeType::Context,
                        content: trimmed_content.to_string(),
                        indent,
                        original_line: Some(content.to_string()),
                    },
                    Some('+') => Change {
                        change_type: ChangeType::Add,
                        content: trimmed_content.to_string(),
                        indent,
                        original_line: Some(content.to_string()),
                    },
                    Some('-') => Change {
                        change_type: ChangeType::Remove,
                        content: trimmed_content.to_string(),
                        indent,
                        original_line: Some(content.to_string()),
                    },
                    _ => Change {
                        change_type: ChangeType::Context,
                        content: if trimmed_content.is_empty() {
                            " ".to_string()
                        } else {
                            format!(" {}", trimmed_content)
                        },
                        indent,
                        original_line: Some(content.to_string()),
                    },
                };

                hunk.changes.push(change);
            }
        }

        if let Some(hunk) = current_hunk {
            if hunk
                .changes
                .iter()
                .any(|c| matches!(c.change_type, ChangeType::Add | ChangeType::Remove))
            {
                hunks.push(hunk);
            }
        }

        Diff { hunks }
    }

    fn split_hunk(&self, hunk: &Hunk) -> Vec<Hunk> {
        let mut result = Vec::new();
        let mut current_hunk: Option<Hunk> = None;
        let mut context_before = Vec::new();
        let mut context_after = Vec::new();
        const MAX_CONTEXT_LINES: usize = 3;

        for change in &hunk.changes {
            match change.change_type {
                ChangeType::Context => {
                    if current_hunk.is_none() {
                        context_before.push(change.clone());
                        if context_before.len() > MAX_CONTEXT_LINES {
                            context_before.remove(0);
                        }
                    } else {
                        context_after.push(change.clone());
                        if context_after.len() > MAX_CONTEXT_LINES {
                            if let Some(hunk) = current_hunk.take() {
                                let mut changes = hunk.changes;
                                changes.extend(context_after.clone());
                                result.push(Hunk { changes });
                                context_before = context_after.clone();
                                context_after = Vec::new();
                            }
                        }
                    }
                }
                _ => {
                    if current_hunk.is_none() {
                        let changes = context_before.clone();
                        current_hunk = Some(Hunk { changes });
                        context_after = Vec::new();
                    } else if !context_after.is_empty() {
                        if let Some(ref mut hunk) = current_hunk {
                            hunk.changes.append(&mut context_after);
                        }
                    }
                    if let Some(ref mut hunk) = current_hunk {
                        hunk.changes.push(change.clone());
                    }
                }
            }
        }

        if let Some(mut hunk) = current_hunk {
            if !context_after.is_empty() {
                hunk.changes.extend(context_after);
            }
            result.push(hunk);
        }

        result
    }
}

#[async_trait]
impl DiffStrategy for NewUnifiedDiffStrategy {
    fn get_tool_description(&self, args: &services::diff::types::ToolArgs) -> String {
        format!(
            r#"# apply_diff Tool - Generate Precise Code Changes

Generate a unified diff that can be cleanly applied to modify code files.

## Step-by-Step Instructions:

1. Start with file headers:
   - First line: "--- {{original_file_path}}"
   - Second line: "+++ {{new_file_path}}"

2. For each change section:
   - Begin with "@@ ... @@" separator line without line numbers
   - Include 2-3 lines of context before and after changes
   - Mark removed lines with "-"
   - Mark added lines with "+"
   - Preserve exact indentation

3. Group related changes:
   - Keep related modifications in the same hunk
   - Start new hunks for logically separate changes
   - When modifying functions/methods, include the entire block

## Requirements:

1. MUST include exact indentation
2. MUST include sufficient context for unique matching
3. MUST group related changes together
4. MUST use proper unified diff format
5. MUST NOT include timestamps in file headers
6. MUST NOT include line numbers in the @@ header

Parameters:
- path: (required) File path relative to {}
- diff: (required) Unified diff content in unified format to apply to the file."#,
            args.cwd
        )
    }

    async fn apply_diff(
        &self,
        original_content: &str,
        diff_content: &str,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> DiffResult {
        let parsed_diff = self.parse_unified_diff(diff_content);
        let original_lines: Vec<String> = original_content.lines().map(String::from).collect();
        let mut result = original_lines.clone();

        if parsed_diff.hunks.is_empty() {
            return DiffResult::Failure {
                error: "No hunks found in diff. Please ensure your diff includes actual changes and follows the unified diff format.".to_string(),
                details: None,
            };
        }

        for hunk in &parsed_diff.hunks {
            let context_str = prepare_search_string(&hunk.changes);
            let search_result =
                find_best_match(&context_str, &result, 0, self.confidence_threshold);

            if search_result.confidence < self.confidence_threshold {
                let sub_hunks = self.split_hunk(hunk);
                let mut sub_hunk_success = true;
                let mut sub_hunk_result = result.clone();

                for sub_hunk in &sub_hunks {
                    let sub_context_str = prepare_search_string(&sub_hunk.changes);
                    let sub_search_result = find_best_match(
                        &sub_context_str,
                        &sub_hunk_result,
                        0,
                        self.confidence_threshold,
                    );

                    if sub_search_result.confidence >= self.confidence_threshold {
                        let sub_edit_result = apply_edit(
                            sub_hunk,
                            &sub_hunk_result,
                            sub_search_result.index,
                            sub_search_result.confidence,
                            Some(self.confidence_threshold),
                        )
                        .await;

                        if sub_edit_result.confidence >= self.confidence_threshold {
                            sub_hunk_result = sub_edit_result.result;
                            continue;
                        }
                    }
                    sub_hunk_success = false;
                    break;
                }

                if sub_hunk_success {
                    result = sub_hunk_result;
                    continue;
                }

                let context_lines = hunk
                    .changes
                    .iter()
                    .filter(|c| matches!(c.change_type, ChangeType::Context))
                    .count();
                let total_lines = hunk.changes.len();
                let context_ratio = context_lines as f64 / total_lines as f64;

                let mut error_msg = format!(
                    "Failed to find a matching location in the file ({}% confidence, needs {}%)\n\n",
                    (search_result.confidence * 100.0).floor(),
                    (self.confidence_threshold * 100.0).floor()
                );

                error_msg.push_str("Debug Info:\n");
                error_msg.push_str(&format!(
                    "- Search Strategy Used: {}\n",
                    search_result.strategy
                ));
                error_msg.push_str(&format!(
                    "- Context Lines: {} out of {} total lines ({}%)\n",
                    context_lines,
                    total_lines,
                    (context_ratio * 100.0).floor()
                ));
                error_msg.push_str(&format!(
                    "- Attempted to split into {} sub-hunks but still failed\n",
                    sub_hunks.len()
                ));

                if context_ratio < 0.2 {
                    error_msg.push_str("\nPossible Issues:\n");
                    error_msg
                        .push_str("- Not enough context lines to uniquely identify the location\n");
                    error_msg
                        .push_str("- Add a few more lines of unchanged code around your changes\n");
                } else if context_ratio > 0.5 {
                    error_msg.push_str("\nPossible Issues:\n");
                    error_msg.push_str("- Too many context lines may reduce search accuracy\n");
                    error_msg.push_str(
                        "- Try to keep only 2-3 lines of context before and after changes\n",
                    );
                } else {
                    error_msg.push_str("\nPossible Issues:\n");
                    error_msg
                        .push_str("- The diff may be targeting a different version of the file\n");
                    error_msg.push_str("- There may be too many changes in a single hunk, try splitting the changes into multiple hunks\n");
                }

                if start_line.is_some() && end_line.is_some() {
                    error_msg.push_str(&format!(
                        "\nSearch Range: lines {}-{}\n",
                        start_line.unwrap(),
                        end_line.unwrap()
                    ));
                }

                return DiffResult::Failure {
                    error: error_msg,
                    details: None,
                };
            }

            let edit_result = apply_edit(
                hunk,
                &result,
                search_result.index,
                search_result.confidence,
                Some(self.confidence_threshold),
            )
            .await;

            if edit_result.confidence >= self.confidence_threshold {
                result = edit_result.result;
            } else {
                let mut error_msg = format!(
                    "Failed to apply the edit using {} strategy ({}% confidence)\n\n",
                    edit_result.strategy,
                    (edit_result.confidence * 100.0).floor()
                );
                error_msg.push_str("Debug Info:\n");
                error_msg
                    .push_str("- The location was found but the content didn't match exactly\n");
                error_msg.push_str(
                    "- This usually means the file has been modified since the diff was created\n",
                );
                error_msg
                    .push_str("- Or the diff may be targeting a different version of the file\n");
                error_msg.push_str("\nPossible Solutions:\n");
                error_msg.push_str("1. Refresh your view of the file and create a new diff\n");
                error_msg.push_str(
                    "2. Double-check that the removed lines (-) match the current file content\n",
                );
                error_msg.push_str("3. Ensure your diff targets the correct version of the file");

                return DiffResult::Failure {
                    error: error_msg,
                    details: None,
                };
            }
        }

        DiffResult::Success {
            content: result.join("\n"),
        }
    }
}
