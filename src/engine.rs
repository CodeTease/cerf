use std::path::PathBuf;
use std::process::Command;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use crate::parser::ParsedCommand;
use crate::builtins;
use crate::signals;

pub struct ShellState {
    pub previous_dir: Option<PathBuf>,
}

impl ShellState {
    pub fn new() -> Self {
        ShellState { previous_dir: None }
    }
}

pub enum ExecutionResult {
    KeepRunning,
    Exit,
}

pub fn execute(cmd: ParsedCommand, state: &mut ShellState) -> ExecutionResult {
    match cmd.name.as_str() {
        "cd" => {
            if let Err(e) = builtins::cd::run(&cmd.args, state) {
                eprintln!("cerf: cd: {}", e);
            }
            ExecutionResult::KeepRunning
        },
        "pwd" => {
             builtins::cd::pwd();
             ExecutionResult::KeepRunning
        },
        "exit" => {
            builtins::system::exit();
            ExecutionResult::Exit
        },
        "clear" => {
            builtins::system::clear();
            ExecutionResult::KeepRunning
        },
        _ => {
            #[cfg(unix)]
            let result = unsafe {
                Command::new(&cmd.name)
                    .args(&cmd.args)
                    .pre_exec(|| {
                        signals::restore_default();
                        Ok(())
                    })
                    .spawn()
            };

            #[cfg(windows)]
            let result = Command::new(&cmd.name)
                .args(&cmd.args)
                .spawn();

            match result {
                Ok(mut child) => {
                    let _ = child.wait();
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        eprintln!("cerf: command not found: {}", cmd.name);
                    } else {
                        eprintln!("cerf: error executing '{}': {}", cmd.name, e);
                    }
                }
            }
            ExecutionResult::KeepRunning
        }
    }
}
