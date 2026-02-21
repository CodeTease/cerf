use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::parser::{CommandEntry, Connector, ParsedCommand, Pipeline};
use crate::builtins;
#[cfg(unix)]
use crate::signals;

use super::state::{ShellState, ExecutionResult};
use super::redirect::{open_stdout_redirect, open_stdin_redirect, resolve_redirects};
use super::alias::expand_alias;
use super::path::{expand_home, find_executable};
use super::glob::expand_globs;

// ── Single command (no pipe) ──────────────────────────────────────────────

/// Execute one simple command with optional redirections.
/// Returns `(ExecutionResult, exit_code)`.
fn execute_simple(pipeline: &Pipeline, state: &mut ShellState) -> (ExecutionResult, i32) {
    let cmd = &pipeline.commands[0];
    let (stdin_redir, stdout_redir) = resolve_redirects(&cmd.redirects);

    if cmd.name.is_none() {
        // Just assignments
        for (key, val) in &cmd.assignments {
            state.variables.insert(key.clone(), val.clone());
            // If already in env, update it there too
            if std::env::var(key).is_ok() {
               unsafe { std::env::set_var(key, val); }
            }
        }
        // Handle residuals like redirects (e.g., VAR=val > file)
        if let Some(redir) = stdin_redir {
            if let Err(e) = open_stdin_redirect(redir) {
                eprintln!("{}", e);
                return (ExecutionResult::KeepRunning, 1);
            }
        }
        if let Some(redir) = stdout_redir {
            if let Err(e) = open_stdout_redirect(redir) {
                eprintln!("{}", e);
                return (ExecutionResult::KeepRunning, 1);
            }
        }
        return (ExecutionResult::KeepRunning, 0);
    }

    let name = cmd.name.as_ref().unwrap();

    // Expand globs on the argument list.
    let args = expand_globs(&cmd.args);

    match name.as_str() {
        "alias" => {
            builtins::alias::run(&args, &mut state.aliases);
            (ExecutionResult::KeepRunning, 0)
        },
        "unalias" => {
            builtins::unalias::run(&args, &mut state.aliases);
            (ExecutionResult::KeepRunning, 0)
        },
        "export" => {
            builtins::export::run(&args, &mut state.variables);
            (ExecutionResult::KeepRunning, 0)
        },
        "unset" => {
            builtins::unset::run(&args, &mut state.variables);
            (ExecutionResult::KeepRunning, 0)
        },
        "set" => {
            let code = builtins::set::run(&args, state);
            (ExecutionResult::KeepRunning, code)
        },
        "jobs" => {
            let code = builtins::jobs::run(state);
            (ExecutionResult::KeepRunning, code)
        },
        "fg" => {
            let code = builtins::fg::run(&args, state);
            (ExecutionResult::KeepRunning, code)
        },
        "bg" => {
            let code = builtins::bg::run(&args, state);
            (ExecutionResult::KeepRunning, code)
        },
        "wait" => {
            let code = builtins::wait::run(&args, state);
            (ExecutionResult::KeepRunning, code)
        },
        "kill" => {
            let code = builtins::kill_cmd::run(&args, state);
            (ExecutionResult::KeepRunning, code)
        },
        "cd" => {
            let code = match builtins::cd::run(&args, state) {
                Ok(()) => 0,
                Err(e) => { eprintln!("cerf: cd: {}", e); 1 }
            };
            (ExecutionResult::KeepRunning, code)
        },
        "pushd" => {
            let redir_file = stdout_redir.and_then(|r| open_stdout_redirect(r).ok());
            let code = match builtins::dirs::pushd(&args, state, redir_file) {
                Ok(()) => 0,
                Err(e) => { eprintln!("cerf: {}", e); 1 }
            };
            (ExecutionResult::KeepRunning, code)
        },
        "popd" => {
            let redir_file = stdout_redir.and_then(|r| open_stdout_redirect(r).ok());
            let code = match builtins::dirs::popd(&args, state, redir_file) {
                Ok(()) => 0,
                Err(e) => { eprintln!("cerf: {}", e); 1 }
            };
            (ExecutionResult::KeepRunning, code)
        },
        "dirs" => {
            let redir_file = stdout_redir.and_then(|r| open_stdout_redirect(r).ok());
            builtins::dirs::run_dirs(state, redir_file);
            (ExecutionResult::KeepRunning, 0)
        },
        "pwd" => {
            if let Some(redir) = stdout_redir {
                match open_stdout_redirect(redir) {
                    Ok(mut f) => {
                        let cwd = std::env::current_dir()
                            .unwrap_or_else(|_ | PathBuf::from("."));
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
        "exec" => {
            match builtins::system::exec(&args) {
                Ok(code) => {
                    // exec succeeded (Windows emulation) — exit the shell
                    // with the child's exit code.
                    (ExecutionResult::Exit, code)
                }
                Err(msg) => {
                    eprintln!("{}", msg);
                    (ExecutionResult::KeepRunning, 1)
                }
            }
        },
        "true" => {
            (ExecutionResult::KeepRunning, builtins::boolean::run_true())
        },
        "false" => {
            (ExecutionResult::KeepRunning, builtins::boolean::run_false())
        },
        "test" => {
            let code = builtins::test_cmd::run(&args, false);
            (ExecutionResult::KeepRunning, code)
        },
        "[" => {
            let code = builtins::test_cmd::run(&args, true);
            (ExecutionResult::KeepRunning, code)
        },
        "echo" => {
            let output = args.join(" ");
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
        "read" => {
            // Apply stdin redirect if present, otherwise use standard stdin
            if let Some(redir) = stdin_redir {
                match open_stdin_redirect(redir) {
                    Ok(_f) => {
                        // We would need to pass this to read::run, but for now we'll rely on the standard stdin
                        // This might not work perfectly with cerf architecture, but we do our best.
                        // For a proper implementation, read::run should take an Optional Reader.
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        return (ExecutionResult::KeepRunning, 1);
                    }
                }
            }
            let code = match builtins::read::run(&args, state) {
                Ok(()) => 0,
                Err(e) => {
                    if !e.is_empty() {
                        eprintln!("cerf: read: {}", e);
                    }
                    1
                }
            };
            (ExecutionResult::KeepRunning, code)
        },
        "source" | "." => {
            builtins::source::run(&args, state)
        },
        "history" => {
            let redir_file = stdout_redir.and_then(|r| open_stdout_redirect(r).ok());
            builtins::history::run(state, redir_file);
            (ExecutionResult::KeepRunning, 0)
        },
        "type" => {
            if let Some(redir) = stdout_redir {
                match open_stdout_redirect(redir) {
                    Ok(mut f) => {
                        for arg in &args {
                            let output = builtins::type_cmd::type_of(arg, &state.aliases);
                            let _ = writeln!(f, "{}", output);
                        }
                        (ExecutionResult::KeepRunning, 0)
                    }
                    Err(e) => { eprintln!("{}", e); (ExecutionResult::KeepRunning, 1) }
                }
            } else {
                builtins::type_cmd::run(&args, &state.aliases);
                (ExecutionResult::KeepRunning, 0)
            }
        },
        _ => {
            let resolved = find_executable(name).unwrap_or_else(|| expand_home(name));
            
            #[cfg(windows)]
            let mut command = {
                let is_batch = resolved.extension().map_or(false, |e| {
                    let e = e.to_string_lossy().to_lowercase();
                    e == "cmd" || e == "bat"
                });
                if is_batch {
                    let mut c = Command::new("cmd");
                    c.arg("/c").arg(&resolved);
                    c
                } else {
                    Command::new(&resolved)
                }
            };
            
            #[cfg(unix)]
            let mut command = Command::new(&resolved);

            command.args(&args);
            command.envs(cmd.assignments.iter().map(|(k, v)| (k, v)));

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
                        let pid = nix::unistd::getpid();
                        let _ = nix::unistd::setpgid(pid, pid);
                        signals::restore_default();
                        Ok(())
                    })
                    .spawn()
            };

            #[cfg(windows)]
            let result = command.spawn();

            let code = match result {
                Ok(mut child) => {
                    let pid = child.id();
                    
                    #[cfg(unix)]
                    if let Some(shell_pgid) = state.shell_pgid {
                        let _ = nix::unistd::setpgid(
                            nix::unistd::Pid::from_raw(pid as i32), 
                            nix::unistd::Pid::from_raw(pid as i32)
                        );
                    }

                    let job = crate::engine::state::Job {
                        id: state.next_job_id,
                        pgid: pid,
                        command: crate::engine::job_control::format_command(pipeline),
                        processes: vec![crate::engine::state::ProcessInfo {
                            pid,
                            name: name.to_string(),
                            state: crate::engine::state::JobState::Running,
                        }],
                        reported_done: false,
                    };
                    let job_id = state.next_job_id;
                    state.jobs.insert(job_id, job);
                    state.next_job_id += 1;
                    
                    if pipeline.background {
                        println!("[{}] {}", job_id, pid);
                        0
                    } else {
                        crate::engine::job_control::wait_for_job(job_id, state, true)
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        eprintln!("cerf: command not found: {}", name);
                    } else {
                        eprintln!("cerf: error executing '{}': {}", name, e);
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
    let mut pipeline = pipeline.clone();

    // Expand aliases on every command's name (only the first command of a
    // pipeline gets alias-expanded, same as bash behaviour for safety).
    for cmd in &mut pipeline.commands {
        expand_alias(cmd, &state.aliases);
    }

    let cmds = &pipeline.commands;

    // Single-command pipeline — just run the command directly (supports builtins).
    if cmds.len() == 1 {
        let (res, code) = execute_simple(&pipeline, state);
        let final_code = if pipeline.negated {
            if code == 0 { 1 } else { 0 }
        } else {
            code
        };
        return (res, final_code);
    }

    // Multi-command pipeline: fork external processes connected by pipes.
    // Builtins in a multi-command pipeline are run as external commands
    // (same behaviour as bash).
    let last_idx = cmds.len() - 1;
    let mut children: Vec<std::process::Child> = Vec::with_capacity(cmds.len());
    let mut prev_stdout: Option<std::process::ChildStdout> = None;

    let mut first_pgid = 0;
    let mut processes = Vec::new();

    for (i, cmd) in cmds.iter().enumerate() {
        let name = match cmd.name.as_ref() {
            Some(n) => n,
            None => {
                continue;
            }
        };

        // If a builtin appears in a multi-command pipeline, check for exit
        if name == "exit" {
            // Kill any children we already spawned
            for mut child in children {
                let _ = child.kill();
            }
            builtins::system::exit();
            return (ExecutionResult::Exit, 0);
        }

        let resolved = find_executable(name).unwrap_or_else(|| expand_home(name));

        // Expand globs on the argument list.
        let args = expand_globs(&cmd.args);

        #[cfg(windows)]
        let mut command = {
            let is_batch = resolved.extension().map_or(false, |e| {
                let e = e.to_string_lossy().to_lowercase();
                e == "cmd" || e == "bat"
            });
            if is_batch {
                let mut c = Command::new("cmd");
                c.arg("/c").arg(&resolved);
                c
            } else {
                Command::new(&resolved)
            }
        };

        #[cfg(unix)]
        let mut command = Command::new(&resolved);

        command.args(&args);
        command.envs(cmd.assignments.iter().map(|(k, v)| (k, v)));

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
        let target_pgid = first_pgid;
        
        #[cfg(unix)]
        let result = unsafe {
            command
                .pre_exec(move || {
                    let pid = nix::unistd::getpid();
                    let pgid = if target_pgid == 0 { pid } else { nix::unistd::Pid::from_raw(target_pgid as i32) };
                    let _ = nix::unistd::setpgid(pid, pgid);
                    signals::restore_default();
                    Ok(())
                })
                .spawn()
        };

        #[cfg(windows)]
        let result = command.spawn();

        match result {
            Ok(mut child) => {
                let pid = child.id();
                if i == 0 {
                    first_pgid = pid;
                }
                
                #[cfg(unix)]
                if let Some(shell_pgid) = state.shell_pgid {
                    let _ = nix::unistd::setpgid(
                        nix::unistd::Pid::from_raw(pid as i32), 
                        nix::unistd::Pid::from_raw(first_pgid as i32)
                    );
                }

                processes.push(crate::engine::state::ProcessInfo {
                    pid,
                    name: name.to_string(),
                    state: crate::engine::state::JobState::Running,
                });
                
                if i != last_idx {
                    prev_stdout = child.stdout.take();
                }
                children.push(child);
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    eprintln!("cerf: command not found: {}", name);
                } else {
                    eprintln!("cerf: error executing '{}': {}", name, e);
                }
                // Kill already started children
                for mut child in children {
                    let _ = child.kill();
                }
                return (ExecutionResult::KeepRunning, 127);
            }
        }
    }

    let job = crate::engine::state::Job {
        id: state.next_job_id,
        pgid: first_pgid,
        command: crate::engine::job_control::format_command(&pipeline),
        processes,
        reported_done: false,
    };
    let job_id = state.next_job_id;
    state.jobs.insert(job_id, job);
    state.next_job_id += 1;

    let last_code = if pipeline.background {
        println!("[{}] {}", job_id, first_pgid);
        0
    } else {
        crate::engine::job_control::wait_for_job(job_id, state, true)
    };

    let final_code = if pipeline.negated {
        if last_code == 0 { 1 } else { 0 }
    } else {
        last_code
    };

    (ExecutionResult::KeepRunning, final_code)
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
            Some(Connector::Amp)    => false,              // &  → always run
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
