use similar::{ChangeTag, TextDiff};
use std::path::{Path, PathBuf};

use crate::services::ai::{ContentBlock, ImageBlock, ImageSource, TextBlock};

pub struct FormatResponse;

impl FormatResponse {
    pub fn tool_denied() -> String {
        "The user denied this operation.".to_string()
    }

    pub fn tool_denied_with_feedback(feedback: Option<&str>) -> String {
        format!(
            "The user denied this operation and provided the following feedback:\n<feedback>\n{}\n</feedback>",
            feedback.unwrap_or("")
        )
    }

    pub fn tool_approved_with_feedback(feedback: Option<&str>) -> String {
        format!(
            "The user approved this operation and provided the following context:\n<feedback>\n{}\n</feedback>",
            feedback.unwrap_or("")
        )
    }

    pub fn tool_error(error: Option<&str>) -> String {
        format!(
            "The tool execution failed with the following error:\n<error>\n{}\n</error>",
            error.unwrap_or("")
        )
    }

    pub fn no_tools_used() -> String {
        format!(
            "[ERROR] You did not use a tool in your previous response! Please retry with a tool use.\n\n{}\n\n# Next Steps\n\nIf you have completed the user's task, use the attempt_completion tool. \nIf you require additional information from the user, use the ask_followup_question tool. \nOtherwise, if you have not completed the task and do not need additional information, then proceed with the next step of the task. \n(This is an automated message, so do not respond to it conversationally.)",
            TOOL_USE_INSTRUCTIONS_REMINDER
        )
    }

    pub fn too_many_mistakes(feedback: Option<&str>) -> String {
        format!(
            "You seem to be having trouble proceeding. The user has provided the following feedback to help guide you:\n<feedback>\n{}\n</feedback>",
            feedback.unwrap_or("")
        )
    }

    pub fn missing_tool_parameter_error(param_name: &str) -> String {
        format!(
            "Missing value for required parameter '{}'. Please retry with complete response.\n\n{}",
            param_name, TOOL_USE_INSTRUCTIONS_REMINDER
        )
    }

    pub fn invalid_mcp_tool_argument_error(server_name: &str, tool_name: &str) -> String {
        format!(
            "Invalid JSON argument used with {} for {}. Please retry with a properly formatted JSON argument.",
            server_name, tool_name
        )
    }

    pub fn tool_result(text: &str, images: Option<Vec<String>>) -> ToolResult {
        if let Some(images) = images {
            if !images.is_empty() {
                let text_block = TextBlock {
                    text: text.to_string(),
                };
                let image_blocks = format_images_into_blocks(&images);
                ToolResult::Blocks(
                    vec![ContentBlock::Text(text_block)]
                        .into_iter()
                        .chain(image_blocks)
                        .collect(),
                )
            } else {
                ToolResult::Text(text.to_string())
            }
        } else {
            ToolResult::Text(text.to_string())
        }
    }

    pub fn image_blocks(images: Option<Vec<String>>) -> Vec<ContentBlock> {
        format_images_into_blocks(&images.unwrap_or_default())
    }

