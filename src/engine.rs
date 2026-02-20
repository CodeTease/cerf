use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use crate::parser::{CommandEntry, Connector, ParsedCommand, Pipeline, Redirect, RedirectKind};
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

// ── Redirect helpers ──────────────────────────────────────────────────────

/// Open a file for an output redirect (stdout).
fn open_stdout_redirect(redirect: &Redirect) -> Result<File, String> {
    match redirect.kind {
        RedirectKind::StdoutOverwrite => {
            File::create(&redirect.file)
                .map_err(|e| format!("cerf: {}: {}", redirect.file, e))
        }
        RedirectKind::StdoutAppend => {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&redirect.file)
                .map_err(|e| format!("cerf: {}: {}", redirect.file, e))
        }
        _ => Err("not a stdout redirect".to_string()),
    }
}

/// Open a file for an input redirect (stdin).
fn open_stdin_redirect(redirect: &Redirect) -> Result<File, String> {
    File::open(&redirect.file)
        .map_err(|e| format!("cerf: {}: {}", redirect.file, e))
}

/// Find the first stdin and last stdout redirect from a list.
fn resolve_redirects(redirects: &[Redirect]) -> (Option<&Redirect>, Option<&Redirect>) {
    let stdin_redir = redirects.iter().rfind(|r| r.kind == RedirectKind::StdinFrom);
    let stdout_redir = redirects.iter().rfind(|r| {
        r.kind == RedirectKind::StdoutOverwrite || r.kind == RedirectKind::StdoutAppend
    });
    (stdin_redir, stdout_redir)
}

// ── Builtin check ─────────────────────────────────────────────────────────



// ── Single command (no pipe) ──────────────────────────────────────────────

