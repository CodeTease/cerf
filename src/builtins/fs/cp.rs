use std::fs;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.cp",
    description: "Copy files and directories.",
    usage: "fs.cp source destination\n\nCopy SOURCE to DEST.",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.len() < 2 {
        eprintln!("cerf: fs.cp: missing file operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let src = expand_home(&args[0]);
    let dst = expand_home(&args[1]);

    if let Err(e) = fs::copy(&src, &dst) {
        eprintln!("cerf: fs.cp: cannot copy '{}' to '{}': {}", args[0], args[1], e);
        return (ExecutionResult::KeepRunning, 1);
    }

    (ExecutionResult::KeepRunning, 0)
}
