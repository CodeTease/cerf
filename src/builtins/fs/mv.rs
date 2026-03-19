use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;
use crate::engine::state::{ExecutionResult, ShellState};
use std::fs;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.mv",
    description: "Move (rename) files.",
    usage: "fs.mv source destination\n\nRename SOURCE to DEST.",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.len() < 2 {
        eprintln!("cerf: fs.mv: missing file operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let src = expand_home(&args[0]);
    let dst = expand_home(&args[1]);

    if let Err(e) = fs::rename(&src, &dst) {
        eprintln!(
            "cerf: fs.mv: cannot move '{}' to '{}': {}",
            args[0], args[1], e
        );
        return (ExecutionResult::KeepRunning, 1);
    }

    (ExecutionResult::KeepRunning, 0)
}
