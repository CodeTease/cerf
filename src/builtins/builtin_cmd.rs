use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO_BUILTIN: CommandInfo = CommandInfo {
    name: "sys.builtin",
    description: "Run a shell builtin.",
    usage: "sys.builtin [shell-builtin [arg ...]]\n\nExecute the specified shell builtin, passing it arguments, and return its exit status. Useful when a function has the same name as a builtin.",
    run: builtin_runner,
};

pub fn builtin_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        return (ExecutionResult::KeepRunning, 0);
    }
    let name = &args[0];
    
    if let Some(cmd_info) = crate::builtins::registry::find_command(name) {
        return (cmd_info.run)(&args[1..], state);
    }
    
    eprintln!("cerf: builtin: {}: not a shell builtin", name);
    (ExecutionResult::KeepRunning, 1)
}
