use std::path::PathBuf;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;


pub const COMMAND_INFO_EXIT: CommandInfo = CommandInfo {
    name: "exit",
    description: "Exit the shell.",
    usage: "exit\n\nExit the shell.",
    run: exit_runner,
};

pub fn exit_runner(_args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    exit();
    (ExecutionResult::Exit, 0)
}

pub const COMMAND_INFO_CLEAR: CommandInfo = CommandInfo {
    name: "clear",
    description: "Clear the terminal screen.",
    usage: "clear\n\nClear the terminal screen.",
    run: clear_runner,
};

pub fn clear_runner(_args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    clear();
    (ExecutionResult::KeepRunning, 0)
}

pub const COMMAND_INFO_EXEC: CommandInfo = CommandInfo {
    name: "exec",
    description: "Replace the shell with the given command.",
    usage: "exec [command [arguments ...]]\n\nReplace the shell with the given command.",
    run: exec_runner,
};

pub fn exec_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    match exec(args) {
        Ok(code) => (ExecutionResult::Exit, code),
        Err(e) => {
            eprintln!("{}", e);
            (ExecutionResult::KeepRunning, 1)
        }
    }
}
use std::process::Command;

use crate::engine::path::{expand_home, find_executable};

pub fn exit() {
    // No-op here, handled by engine return value
}

pub fn clear() {
    use std::io::{self, Write};
    print!("\x1B[2J\x1B[1;1H");
    let _ = io::stdout().flush();
}

/// Run the `exec` built-in.
///
/// Replaces the current shell process with the given command.
/// - On **Unix** this calls `execvp` and never returns on success.
/// - On **Windows** there is no real `exec`, so we spawn the process, wait for
///   it, and return its exit code; the caller should exit the shell.
///
/// If no command is given, `exec` is a no-op (returns success).
pub fn exec(args: &[String]) -> Result<i32, String> {
    if args.is_empty() {
        // No command — just succeed (POSIX: `exec` with no args is a no-op).
        return Ok(0);
    }

    let cmd_name = &args[0];
    let cmd_args = &args[1..];

    let resolved: PathBuf = find_executable(cmd_name)
        .unwrap_or_else(|| expand_home(cmd_name));

    // ── Unix: true exec (replaces the process image) ─────────────────
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        // Restore default signal handling before exec-ing.
        let err = Command::new(&resolved)
            .args(cmd_args)
            .exec(); // never returns on success

        return Err(format!("cerf: exec: {}: {}", cmd_name, err));
    }

    // ── Windows: spawn + exit (best-effort emulation) ────────────────
    #[cfg(windows)]
    {
        let is_batch = resolved.extension().map_or(false, |e| {
            let e = e.to_string_lossy().to_lowercase();
            e == "cmd" || e == "bat"
        });

        let mut command = if is_batch {
            let mut c = Command::new("cmd");
            c.arg("/c").arg(&resolved);
            c
        } else {
            Command::new(&resolved)
        };

        command.args(cmd_args);

        match command.spawn() {
            Ok(mut child) => {
                let code = child
                    .wait()
                    .map(|s| s.code().unwrap_or(1))
                    .unwrap_or(1);
                Ok(code)
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(format!("cerf: exec: {}: command not found", cmd_name))
                } else {
                    Err(format!("cerf: exec: {}: {}", cmd_name, e))
                }
            }
        }
    }
}
