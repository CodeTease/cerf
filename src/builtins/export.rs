use std::collections::HashMap;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "env.export",
    description: "Set export attribute for shell variables.",
    usage: "env.export [name[=value] ...]\n\nMarks each NAME for automatic export to the environment of subsequently executed commands. If VALUE is supplied, assign VALUE before exporting.",
    run: export_runner,
};

pub fn export_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    run(args, &mut state.variables);
    (ExecutionResult::KeepRunning, 0)
}

/// Run the `export` builtin.
///
/// Behaviour:
/// - `export`             → print all environment variables
/// - `export name=value`  → set variable in both shell and environment
/// - `export name`        → promote existing shell variable to environment
pub fn run(args: &[String], variables: &mut HashMap<String, String>) {
    if args.is_empty() {
        let mut pairs: Vec<(String, String)> = std::env::vars().collect();
        pairs.sort_by_key(|(k, _)| k.clone());
        for (name, value) in pairs {
            println!("export {}='{}'", name, value);
        }
        return;
    }

    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            // Assignment: name=value
            let name = arg[..eq_pos].to_string();
            let value = arg[eq_pos + 1..].to_string();
            if name.is_empty() {
                eprintln!("cerf: export: '{}': not a valid identifier", arg);
            } else {
                variables.insert(name.clone(), value.clone());
                unsafe { std::env::set_var(name, value); }
            }
        } else {
            // Export existing: promote existing shell variable to env
            if let Some(value) = variables.get(arg) {
                unsafe { std::env::set_var(arg, value); }
            } else {
                // If not in shell variables but already in env, do nothing
                // If not in either, bash usually does nothing or adds it to exported list but empty.
                // For simplicity, we just check env.
                if std::env::var(arg).is_err() {
                   // Optional: bash allows 'export FOO' which marks FOO as exported even if not set.
                   // We'll skip that for now or just set as empty.
                }
            }
        }
    }
}
