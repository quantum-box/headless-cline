use std::fmt::Debug;

use anyhow::Result;

pub trait TerminalManager: Debug + 'static {
    fn dispose_all(&mut self);
    fn get_or_create_terminal(&mut self, workspace_path: String) -> Result<TerminalInfo>;
    fn run_command(&mut self, terminal_info: TerminalInfo, command: String) -> Result<Process>;
}

pub struct TerminalInfo {
    pub id: u32,
    pub last_command: String,
}

pub struct Process {
    pub id: u32,
    pub command: String,
}
