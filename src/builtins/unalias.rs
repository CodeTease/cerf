use std::collections::HashMap;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "alias.unset",
    description: "Remove each NAME from the list of defined aliases.",
    usage: "alias.unset [-a] name [name ...]\n\nRemove each NAME from the list of defined aliases. If -a is supplied, all alias definitions are removed.",
    run: unalias_runner,
};

pub fn unalias_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    run(args, &mut state.aliases);
    (ExecutionResult::KeepRunning, 0)
}

/// Run the `unalias` builtin.
///
/// - `unalias name …` → remove each named alias (error if not set)
/// - `unalias -a`     → remove **all** aliases
pub fn run(args: &[String], aliases: &mut HashMap<String, String>) {
    if args.is_empty() {
        eprintln!("cerf: unalias: usage: unalias [-a] name [name …]");
        return;
    }

    if args.len() == 1 && args[0] == "-a" {
        aliases.clear();
        return;
    }

    for arg in args {
        if arg == "-a" {
            // -a mixed with names: honour it fully (clear everything)
            aliases.clear();
            return;
        }
        if aliases.remove(arg.as_str()).is_none() {
            eprintln!("cerf: unalias: {}: not found", arg);
        }
    }
}
