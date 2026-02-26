use std::collections::HashMap;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "unset",
    description: "Unset values and attributes of shell variables and functions.",
    usage: "unset [-f] [-v] [-n] [name ...]\n\nUnset values and attributes of shell variables and functions.",
    run: unset_runner,
};

pub fn unset_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    run(args, &mut state.variables);
    (ExecutionResult::KeepRunning, 0)
}

/// Run the `unset` builtin.
///
/// Behaviour:
/// - `unset name …` → remove each named variable from shell and environment
pub fn run(args: &[String], variables: &mut HashMap<String, String>) {
    if args.is_empty() {
        return;
    }

    for arg in args {
        // Bash allows 'unset' to fail silently if the variable doesn't exist.
        // It also removes it from the environment.
        variables.remove(arg);
        unsafe { std::env::remove_var(arg); }
    }
}
