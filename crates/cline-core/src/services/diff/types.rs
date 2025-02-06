use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffResult {
    Success {
        content: String,
    },
    Failure {
        error: String,
        details: Option<DiffResultDetails>,
    },
}

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

use crate::prompts::tools::types::ToolArgs;
