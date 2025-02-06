use crate::prompts::tools::types::ToolArgs;

pub fn get_read_file_description(args: &ToolArgs) -> String {
    format!(
        r##"## read_file
Description: Request to read the contents of a file at the specified path. Use this when you need to examine the contents of an existing file you do not know the contents of, for example to analyze code, review text files, or extract information from configuration files. The output includes line numbers prefixed to each line (e.g. "1 | const x = 1"), making it easier to reference specific lines when creating diffs or discussing code. Automatically extracts raw text from PDF and DOCX files. May not be suitable for other types of binary files, as it returns the raw content as a string.
Parameters:
- path: (required) The path of the file to read (relative to the current working directory {})
Usage:
<read_file>
<path>File path here</path>
</read_file>

Example: Requesting to read frontend-config.json
<read_file>
<path>frontend-config.json</path>
</read_file>"##,
        args.cwd
    )
}
