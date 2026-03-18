use std::fs;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.touch",
    description: "Update the access and modification times of each FILE to the current time.",
    usage: "fs.touch [file ...]\n\nUpdate the access and modification times of each FILE to the current time. A FILE argument that does not exist is created empty.",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        eprintln!("cerf: fs.touch: missing file operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let mut exit_code = 0;
    for arg in args {
        let path = expand_home(arg);
        let res = if path.exists() {
            // Update timestamp - for now just open and close it
            fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .map(|_| ())
        } else {
            fs::File::create(&path).map(|_| ())
        };

        if let Err(e) = res {
            eprintln!("cerf: fs.touch: cannot touch '{}': {}", arg, e);
            exit_code = 1;
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
}
