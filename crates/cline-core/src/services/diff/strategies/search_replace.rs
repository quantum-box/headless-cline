use crate::services::diff::types::{DiffResult, DiffResultDetails, DiffStrategy};
use crate::prompts::tools::types::ToolArgs;
use async_trait::async_trait;
use strsim::normalized_levenshtein;

const BUFFER_LINES: usize = 20;

#[derive(Debug)]
pub struct SearchReplaceDiffStrategy {
    fuzzy_threshold: f64,
    buffer_lines: usize,
}

impl SearchReplaceDiffStrategy {
    pub fn new(fuzzy_threshold: Option<f64>, buffer_lines: Option<usize>) -> Self {
        Self {
            fuzzy_threshold: fuzzy_threshold.unwrap_or(1.0),
            buffer_lines: buffer_lines.unwrap_or(BUFFER_LINES),
        }
    }

    fn get_similarity(original: &str, search: &str) -> f64 {
        if search.is_empty() {
            return 1.0;
        }

        let normalized_original = original
            .replace(char::is_whitespace, " ")
            .trim()
            .to_string();
        let normalized_search = search.replace(char::is_whitespace, " ").trim().to_string();

        if normalized_original == normalized_search {
            return 1.0;
        }

        normalized_levenshtein(&normalized_original, &normalized_search)
    }
}

#[async_trait]
impl DiffStrategy for SearchReplaceDiffStrategy {
    fn get_tool_description(&self, args: &ToolArgs) -> String {
        format!(
            r#"## apply_diff
Description: Request to replace existing code using a search and replace block.
This tool allows for precise, surgical replaces to files by specifying exactly what content to search for and what to replace it with.
The tool will maintain proper indentation and formatting while making changes.
Only a single operation is allowed per tool use.
The SEARCH section must exactly match existing content including whitespace and indentation.
If you're not confident in the exact content to search for, use the read_file tool first to get the exact content.
When applying the diffs, be extra careful to remember to change any closing brackets or other syntax that may be affected by the diff farther down in the file.

Parameters:
- path: (required) The path of the file to modify (relative to the current working directory {})
- diff: (required) The search/replace block defining the changes.
- start_line: (required) The line number where the search block starts.
- end_line: (required) The line number where the search block ends.

Diff format:
```
<<<<<<< SEARCH
[exact content to find including whitespace]
=======
[new content to replace with]
>>>>>>> REPLACE
```"#,
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
        let re = match regex::Regex::new(
            r"<<<<<<< SEARCH\n([\s\S]*?)\n=======\n([\s\S]*?)\n>>>>>>> REPLACE",
        ) {
            Ok(re) => re,
            Err(e) => {
                return DiffResult::Failure {
                    error: format!("Invalid regex pattern: {}", e),
                    details: None,
                };
            }
        };

        let captures = match re.captures(diff_content) {
            Some(captures) => captures,
            None => {
                return DiffResult::Failure {
                    error: "Invalid diff format - missing required SEARCH/REPLACE sections"
                        .to_string(),
                    details: None,
                };
            }
        };

        let search_content = captures.get(1).unwrap().as_str();
        let replace_content = captures.get(2).unwrap().as_str();

        let original_lines: Vec<&str> = original_content.lines().collect();
        let search_lines: Vec<&str> = search_content.lines().collect();

        if search_lines.is_empty() && start_line.is_none() {
            return DiffResult::Failure {
                error: "Empty search content requires start_line to be specified".to_string(),
                details: None,
            };
        }

        if search_lines.is_empty() && start_line != end_line {
            return DiffResult::Failure {
                error: format!(
                    "Empty search content requires start_line and end_line to be the same (got {}-{})",
                    start_line.unwrap_or(0),
                    end_line.unwrap_or(0)
                ),
                details: None,
            };
        }

        let mut best_match_index = None;
        let mut best_match_score = 0.0;

        if let (Some(start), Some(end)) = (start_line, end_line) {
            let start_idx = start.saturating_sub(1);
            let end_idx = end.min(original_lines.len());

            if start_idx >= original_lines.len()
                || end_idx > original_lines.len()
                || start_idx > end_idx
            {
                return DiffResult::Failure {
                    error: format!(
                        "Line range {}-{} is invalid (file has {} lines)",
                        start,
                        end,
                        original_lines.len()
                    ),
                    details: None,
                };
            }

            let window = &original_lines[start_idx..end_idx];
            let window_str = window.join("\n");
            let similarity = Self::get_similarity(&window_str, search_content);

            if similarity >= self.fuzzy_threshold {
                best_match_index = Some(start_idx);
                best_match_score = similarity;
            }
        }

        if best_match_index.is_none() {
            let search_range_start = start_line.map(|l| l.saturating_sub(1)).unwrap_or(0);
            let search_range_end = end_line.unwrap_or(original_lines.len());

            for i in search_range_start..=search_range_end.saturating_sub(search_lines.len()) {
                let window = &original_lines[i..i + search_lines.len()];
                let window_str = window.join("\n");
                let similarity = Self::get_similarity(&window_str, search_content);

                if similarity > best_match_score {
                    best_match_score = similarity;
                    best_match_index = Some(i);
                }
            }
        }

        if best_match_index.is_none() || best_match_score < self.fuzzy_threshold {
            return DiffResult::Failure {
                error: format!(
                    "No sufficiently similar match found ({}% similar, needs {}%)",
                    (best_match_score * 100.0) as i32,
                    (self.fuzzy_threshold * 100.0) as i32
                ),
                details: Some(DiffResultDetails {
                    similarity: Some(best_match_score),
                    threshold: Some(self.fuzzy_threshold),
                    matched_range: None,
                    search_content: Some(search_content.to_string()),
                    best_match: None,
                }),
            };
        }

        let match_index = best_match_index.unwrap();
        let mut result = Vec::new();
        result.extend_from_slice(&original_lines[..match_index]);
        result.extend(replace_content.lines());
        result.extend_from_slice(&original_lines[match_index + search_lines.len()..]);

        DiffResult::Success {
            content: result.join("\n"),
        }
    }
}
