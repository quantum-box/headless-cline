use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiffResultDetails {
    pub similarity: Option<f64>,
    pub threshold: Option<f64>,
    pub matched_range: Option<MatchedRange>,
    pub search_content: Option<String>,
    pub best_match: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedRange {
    pub start: usize,
    pub end: usize,
}

pub type DiffResult = Result<String, String>;

#[async_trait]
pub trait DiffStrategy: Debug {
    fn get_tool_description(&self, args: &ToolArgs) -> String;
    async fn apply_diff(
        &self,
        original_content: &str,
        diff_content: &str,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> DiffResult;
}

pub struct ToolArgs {
    pub cwd: String,
    pub tool_options: Option<std::collections::HashMap<String, String>>,
}
