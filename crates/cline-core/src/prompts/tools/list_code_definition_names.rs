use crate::prompts::tools::types::ToolArgs;

#[allow(dead_code)]
pub fn get_list_code_definition_names_description(args: &ToolArgs) -> String {
    format!(
        r#"## list_code_definition_names
Description: Request to list definition names (classes, functions, methods, etc.) used in source code files at the top level of the specified directory. This tool provides insights into the codebase structure and important constructs, encapsulating high-level concepts and relationships that are crucial for understanding the overall architecture.
Parameters:
- path: (required) The path of the directory (relative to the current working directory {}) to list top level source code definitions for.
Usage:
<list_code_definition_names>
<path>Directory path here</path>
</list_code_definition_names>

Example: Requesting to list all top level source code definitions in the current directory
<list_code_definition_names>
<path>.</path>
</list_code_definition_names>"#,
        args.cwd
    )
}
