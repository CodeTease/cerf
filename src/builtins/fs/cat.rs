use std::fs;
use std::io::{self, Write};
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.cat",
    description: "Concatenate and print files.",
    usage: "fs.cat [file ...]\n\nConcatenate FILE(s) to standard output.",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        return (ExecutionResult::KeepRunning, 0);
    }

    let mut exit_code = 0;
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for arg in args {
        let path = expand_home(arg);
        match fs::File::open(&path) {
            Ok(mut file) => {
                if let Err(e) = io::copy(&mut file, &mut handle) {
                    eprintln!("cerf: fs.cat: {}: {}", arg, e);
                    exit_code = 1;
                }
            }
            Err(e) => {
                eprintln!("cerf: fs.cat: {}: {}", arg, e);
                exit_code = 1;
            }
        }
    }
    let _ = handle.flush();
    (ExecutionResult::KeepRunning, exit_code)
}
