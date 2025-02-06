use crate::modes::{get_mode_by_slug, Mode, ModeConfig};

pub fn get_system_info_section(
    cwd: &str,
    mode: Mode,
    custom_modes: Option<&[ModeConfig]>,
) -> String {
    let _current_mode = get_mode_by_slug(mode, custom_modes);
    format!(
        "====\n\nSYSTEM INFORMATION\n\nOperating System: {}\nDefault Shell: {}\nHome Directory: {}\nCurrent Working Directory: {}\n\nWhen the user initially gives you a task, a recursive list of all filepaths in the current working directory ('{}') will be included in environment_details. This provides an overview of the project's file structure, offering key insights into the project from directory/file names (how developers conceptualize and organize their code) and file extensions (the language used). This can also guide decision-making on which files to explore further. If you need to further explore directories such as outside the current working directory, you can use the list_files tool. If you pass 'true' for the recursive parameter, it will list files recursively. Otherwise, it will list files at the top level, which is better suited for generic directories where you don't necessarily need the nested structure, like the Desktop.",
        std::env::consts::OS,
        std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string()),
        dirs::home_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        cwd,
        cwd
    )
}