    pub fn format_files_list(
        absolute_path: &Path,
        files: &[String],
        did_hit_limit: bool,
    ) -> String {
        let mut sorted = files
            .iter()
            .map(|file| {
                let path = PathBuf::from(file);
                let relative_path = path
                    .strip_prefix(absolute_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                if file.ends_with('/') {
                    format!("{}/", relative_path)
                } else {
                    relative_path.to_string()
                }
            })
            .collect::<Vec<String>>();

        sorted.sort_by(|a, b| {
            let a_parts: Vec<&str> = a.split('/').collect();
            let b_parts: Vec<&str> = b.split('/').collect();
            for i in 0..a_parts.len().min(b_parts.len()) {
                if a_parts[i] != b_parts[i] {
                    if i + 1 == a_parts.len() && i + 1 < b_parts.len() {
                        return std::cmp::Ordering::Less;
                    }
                    if i + 1 == b_parts.len() && i + 1 < a_parts.len() {
                        return std::cmp::Ordering::Greater;
                    }
                    return a_parts[i].to_lowercase().cmp(&b_parts[i].to_lowercase());
                }
            }
            a_parts.len().cmp(&b_parts.len())
        });

        if did_hit_limit {
            format!(
                "{}\n\n(File list truncated. Use list_files on specific subdirectories if you need to explore further.)",
                sorted.join("\n")
            )
        } else if sorted.is_empty() || (sorted.len() == 1 && sorted[0].is_empty()) {
            "No files found.".to_string()
        } else {
            sorted.join("\n")
        }
    }

    pub fn create_pretty_patch(
        filename: Option<&str>,
        old_str: Option<&str>,
        new_str: Option<&str>,
    ) -> String {
        let filename = filename.unwrap_or("file").replace('\\', "/");
        let old_str = old_str.unwrap_or("");
        let new_str = new_str.unwrap_or("");

        let diff = TextDiff::from_lines(old_str, new_str);
        let mut result = Vec::new();

        for change in diff.iter_all_changes() {
            let (sign, text) = match change.tag() {
                ChangeTag::Delete => ("-", change.as_str().unwrap_or("")),
                ChangeTag::Insert => ("+", change.as_str().unwrap_or("")),
                ChangeTag::Equal => (" ", change.as_str().unwrap_or("")),
            };
            result.push(format!("{}{}", sign, text));
        }

        result.join("")
    }
}

pub fn format_images_into_blocks(images: &[String]) -> Vec<ContentBlock> {
    images
        .iter()
        .filter_map(|data_url| {
            let parts: Vec<&str> = data_url.split(',').collect();
            if parts.len() != 2 {
                return None;
            }

            let rest = parts[0];
            let base64 = parts[1];
            let mime_type = rest.split(':').nth(1)?.split(';').next()?.to_string();

            Some(ContentBlock::Image(ImageBlock {
                source: ImageSource {
                    source_type: "base64".to_string(),
                    media_type: mime_type,
                    data: base64.to_string(),
                },
            }))
        })
        .collect()
}

#[derive(Debug)]
pub enum ToolResult {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug)]
pub enum Block {
    Text(TextBlock),
    Image(ImageBlock),
}

pub const TOOL_USE_INSTRUCTIONS_REMINDER: &str = r#"# Reminder: Instructions for Tool Use

Tool uses are formatted using XML-style tags. The tool name is enclosed in opening and closing tags, and each parameter is similarly enclosed within its own set of tags. Here's the structure:

<tool_name>
<parameter1_name>value1</parameter1_name>
<parameter2_name>value2</parameter2_name>
...
</tool_name>

For example:

<attempt_completion>
<result>
I have completed the task...
</result>
</attempt_completion>

Always adhere to this format for all tool uses to ensure proper parsing and execution."#;

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_tool_denied() {
        assert_eq!(
            FormatResponse::tool_denied(),
            "The user denied this operation."
        );
    }

    #[test]
    fn test_tool_denied_with_feedback() {
        assert_eq!(
            FormatResponse::tool_denied_with_feedback(Some("Test feedback")),
            "The user denied this operation and provided the following feedback:\n<feedback>\nTest feedback\n</feedback>"
        );
    }

    #[test]
    fn test_format_files_list() {
        let absolute_path = Path::new("/test/path");
        let files = vec![
            "/test/path/dir2/file2".to_string(),
            "/test/path/dir1/".to_string(),
            "/test/path/file1".to_string(),
        ];
        let result = FormatResponse::format_files_list(absolute_path, &files, false);
        assert_eq!(result, "dir1/\ndir2/file2\nfile1");
    }

    #[test]
    fn test_format_files_list_with_limit() {
        let absolute_path = Path::new("/test/path");
        let files = vec!["/test/path/file1".to_string()];
        let result = FormatResponse::format_files_list(absolute_path, &files, true);
        assert_eq!(
            result,
            "file1\n\n(File list truncated. Use list_files on specific subdirectories if you need to explore further.)"
        );
    }

    #[test]
    fn test_create_pretty_patch() {
        let old_str = "line1\nline2\n";
        let new_str = "line1\nline3\n";
        let result =
            FormatResponse::create_pretty_patch(Some("test.txt"), Some(old_str), Some(new_str));
        assert!(result.contains("-line2"));
        assert!(result.contains("+line3"));
    }
}
