use std::collections::HashMap;
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
    /// All currently-defined aliases. Maps alias name → replacement string.
    pub aliases: HashMap<String, String>,
    /// All currently-defined shell variables.
    pub variables: HashMap<String, String>,
}

impl ShellState {
    pub fn new() -> Self {
        let variables = init_env_vars();

        ShellState {
            previous_dir: None,
            aliases: HashMap::new(),
            variables,
        }
    }
}

/// Initialize shell variables from the OS environment and set defaults for missing ones.
fn init_env_vars() -> HashMap<String, String> {
    let mut vars: HashMap<String, String> = std::env::vars().collect();

    // 1. Ensure HOME is set
    if !vars.contains_key("HOME") {
        #[cfg(windows)]
        {
            if let Ok(profile) = std::env::var("USERPROFILE") {
                vars.insert("HOME".to_string(), profile);
            }
        }
        #[cfg(not(windows))]
        {
            if let Ok(home) = std::env::var("HOME") {
                vars.insert("HOME".to_string(), home);
            } else if let Some(home_path) = dirs::home_dir() {
                vars.insert("HOME".to_string(), home_path.to_string_lossy().to_string());
            }
        }
    }

    // 2. Ensure PATH is set
    if !vars.contains_key("PATH") {
        #[cfg(windows)]
        vars.insert("PATH".to_string(), "C:\\Windows\\system32;C:\\Windows".to_string());
        #[cfg(not(windows))]
        vars.insert("PATH".to_string(), "/usr/local/bin:/usr/bin:/bin".to_string());
    }

    // 3. Ensure EDITOR is set
    if !vars.contains_key("EDITOR") {
        #[cfg(windows)]
        vars.insert("EDITOR".to_string(), "notepad".to_string());
        #[cfg(not(windows))]
        vars.insert("EDITOR".to_string(), "vi".to_string());
    }

    // Sync environment variables that we just added defaults for
    for (key, val) in &vars {
        if std::env::var(key).is_err() {
            unsafe { std::env::set_var(key, val); }
        }
    }

    vars
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

// ── Alias expansion ───────────────────────────────────────────────────────

/// Expand aliases on a `ParsedCommand` in-place (bash-style, one level).
///
/// If `cmd.name` matches an alias whose value is a single word, the name is
/// replaced and the replacement's trailing args are prepended to `cmd.args`.
/// If the alias value is a multi-word string, it is re-parsed: the first
/// token becomes the new command name and the rest are prepended to the
/// existing args.
///
/// Returns `true` when an expansion happened.
fn expand_alias(cmd: &mut ParsedCommand, aliases: &HashMap<String, String>) -> bool {
    let name = match cmd.name.as_ref() {
        Some(n) => n,
        None => return false,
    };
    if let Some(value) = aliases.get(name) {
        let value = value.clone();
        // Tokenise the alias value with a simple whitespace split that
        // respects single-quoted segments (good enough for shell aliases).
        let tokens = shell_split(&value);
        if tokens.is_empty() {
            return false;
        }
        // The first token is the new command name.
        cmd.name = Some(tokens[0].clone());
        // Any remaining alias tokens are prepended to the original args.
        let mut new_args = tokens[1..].to_vec();
        new_args.extend(cmd.args.drain(..));
        cmd.args = new_args;
        return true;
    }
    false
}

/// Very small shell-word splitter that honours `'…'` quoting.
/// Used only for parsing alias values.
fn shell_split(s: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_single => in_single = true,
            '\'' if in_single  => in_single = false,
            ' ' | '\t' if !in_single => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            other => current.push(other),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

// ── Path resolution helper ────────────────────────────────────────────────

pub fn find_executable(cmd: &str) -> Option<PathBuf> {
    let cmd_path = PathBuf::from(cmd);

    // 1. If it has a separator, check it directly
    if cmd.contains('/') || (cfg!(windows) && cmd.contains('\\')) {
        return check_path(cmd_path);
    }

    // 2. Search PATH
    if let Ok(paths) = std::env::var("PATH") {
        for path in std::env::split_paths(&paths) {
            if let Some(found) = check_path(path.join(cmd)) {
                return Some(found);
            }
        }
    }

    // 3. Search current directory on Windows (traditional behavior)
    #[cfg(windows)]
    {
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(found) = check_path(cwd.join(cmd)) {
                return Some(found);
            }
        }
    }

    None
}

fn check_path(p: PathBuf) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        if p.is_file() {
            return Some(p);
        }
    }

    #[cfg(windows)]
    {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
        let pathext_list: Vec<_> = pathext
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_uppercase())
            .collect();

        // 1. If it already has an extension that's in PATHEXT, try it as is first.
        if let Some(ext) = p.extension() {
            let ext_dot = format!(".{}", ext.to_string_lossy().to_uppercase());
            if pathext_list.iter().any(|e| e == &ext_dot) {
                if p.is_file() {
                    return Some(p);
                }
            }
        }

        // 2. Try appending extensions from PATHEXT.
        for ext in &pathext_list {
            let mut os_str = p.clone().into_os_string();
            if !ext.starts_with('.') {
                os_str.push(".");
            }
            os_str.push(ext);
            let p_ext = PathBuf::from(os_str);

            if p_ext.is_file() {
                return Some(p_ext);
            }
        }

        // 3. Fallback: try as-is only if we didn't already try it in step 1.
        // This handles extensionless files (though they might fail to exec).
        if p.is_file() {
            return Some(p);
        }
    }

    None
}

