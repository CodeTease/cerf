use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "io.echo",
    description: "Write arguments to the standard output.",
    usage: "io.echo [arg ...]",
    run,
};

pub fn run(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    println!("{}", args.join(" "));
    (ExecutionResult::KeepRunning, 0)
}
