use std::io::Write;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "history",
    description: "Display the history list with line numbers.",
    usage: "history\n\nDisplay the history list with line numbers. Lines listed with a `*` have been modified.",
    run: history_runner,
};

// We will use standard stdout redirect logic in execution.rs later, but for now we'll match history's previous custom signature minimally
pub fn history_runner(_args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    run(state, None); // Redirs will be handled automatically later
    (ExecutionResult::KeepRunning, 0)
}
/// Run the `history` builtin.
///
/// Prints all recorded history entries, numbered starting from 1.
pub fn run(state: &ShellState, stdout_redirect: Option<std::fs::File>) {
    let entries = &state.history;

    if let Some(mut f) = stdout_redirect {
        for (i, entry) in entries.iter().enumerate() {
            let _ = writeln!(f, "  {}  {}", i + 1, entry);
        }
    } else {
        for (i, entry) in entries.iter().enumerate() {
            println!("  {}  {}", i + 1, entry);
        }
    }
}
