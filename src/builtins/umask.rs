use crate::builtins::registry::CommandInfo;
use crate::engine::state::{ExecutionResult, ShellState};

pub const COMMAND_INFO_UMASK: CommandInfo = CommandInfo {
    name: "sys.umask",
    description: "Display or set file mode creation mask.",
    usage: "sys.umask [-p] [-S] [mode]\n\nDisplay or set file mode creation mask.",
    run: umask_runner,
};

pub fn umask_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    #[cfg(unix)]
    {
        if args.is_empty() {
            let mask = unsafe { nix::libc::umask(0) };
            unsafe { nix::libc::umask(mask) };
            println!("{:04o}", mask);
            return (ExecutionResult::KeepRunning, 0);
        } else if let Ok(val) = u32::from_str_radix(&args[0], 8) {
            unsafe { nix::libc::umask(val as _) };
            return (ExecutionResult::KeepRunning, 0);
        } else {
            eprintln!("cerf: umask: {}: octal number required", args[0]);
            return (ExecutionResult::KeepRunning, 1);
        }
    }

    #[cfg(windows)]
    {
        if args.is_empty() {
            println!("0000"); // Stub for Windows
        }
        (ExecutionResult::KeepRunning, 0)
    }
}
