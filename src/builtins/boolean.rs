use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO_TRUE: CommandInfo = CommandInfo {
    name: "test.true",
    description: "Return a successful result.",
    usage: "test.true\n\nReturn a successful result.",
    run: true_runner,
};

pub fn true_runner(_args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    (ExecutionResult::KeepRunning, run_true())
}

pub const COMMAND_INFO_FALSE: CommandInfo = CommandInfo {
    name: "test.false",
    description: "Return an unsuccessful result.",
    usage: "test.false\n\nReturn an unsuccessful result.",
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
