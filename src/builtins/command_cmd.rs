use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO_COMMAND: CommandInfo = CommandInfo {
    name: "sys.command",
    description: "Run a simple command ignoring aliases.",
    usage: "sys.command [arg ...]\n\nRuns a command ignoring shell functions and aliases.",
    run: command_runner,
};

pub fn command_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        return (ExecutionResult::KeepRunning, 0);
    }
    
    let name = &args[0];
    
    if let Some(cmd_info) = crate::builtins::registry::find_command(name) {
        return (cmd_info.run)(&args[1..], state);
    }
    
    // external executable
    let resolved = crate::engine::path::find_executable(name).unwrap_or_else(|| crate::engine::path::expand_home(name));
    
    #[cfg(windows)]
    let mut command = {
        let is_batch = resolved.extension().map_or(false, |e| {
            let e = e.to_string_lossy().to_lowercase();
            e == "cmd" || e == "bat"
        });
        if is_batch {
            let mut c = std::process::Command::new("cmd");
            c.arg("/c").arg(&resolved);
            c
        } else {
            std::process::Command::new(&resolved)
        }
    };
    
    #[cfg(unix)]
    let mut command = std::process::Command::new(&resolved);

    // This bypasses proper process job control for Cerf but works as a simple implementation for now.
    command.args(&args[1..]);
    
    let code = match command.status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!("cerf: command not found: {}", name);
            } else {
                eprintln!("cerf: command error: {}", e);
            }
            127
        }
    };
    
    (ExecutionResult::KeepRunning, code)
}