/// Execute one simple command with optional redirections.
/// Returns `(ExecutionResult, exit_code)`.
fn execute_simple(cmd: &ParsedCommand, state: &mut ShellState) -> (ExecutionResult, i32) {
    let (stdin_redir, stdout_redir) = resolve_redirects(&cmd.redirects);

    match cmd.name.as_str() {
        "cd" => {
            let code = match builtins::cd::run(&cmd.args, state) {
                Ok(()) => 0,
                Err(e) => { eprintln!("cerf: cd: {}", e); 1 }
            };
            (ExecutionResult::KeepRunning, code)
        },
        "pwd" => {
            if let Some(redir) = stdout_redir {
                match open_stdout_redirect(redir) {
                    Ok(mut f) => {
                        let cwd = std::env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."));
                        let _ = writeln!(f, "{}", cwd.display());
                        (ExecutionResult::KeepRunning, 0)
                    }
                    Err(e) => { eprintln!("{}", e); (ExecutionResult::KeepRunning, 1) }
                }
            } else {
                builtins::cd::pwd();
                (ExecutionResult::KeepRunning, 0)
            }
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
            let output = cmd.args.join(" ");
            if let Some(redir) = stdout_redir {
                match open_stdout_redirect(redir) {
                    Ok(mut f) => {
                        let _ = writeln!(f, "{}", output);
                        (ExecutionResult::KeepRunning, 0)
                    }
                    Err(e) => { eprintln!("{}", e); (ExecutionResult::KeepRunning, 1) }
                }
            } else {
                println!("{}", output);
                (ExecutionResult::KeepRunning, 0)
            }
        },
        "type" => {
            if let Some(redir) = stdout_redir {
                match open_stdout_redirect(redir) {
                    Ok(mut f) => {
                        for arg in &cmd.args {
                            let output = builtins::type_cmd::type_of(arg);
                            let _ = writeln!(f, "{}", output);
                        }
                        (ExecutionResult::KeepRunning, 0)
                    }
                    Err(e) => { eprintln!("{}", e); (ExecutionResult::KeepRunning, 1) }
                }
            } else {
                builtins::type_cmd::run(&cmd.args);
                (ExecutionResult::KeepRunning, 0)
            }
        },
        _ => {
            let mut command = Command::new(&cmd.name);
            command.args(&cmd.args);

            // Apply stdin redirect
            if let Some(redir) = stdin_redir {
                match open_stdin_redirect(redir) {
                    Ok(f) => { command.stdin(Stdio::from(f)); }
                    Err(e) => {
                        eprintln!("{}", e);
                        return (ExecutionResult::KeepRunning, 1);
                    }
                }
            }

            // Apply stdout redirect
            if let Some(redir) = stdout_redir {
                match open_stdout_redirect(redir) {
                    Ok(f) => { command.stdout(Stdio::from(f)); }
                    Err(e) => {
                        eprintln!("{}", e);
                        return (ExecutionResult::KeepRunning, 1);
                    }
                }
            }

            #[cfg(unix)]
            let result = unsafe {
                command
                    .pre_exec(|| {
                        signals::restore_default();
                        Ok(())
                    })
                    .spawn()
            };

            #[cfg(windows)]
            let result = command.spawn();

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

// ── Pipeline execution ────────────────────────────────────────────────────

/// Execute a full pipeline (one or more commands connected by `|`).
/// Returns `(ExecutionResult, exit_code)`.
pub fn execute(pipeline: &Pipeline, state: &mut ShellState) -> (ExecutionResult, i32) {
    let cmds = &pipeline.commands;

    // Single-command pipeline — just run the command directly (supports builtins).
    if cmds.len() == 1 {
        return execute_simple(&cmds[0], state);
    }

    // Multi-command pipeline: fork external processes connected by pipes.
    // Builtins in a multi-command pipeline are run as external commands
    // (same behaviour as bash).
    let last_idx = cmds.len() - 1;
    let mut children: Vec<std::process::Child> = Vec::with_capacity(cmds.len());
    let mut prev_stdout: Option<std::process::ChildStdout> = None;

    for (i, cmd) in cmds.iter().enumerate() {
        // If a builtin appears in a multi-command pipeline, check for exit
        if cmd.name == "exit" {
            // Kill any children we already spawned
            for mut child in children {
                let _ = child.kill();
            }
            builtins::system::exit();
            return (ExecutionResult::Exit, 0);
        }

        let mut command = Command::new(&cmd.name);
        command.args(&cmd.args);

        // Stdin: first command may have < redirect, others get previous pipe
        if i == 0 {
            let (stdin_redir, _) = resolve_redirects(&cmd.redirects);
            if let Some(redir) = stdin_redir {
                match open_stdin_redirect(redir) {
                    Ok(f) => { command.stdin(Stdio::from(f)); }
                    Err(e) => {
                        eprintln!("{}", e);
                        // Kill already started children
                        for mut child in children {
                            let _ = child.kill();
                        }
                        return (ExecutionResult::KeepRunning, 1);
                    }
                }
            }
        } else if let Some(stdout) = prev_stdout.take() {
            command.stdin(Stdio::from(stdout));
        }

        // Stdout: last command may have > or >> redirect, others pipe
        if i == last_idx {
            let (_, stdout_redir) = resolve_redirects(&cmd.redirects);
            if let Some(redir) = stdout_redir {
                match open_stdout_redirect(redir) {
                    Ok(f) => { command.stdout(Stdio::from(f)); }
                    Err(e) => {
                        eprintln!("{}", e);
                        for mut child in children {
                            let _ = child.kill();
                        }
                        return (ExecutionResult::KeepRunning, 1);
                    }
                }
            }
        } else {
            command.stdout(Stdio::piped());
        }

        #[cfg(unix)]
        let result = unsafe {
            command
                .pre_exec(|| {
                    signals::restore_default();
                    Ok(())
                })
                .spawn()
        };

        #[cfg(windows)]
        let result = command.spawn();

        match result {
            Ok(mut child) => {
                if i != last_idx {
                    prev_stdout = child.stdout.take();
                }
                children.push(child);
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    eprintln!("cerf: command not found: {}", cmd.name);
                } else {
                    eprintln!("cerf: error executing '{}': {}", cmd.name, e);
                }
                // Kill already started children
                for mut child in children {
                    let _ = child.kill();
                }
                return (ExecutionResult::KeepRunning, 127);
            }
        }
    }

    // Wait for all children; use the last command's exit code.
    let mut last_code = 0;
    for (i, mut child) in children.into_iter().enumerate() {
        let code = child.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1);
        if i == last_idx {
            last_code = code;
        }
    }

    (ExecutionResult::KeepRunning, last_code)
}

// ── Command list (&&, ||, ;) ───────────────────────────────────────────────

/// Execute a list of pipelines chained by `&&`, `||`, and `;`.
///
/// Semantics follow POSIX sh:
/// - **`;`**  — always run the next pipeline regardless of the previous exit code.
/// - **`&&`** — run the next pipeline only if the previous returned exit
///              code `0` (success).
/// - **`||`** — run the next pipeline only if the previous returned a
///              non-zero exit code (failure).
pub fn execute_list(entries: Vec<CommandEntry>, state: &mut ShellState) -> ExecutionResult {
    let mut last_code: i32 = 0;

    for entry in entries {
        // Decide whether to skip this pipeline based on the connector and the
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

        let (result, code) = execute(&entry.pipeline, state);
        last_code = code;

        if let ExecutionResult::Exit = result {
            return ExecutionResult::Exit;
        }
    }

    ExecutionResult::KeepRunning
}
