use std::path::PathBuf;
use std::process::Command;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use crate::parser::{CommandEntry, Connector, ParsedCommand};
use crate::builtins;
#[cfg(unix)]
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

// ── Single command ─────────────────────────────────────────────────────────

/// Execute one command. Returns `(ExecutionResult, exit_code)`.
///
/// `exit_code` is `0` for success and non-zero for failure.  Builtins that
/// do not have a meaningful failure mode always return `0`.
pub fn execute(cmd: ParsedCommand, state: &mut ShellState) -> (ExecutionResult, i32) {
    match cmd.name.as_str() {
        "cd" => {
            let code = match builtins::cd::run(&cmd.args, state) {
                Ok(()) => 0,
                Err(e) => { eprintln!("cerf: cd: {}", e); 1 }
            };
            (ExecutionResult::KeepRunning, code)
        },
        "pwd" => {
            builtins::cd::pwd();
            (ExecutionResult::KeepRunning, 0)
        },
        "exit" => {
            builtins::system::exit();
            (ExecutionResult::Exit, 0)
        },
        "clear" => {
            builtins::system::clear();
            (ExecutionResult::KeepRunning, 0)
        },
        "echo" => {
            builtins::echo::run(&cmd.args);
            (ExecutionResult::KeepRunning, 0)
        },
        "type" => {
            builtins::type_cmd::run(&cmd.args);
            (ExecutionResult::KeepRunning, 0)
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

            let code = match result {
                Ok(mut child) => {
                    child.wait()
                        .map(|s| s.code().unwrap_or(1))
                        .unwrap_or(1)
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        eprintln!("cerf: command not found: {}", cmd.name);
                    } else {
                        eprintln!("cerf: error executing '{}': {}", cmd.name, e);
                    }
                    127
                }
            };
            (ExecutionResult::KeepRunning, code)
        }
    }
}

// ── Command list (&&, ||, ;) ───────────────────────────────────────────────

/// Execute a list of commands chained by `&&`, `||`, and `;`.
///
/// Semantics follow POSIX sh:
/// - **`;`**  — always run the next command regardless of the previous exit code.
/// - **`&&`** — run the next command only if the previous command returned exit
///              code `0` (success).
/// - **`||`** — run the next command only if the previous command returned a
///              non-zero exit code (failure).
pub fn execute_list(entries: Vec<CommandEntry>, state: &mut ShellState) -> ExecutionResult {
    let mut last_code: i32 = 0;

    for entry in entries {
        // Decide whether to skip this command based on the connector and the
        // last exit code.
        let skip = match entry.connector {
            None                    => false,              // first command: always run
            Some(Connector::Semi)   => false,              // ;  → always run
            Some(Connector::And)    => last_code != 0,     // && → skip on failure
            Some(Connector::Or)     => last_code == 0,     // || → skip on success
        };

        if skip {
            continue;
        }

        let (result, code) = execute(entry.command, state);
        last_code = code;

        if let ExecutionResult::Exit = result {
            return ExecutionResult::Exit;
        }
    }

    ExecutionResult::KeepRunning
}
