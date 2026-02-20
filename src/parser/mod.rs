mod ast;
mod combinators;
mod expand;

// Re-export the public surface so that `crate::parser::*` keeps working
// for all existing callers (engine.rs, main.rs, etc.).
pub use ast::{CommandEntry, Connector, ParsedCommand};
pub use expand::expand_env_vars;

use combinators::{parse_connector, parse_single_command};

// ── Public API ────────────────────────────────────────────────────────────

/// Parse an entire input line into a list of [`CommandEntry`] items.
///
/// Returns `None` if the line is empty or a comment.
/// Returns `Some(entries)` where `entries` has at least one element.
pub fn parse_pipeline(input: &str) -> Option<Vec<CommandEntry>> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    // Expand environment variables before handing the line to nom.
    let expanded = expand_env_vars(input);
    let s = expanded.trim();

    let mut entries: Vec<CommandEntry> = Vec::new();
    let mut rest = s;

    // Parse the first command (no leading connector).
    let (after_first, first_cmd) = match parse_single_command(rest) {
        Ok(v) => v,
        Err(_) => return None,
    };
    entries.push(CommandEntry { connector: None, command: first_cmd });
    rest = after_first;

    // Parse (connector, command) pairs until input is exhausted.
    loop {
        if rest.trim().is_empty() {
            break;
        }
        let (after_conn, conn) = match parse_connector(rest) {
            Ok(v) => v,
            Err(_) => break,
        };
        let (after_cmd, cmd) = match parse_single_command(after_conn) {
            Ok(v) => v,
            Err(_) => break,
        };
        entries.push(CommandEntry { connector: Some(conn), command: cmd });
        rest = after_cmd;
    }

    if entries.is_empty() { None } else { Some(entries) }
}

/// Backwards-compatible single-command parse (used in tests & legacy paths).
#[allow(dead_code)]
pub fn parse_line(input: &str) -> Option<ParsedCommand> {
    parse_pipeline(input).and_then(|mut v| if v.len() == 1 { Some(v.remove(0).command) } else { None })
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── single-command tests ───────────────────────────────────────────────

    #[test]
    fn test_parse_simple() {
        let cmd = parse_line("ls -la").unwrap();
        assert_eq!(cmd.name, "ls");
        assert_eq!(cmd.args, vec!["-la"]);
    }

    #[test]
    fn test_parse_quoted() {
        let cmd = parse_line("echo \"hello world\"").unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["hello world"]);
    }

    #[test]
    fn test_parse_mixed() {
        let cmd = parse_line("cd \"My Documents\" backup").unwrap();
        assert_eq!(cmd.name, "cd");
        assert_eq!(cmd.args, vec!["My Documents", "backup"]);
    }

    #[test]
    fn test_extra_spaces() {
        let cmd = parse_line("  ls   -la  ").unwrap();
        assert_eq!(cmd.name, "ls");
        assert_eq!(cmd.args, vec!["-la"]);
    }

    #[test]
    fn test_empty() {
        assert!(parse_line("").is_none());
        assert!(parse_line("   ").is_none());
    }

    #[test]
    fn test_comment() {
        assert!(parse_line("# comment").is_none());
        assert!(parse_line("   # comment indented").is_none());
    }

    // ── connector / pipeline tests ────────────────────────────────────────

    #[test]
    fn test_semicolon_two_commands() {
        let entries = parse_pipeline("echo hello ; echo world").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].connector, None);
        assert_eq!(entries[0].command.name, "echo");
        assert_eq!(entries[1].connector, Some(Connector::Semi));
        assert_eq!(entries[1].command.name, "echo");
    }

    #[test]
    fn test_and_operator() {
        let entries = parse_pipeline("make && make install").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].connector, None);
        assert_eq!(entries[0].command.name, "make");
        assert_eq!(entries[1].connector, Some(Connector::And));
        assert_eq!(entries[1].command.name, "make");
        assert_eq!(entries[1].command.args, vec!["install"]);
    }

    #[test]
    fn test_or_operator() {
        let entries = parse_pipeline("cat file.txt || echo missing").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command.name, "cat");
        assert_eq!(entries[1].connector, Some(Connector::Or));
        assert_eq!(entries[1].command.name, "echo");
        assert_eq!(entries[1].command.args, vec!["missing"]);
    }

    #[test]
    fn test_chained_operators() {
        let entries = parse_pipeline("a && b || c ; d").unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[1].connector, Some(Connector::And));
        assert_eq!(entries[2].connector, Some(Connector::Or));
        assert_eq!(entries[3].connector, Some(Connector::Semi));
    }

    // ── integration: parse_pipeline with env expansion ────────────────────

    #[test]
    fn test_parse_line_expands_var_in_arg() {
        unsafe { std::env::set_var("CERF_DIR", "/tmp/test"); }
        let cmd = parse_line("cd $CERF_DIR").unwrap();
        assert_eq!(cmd.name, "cd");
        assert_eq!(cmd.args, vec!["/tmp/test"]);
        unsafe { std::env::remove_var("CERF_DIR"); }
    }

    #[test]
    fn test_parse_line_expands_var_in_quoted_arg() {
        unsafe { std::env::set_var("CERF_MSG", "hello world"); }
        let cmd = parse_line("echo \"$CERF_MSG\"").unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["hello world"]);
        unsafe { std::env::remove_var("CERF_MSG"); }
    }

    #[test]
    fn test_parse_line_expands_path_var() {
        let path_val = std::env::var("PATH").unwrap_or_default();
        let expanded = expand_env_vars("echo $PATH");
        assert!(expanded.contains(&path_val), "expanded line should contain the PATH value");
    }
}
