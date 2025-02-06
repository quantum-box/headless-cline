use crate::services::diff::types::{DiffResult, DiffStrategy, ToolArgs};
use async_trait::async_trait;
use diffy::apply;
use std::fmt;

#[derive(Default)]
pub struct UnifiedDiffStrategy;

impl UnifiedDiffStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for UnifiedDiffStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnifiedDiffStrategy").finish()
    }
}

#[async_trait]
impl DiffStrategy for UnifiedDiffStrategy {
    fn get_tool_description(&self, args: &ToolArgs) -> String {
        format!(
            r#"## apply_diff
Description: Apply a unified diff to a file at the specified path. This tool is useful when you need to make specific modifications to a file based on a set of changes provided in unified diff format (diff -U3).

Parameters:
- path: (required) The path of the file to apply the diff to (relative to the current working directory {})
- diff: (required) The diff content in unified format to apply to the file.

Format Requirements:

1. Header (REQUIRED):
    ```
    --- path/to/original/file
    +++ path/to/modified/file
    ```
    - Must include both lines exactly as shown
    - Use actual file paths
    - NO timestamps after paths

2. Hunks:
    ```
    @@ -lineStart,lineCount +lineStart,lineCount @@
    -removed line
    +added line
    ```
    - Each hunk starts with @@ showing line numbers for changes
    - Format: @@ -originalStart,originalCount +newStart,newCount @@
    - Use - for removed/changed lines
    - Use + for new/modified lines
    - Indentation must match exactly"#,
            args.cwd
        )
    }

    async fn apply_diff(
        &self,
        original_content: &str,
        diff_content: &str,
        _start_line: Option<usize>,
        _end_line: Option<usize>,
    ) -> DiffResult {
        match diffy::Patch::from_str(diff_content) {
            Ok(patch) => match diffy::apply(original_content, &patch) {
                Ok(result) => DiffResult::Success { content: result },
                Err(e) => DiffResult::Failure {
                    error: format!("Failed to apply unified diff: {}", e),
                    details: None,
                },
            },
            Err(e) => DiffResult::Failure {
                error: format!("Failed to parse unified diff: {}", e),
                details: None,
            },
        }
    }
}
