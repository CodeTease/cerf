use std::collections::HashMap;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "alias.set",
    description: "Define or display aliases.",
    usage: "alias.set [name[=value] ... ]\n\nAlias with no arguments or with the -p option prints the list of aliases in the form alias NAME=VALUE on standard output. Otherwise, an alias is defined for each NAME whose VALUE is given.",
    run: alias_runner,
};

pub fn alias_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    run(args, &mut state.aliases);
    (ExecutionResult::KeepRunning, 0)
}

/// Run the `alias` builtin.
///
/// Behaviour mirrors bash/zsh:
/// - `alias`              → print all aliases sorted by name
/// - `alias name`         → print the definition of `name` (error if not set)
/// - `alias name=value`   → define an alias
/// - Multiple mixed args are accepted in a single call.
pub fn run(args: &[String], aliases: &mut HashMap<String, String>) {
    if args.is_empty() {
        // Print all aliases, sorted for deterministic output.
        let mut pairs: Vec<(&String, &String)> = aliases.iter().collect();
        pairs.sort_by_key(|(k, _)| k.as_str());
        for (name, value) in pairs {
            println!("alias {}='{}'", name, value);
        }
        return;
    }

    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            // Definition: name=value
            let name = arg[..eq_pos].to_string();
            let value = arg[eq_pos + 1..].to_string();
            if name.is_empty() {
                eprintln!("cerf: alias.set: '{}': invalid alias name", arg);
            } else if crate::builtins::registry::find_command(&name).is_some() {
                eprintln!("cerf: alias.set: '{}': cannot override builtin command", name);
            } else {
                aliases.insert(name, value);
            }
        } else {
            // Query: print the existing definition or report an error.
            match aliases.get(arg.as_str()) {
                Some(value) => println!("alias {}='{}'", arg, value),
                None => eprintln!("cerf: alias.set: {}: not found", arg),
            }
        }
    }
}
