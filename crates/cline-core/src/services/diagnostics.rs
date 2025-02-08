use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Debug, Default)]
pub struct DiagnosticsProvider {
    diagnostics: HashMap<PathBuf, Vec<Diagnostic>>,
}

impl DiagnosticsProvider {
    pub fn new() -> Self {
        Self {
            diagnostics: HashMap::new(),
        }
    }

    pub fn add_diagnostic(&mut self, file_path: PathBuf, diagnostic: Diagnostic) {
        self.diagnostics
            .entry(file_path)
            .or_default()
            .push(diagnostic);
    }

    pub fn get_diagnostics(&self, file_path: &Path) -> Option<&Vec<Diagnostic>> {
        self.diagnostics.get(file_path)
    }

    pub fn get_all_diagnostics(&self) -> &HashMap<PathBuf, Vec<Diagnostic>> {
        &self.diagnostics
    }

    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    pub fn format_diagnostics(&self) -> String {
        let mut result = String::new();

        for (path, diagnostics) in &self.diagnostics {
            let errors: Vec<_> = diagnostics
                .iter()
                .filter(|d| matches!(d.severity, DiagnosticSeverity::Error))
                .collect();

            if !errors.is_empty() {
                result.push_str(&format!("\n## {}", path.display()));
                for diagnostic in errors {
                    let source = diagnostic
                        .source
                        .as_ref()
                        .map(|s| format!("[{}] ", s))
                        .unwrap_or_default();
                    result.push_str(&format!(
                        "\n- {}Line {}: {}",
                        source, diagnostic.line, diagnostic.message
                    ));
                }
            }
        }

        if result.is_empty() {
            "(No errors detected)".to_string()
        } else {
            result.trim().to_string()
        }
    }
}
