use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Context,
    Add,
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub change_type: ChangeType,
    pub content: String,
    pub indent: String,
    pub original_line: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hunk {
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditResult {
    pub confidence: f64,
    pub result: Vec<String>,
    pub strategy: String,
}
