use std::fs;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.ls",
    description: "List directory contents.",
    usage: "fs.ls [path ...]\n\nList information about the FILEs (the current directory by default).",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    let targets = if args.is_empty() {
        vec![".".to_string()]
    } else {
        args.to_vec()
    };

    let mut exit_code = 0;
    let multiple = targets.len() > 1;

    for (i, target) in targets.iter().enumerate() {
        let path = expand_home(target);
        if !path.exists() {
            eprintln!("cerf: fs.ls: cannot access '{}': No such file or directory", target);
            exit_code = 1;
            continue;
        }

        if path.is_dir() {
            if multiple {
                if i > 0 { println!(); }
                println!("{}:", target);
            }
            match fs::read_dir(&path) {
                Ok(entries) => {
                    let mut names: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| {
                            let name = e.file_name().to_string_lossy().into_owned();
                            if e.path().is_dir() {
                                format!("{}/", name)
                            } else {
                                name
                            }
                        })
                        .collect();
                    names.sort();
                    println!("{}", names.join("  "));
                }
                Err(e) => {
                    eprintln!("cerf: fs.ls: cannot open directory '{}': {}", target, e);
                    exit_code = 1;
                }
            }
        } else {
            println!("{}", target);
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
}
