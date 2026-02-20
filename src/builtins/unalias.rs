use std::collections::HashMap;

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
