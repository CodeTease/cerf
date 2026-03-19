use crate::builtins::registry::CommandInfo;
use crate::engine::state::{ExecutionResult, ShellState};
use std::collections::HashMap;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "sys.type",
    description: "Display information about command type.",
    usage: "sys.type [-afptP] name [name ...]\n\nDisplay information about command type.",
    run: type_runner,
};

pub fn type_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    run(args, &state.aliases);
    (ExecutionResult::KeepRunning, 0)
}

/// Return the type description for a single command name.
pub fn type_of(cmd: &str, aliases: &HashMap<String, String>) -> String {
    // 1. Check aliases first (they shadow everything else, just like bash).
    if let Some(value) = aliases.get(cmd) {
        return format!("{} is aliased to `{}`", cmd, value);
    }

    // 2. Shell keywords.
    if crate::parser::is_reserved_word(cmd) {
        return format!("{} is a shell keyword", cmd);
    }

    // 3. Shell builtins.
    if crate::builtins::registry::find_command(cmd).is_some() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_of_keyword() {
        let aliases = HashMap::new();
        assert_eq!(type_of("if", &aliases), "if is a shell keyword");
        assert_eq!(type_of("while", &aliases), "while is a shell keyword");
        assert_eq!(type_of("for", &aliases), "for is a shell keyword");
        assert_eq!(type_of("{", &aliases), "{ is a shell keyword");
        assert_eq!(type_of("!", &aliases), "! is a shell keyword");
    }

    #[test]
    fn test_type_of_builtin() {
        let aliases = HashMap::new();
        // Internal names are prefixed
        assert_eq!(type_of("dir.cd", &aliases), "dir.cd is a shell builtin");
        assert_eq!(type_of("io.echo", &aliases), "io.echo is a shell builtin");
    }

    #[test]
    fn test_type_of_alias() {
        let mut aliases = HashMap::new();
        aliases.insert("cd".to_string(), "dir.cd".to_string());
        assert_eq!(type_of("cd", &aliases), "cd is aliased to `dir.cd`");
    }
}