// ── Single command (no pipe) ──────────────────────────────────────────────

/// Execute one simple command with optional redirections.
/// Returns `(ExecutionResult, exit_code)`.
fn execute_simple(cmd: &ParsedCommand, state: &mut ShellState) -> (ExecutionResult, i32) {
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

    match name.as_str() {
        "alias" => {
            builtins::alias::run(&cmd.args, &mut state.aliases);
            (ExecutionResult::KeepRunning, 0)
        },
        "unalias" => {
            builtins::unalias::run(&cmd.args, &mut state.aliases);
            (ExecutionResult::KeepRunning, 0)
        },
        "export" => {
            builtins::export::run(&cmd.args, &mut state.variables);
            (ExecutionResult::KeepRunning, 0)
        },
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
                            let output = builtins::type_cmd::type_of(arg, &state.aliases);
                            let _ = writeln!(f, "{}", output);
                        }
                        (ExecutionResult::KeepRunning, 0)
                    }
                    Err(e) => { eprintln!("{}", e); (ExecutionResult::KeepRunning, 1) }
                }
            } else {
                builtins::type_cmd::run(&cmd.args, &state.aliases);
                (ExecutionResult::KeepRunning, 0)
            }
        },
        _ => {
            let resolved = find_executable(name).unwrap_or_else(|| PathBuf::from(name));
            
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

            command.args(&cmd.args);
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
        let (res, code) = execute_simple(&cmds[0], state);
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

    for (i, cmd) in cmds.iter().enumerate() {
        let name = match cmd.name.as_ref() {
            Some(n) => n,
            None => {
                // Command with just assignments in a multi-command pipeline.
                // In POSIX, each part of a pipeline is run in a subshell.
                // For simplicity, we'll just skip this command after setting vars
                // if we were in a forked process, but here we are in the main process
                // forking children.
                // We should probably spawn a dummy process or just skip it.
                // Bash behavior: `VAR=val | cat` -> VAR is set in a subshell, then exit.
                // We'll skip it for now.
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

        let resolved = find_executable(name).unwrap_or_else(|| PathBuf::from(name));

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

        command.args(&cmd.args);
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

    // Wait for all children; use the last command's exit code.
    let mut last_code = 0;
    for (i, mut child) in children.into_iter().enumerate() {
        let code = child.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1);
        if i == last_idx {
            last_code = code;
        }
    }

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
