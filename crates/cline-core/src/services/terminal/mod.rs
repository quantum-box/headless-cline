use std::fmt::Debug;

use anyhow::Result;

pub trait TerminalManager: Debug + 'static {
    fn dispose_all(&mut self);
    fn get_or_create_terminal(&mut self, workspace_path: String) -> Result<TerminalInfo>;
    fn run_command(&mut self, terminal_info: TerminalInfo, command: String) -> Result<Process>;
    fn get_unretrieved_output(&mut self, terminal_id: u32) -> Option<String>;
    fn is_process_hot(&self, process_id: u32) -> bool;
    fn get_terminals(&self, busy_only: bool) -> Vec<TerminalInfo>;
}

#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub id: u32,
    pub last_command: String,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub struct Process {
    pub id: u32,
    pub command: String,
}
