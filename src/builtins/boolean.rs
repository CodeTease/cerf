use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO_TRUE: CommandInfo = CommandInfo {
    name: "true",
    description: "Return a successful result.",
    usage: "true\n\nReturn a successful result.",
    run: true_runner,
};

pub fn true_runner(_args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    (ExecutionResult::KeepRunning, run_true())
}

pub const COMMAND_INFO_FALSE: CommandInfo = CommandInfo {
    name: "false",
    description: "Return an unsuccessful result.",
    usage: "false\n\nReturn an unsuccessful result.",
    run: false_runner,
};

pub fn false_runner(_args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    (ExecutionResult::KeepRunning, run_false())
}

/// The `true` built-in command.
pub fn run_true() -> i32 {
    0
}

/// The `false` built-in command.
/// Always returns failure (1).
pub fn run_false() -> i32 {
    1
}
