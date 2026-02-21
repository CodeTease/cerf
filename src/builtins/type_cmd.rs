use std::collections::HashMap;

/// Return the type description for a single command name.
pub fn type_of(cmd: &str, aliases: &HashMap<String, String>) -> String {
    // 1. Check aliases first (they shadow everything else, just like bash).
    if let Some(value) = aliases.get(cmd) {
        return format!("{} is aliased to `{}`", cmd, value);
    }

    // 2. Shell builtins.
    let builtins = ["alias", "unalias", "cd", "pwd", "exit", "clear", "echo", "type", "export", "unset", "source", ".", "history", "pushd", "popd", "dirs", "exec", "read", "true", "false", "test", "[", "set"];
    if builtins.contains(&cmd) {
        return format!("{} is a shell builtin", cmd);
    }

    // 3. Search PATH and other locations.
    if let Some(path) = crate::engine::find_executable(cmd) {
        return format!("{} is {}", cmd, path.display());
    }

    format!("cerf: type: {}: not found", cmd)
}

pub fn run(args: &[String], aliases: &HashMap<String, String>) {
    if args.is_empty() {
        return;
    }

    for cmd in args {
        let desc = type_of(cmd, aliases);
        if desc.starts_with("cerf: type:") {
            eprintln!("{}", desc);
        } else {
            println!("{}", desc);
        }
    }
}
