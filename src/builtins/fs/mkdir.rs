use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;
use crate::engine::state::{ExecutionResult, ShellState};
use std::fs;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.mkdir",
    description: "Create directories.",
    usage: "fs.mkdir [dir ...]\n\nCreate the DIRECTORY(ies), if they do not already exist.",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        eprintln!("cerf: fs.mkdir: missing operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let mut exit_code = 0;
    for arg in args {
        let path = expand_home(arg);
        if let Err(e) = fs::create_dir_all(&path) {
            eprintln!("cerf: fs.mkdir: cannot create directory '{}': {}", arg, e);
            exit_code = 1;
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
}
