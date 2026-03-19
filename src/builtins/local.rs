use crate::builtins::declare;
use crate::builtins::registry::CommandInfo;
use crate::engine::state::{ExecutionResult, ShellState};

pub const COMMAND_INFO_LOCAL: CommandInfo = CommandInfo {
    name: "env.local",
    description: "Create local variables.",
    usage: "env.local [name[=value] ...]\n\nCreate a local variable.",
    run: local_runner,
};

pub fn local_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    let mode = declare::run(args, state, true);
    (ExecutionResult::KeepRunning, mode)
}
