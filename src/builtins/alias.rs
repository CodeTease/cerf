use std::collections::HashMap;

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
                eprintln!("cerf: alias: '{}': invalid alias name", arg);
            } else {
                aliases.insert(name, value);
            }
        } else {
            // Query: print the existing definition or report an error.
            match aliases.get(arg.as_str()) {
                Some(value) => println!("alias {}='{}'", arg, value),
                None => eprintln!("cerf: alias: {}: not found", arg),
            }
        }
    }
}
