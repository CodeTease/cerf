use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;
use crate::engine::state::{ExecutionResult, ShellState};
use std::fs;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.rm",
    description: "Remove files or directories.",
    usage: "fs.rm [-r] [file ...]\n\nRemove (unlink) the FILE(s).",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        eprintln!("cerf: fs.rm: missing operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let mut recursive = false;
    let mut files = Vec::new();

    for arg in args {
        if arg == "-r" || arg == "-R" {
            recursive = true;
        } else {
            files.push(arg);
        }
    }

    if files.is_empty() {
        eprintln!("cerf: fs.rm: missing operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let mut exit_code = 0;
    for arg in files {
        let path = expand_home(arg);
        if !path.exists() {
            eprintln!(
                "cerf: fs.rm: cannot remove '{}': No such file or directory",
                arg
            );
            exit_code = 1;
            continue;
        }

        let res = if path.is_dir() {
            if recursive {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_dir(&path)
            }
        } else {
            fs::remove_file(&path)
        };

        if let Err(e) = res {
            eprintln!("cerf: fs.rm: cannot remove '{}': {}", arg, e);
            exit_code = 1;
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
}
