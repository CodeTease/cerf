mod ast;
mod combinators;
mod expand;

// Re-export the public surface so that `crate::parser::*` keeps working
// for all existing callers (engine.rs, main.rs, etc.).
pub use ast::{CommandEntry, Connector, ParsedCommand, Pipeline, Redirect, RedirectKind};
pub use expand::expand_env_vars;

use combinators::{parse_connector, parse_pipeline_expr};

// ── Public API ────────────────────────────────────────────────────────────

/// Parse an entire input line into a list of [`CommandEntry`] items.
///
/// Returns `None` if the line is empty or a comment.
/// Returns `Some(entries)` where `entries` has at least one element.
pub fn parse_input(input: &str) -> Option<Vec<CommandEntry>> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    // Expand environment variables before handing the line to nom.
    let expanded = expand_env_vars(input);
    let s = expanded.trim();

    let mut entries: Vec<CommandEntry> = Vec::new();
    let mut rest = s;

    // Parse the first pipeline (no leading connector).
    let (after_first, first_pipeline) = match parse_pipeline_expr(rest) {
        Ok(v) => v,
        Err(_) => return None,
    };
    entries.push(CommandEntry { connector: None, pipeline: first_pipeline });
    rest = after_first;

    // Parse (connector, pipeline) pairs until input is exhausted.
    loop {
        if rest.trim().is_empty() {
            break;
        }
        let (after_conn, conn) = match parse_connector(rest) {
            Ok(v) => v,
            Err(_) => break,
        };
        let (after_pipeline, pipeline) = match parse_pipeline_expr(after_conn) {
            Ok(v) => v,
            Err(_) => break,
        };
        entries.push(CommandEntry { connector: Some(conn), pipeline });
        rest = after_pipeline;
    }

    if entries.is_empty() { None } else { Some(entries) }
}

/// Backwards-compatible alias — kept so call-sites in main.rs don't break.
pub fn parse_pipeline(input: &str) -> Option<Vec<CommandEntry>> {
    parse_input(input)
}

/// Backwards-compatible single-command parse (used in tests & legacy paths).
#[allow(dead_code)]
pub fn parse_line(input: &str) -> Option<ParsedCommand> {
    parse_input(input).and_then(|mut v| {
        if v.len() == 1 && v[0].pipeline.commands.len() == 1 {
            Some(v.remove(0).pipeline.commands.remove(0))
        } else {
            None
        }
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── single-command tests ───────────────────────────────────────────────

    #[test]
    fn test_parse_simple() {
        let cmd = parse_line("ls -la").unwrap();
        assert_eq!(cmd.name.as_deref(), Some("ls"));
        assert_eq!(cmd.args, vec!["-la"]);
    }

    #[test]
    fn test_parse_quoted() {
        let cmd = parse_line("echo \"hello world\"").unwrap();
        assert_eq!(cmd.name.as_deref(), Some("echo"));
        assert_eq!(cmd.args, vec!["hello world"]);
    }

    #[test]
    fn test_parse_mixed() {
        let cmd = parse_line("cd \"My Documents\" backup").unwrap();
        assert_eq!(cmd.name.as_deref(), Some("cd"));
        assert_eq!(cmd.args, vec!["My Documents", "backup"]);
    }

    #[test]
    fn test_extra_spaces() {
        let cmd = parse_line("  ls   -la  ").unwrap();
        assert_eq!(cmd.name.as_deref(), Some("ls"));
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
        assert_eq!(entries[0].pipeline.commands[0].name.as_deref(), Some("echo"));
        assert_eq!(entries[1].connector, Some(Connector::Semi));
        assert_eq!(entries[1].pipeline.commands[0].name.as_deref(), Some("echo"));
    }

    #[test]
    fn test_and_operator() {
        let entries = parse_pipeline("make && make install").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].connector, None);
        assert_eq!(entries[0].pipeline.commands[0].name.as_deref(), Some("make"));
        assert_eq!(entries[1].connector, Some(Connector::And));
        assert_eq!(entries[1].pipeline.commands[0].name.as_deref(), Some("make"));
        assert_eq!(entries[1].pipeline.commands[0].args, vec!["install"]);
    }

    #[test]
    fn test_or_operator() {
        let entries = parse_pipeline("cat file.txt || echo missing").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].pipeline.commands[0].name.as_deref(), Some("cat"));
        assert_eq!(entries[1].connector, Some(Connector::Or));
        assert_eq!(entries[1].pipeline.commands[0].name.as_deref(), Some("echo"));
        assert_eq!(entries[1].pipeline.commands[0].args, vec!["missing"]);
    }

    #[test]
    fn test_chained_operators() {
        let entries = parse_pipeline("a && b || c ; d").unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[1].connector, Some(Connector::And));
        assert_eq!(entries[2].connector, Some(Connector::Or));
        assert_eq!(entries[3].connector, Some(Connector::Semi));
    }

    // ── piping tests ──────────────────────────────────────────────────────

    #[test]
    fn test_single_pipe() {
        let entries = parse_pipeline("ls | grep foo").unwrap();
        assert_eq!(entries.len(), 1);
        let pipeline = &entries[0].pipeline;
        assert_eq!(pipeline.commands.len(), 2);
        assert_eq!(pipeline.commands[0].name.as_deref(), Some("ls"));
        assert_eq!(pipeline.commands[1].name.as_deref(), Some("grep"));
        assert_eq!(pipeline.commands[1].args, vec!["foo"]);
    }

    #[test]
    fn test_multi_pipe() {
        let entries = parse_pipeline("cat f | sort | uniq").unwrap();
        assert_eq!(entries.len(), 1);
        let pipeline = &entries[0].pipeline;
        assert_eq!(pipeline.commands.len(), 3);
        assert_eq!(pipeline.commands[0].name.as_deref(), Some("cat"));
        assert_eq!(pipeline.commands[1].name.as_deref(), Some("sort"));
        assert_eq!(pipeline.commands[2].name.as_deref(), Some("uniq"));
    }

    #[test]
    fn test_not_operator() {
        let entries = parse_pipeline("! ls").unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].pipeline.negated);
        assert_eq!(entries[0].pipeline.commands[0].name.as_deref(), Some("ls"));

        let entries = parse_pipeline("!  ls -la").unwrap();
        assert_eq!(entries[0].pipeline.commands[0].name.as_deref(), Some("ls"));
        assert!(entries[0].pipeline.negated);
    }

    #[test]
    fn test_not_with_pipe() {
        let entries = parse_pipeline("! ls | grep foo").unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].pipeline.negated);
        assert_eq!(entries[0].pipeline.commands.len(), 2);
    }

    #[test]
    fn test_pipe_with_connectors() {
        let entries = parse_pipeline("ls | grep foo && echo done").unwrap();
        assert_eq!(entries.len(), 2);
        // First entry is a pipeline: ls | grep foo
        assert_eq!(entries[0].pipeline.commands.len(), 2);
        assert_eq!(entries[0].pipeline.commands[0].name.as_deref(), Some("ls"));
        assert_eq!(entries[0].pipeline.commands[1].name.as_deref(), Some("grep"));
        // Second entry is a simple command: echo done
        assert_eq!(entries[1].connector, Some(Connector::And));
        assert_eq!(entries[1].pipeline.commands.len(), 1);
        assert_eq!(entries[1].pipeline.commands[0].name.as_deref(), Some("echo"));
    }

    // ── redirection tests ─────────────────────────────────────────────────

    #[test]
    fn test_redirect_stdout() {
        let entries = parse_pipeline("echo hi > out.txt").unwrap();
        let cmd = &entries[0].pipeline.commands[0];
        assert_eq!(cmd.name.as_deref(), Some("echo"));
        assert_eq!(cmd.args, vec!["hi"]);
        assert_eq!(cmd.redirects.len(), 1);
        assert_eq!(cmd.redirects[0].kind, RedirectKind::StdoutOverwrite);
        assert_eq!(cmd.redirects[0].file, "out.txt");
    }

    #[test]
    fn test_redirect_append() {
        let entries = parse_pipeline("echo hi >> out.txt").unwrap();
        let cmd = &entries[0].pipeline.commands[0];
        assert_eq!(cmd.redirects.len(), 1);
        assert_eq!(cmd.redirects[0].kind, RedirectKind::StdoutAppend);
        assert_eq!(cmd.redirects[0].file, "out.txt");
    }

    #[test]
    fn test_redirect_stdin() {
        let entries = parse_pipeline("sort < in.txt").unwrap();
        let cmd = &entries[0].pipeline.commands[0];
        assert_eq!(cmd.name.as_deref(), Some("sort"));
        assert_eq!(cmd.redirects.len(), 1);
        assert_eq!(cmd.redirects[0].kind, RedirectKind::StdinFrom);
        assert_eq!(cmd.redirects[0].file, "in.txt");
    }

    #[test]
    fn test_pipe_with_redirect() {
        let entries = parse_pipeline("cat < in.txt | sort > out.txt").unwrap();
        let pipeline = &entries[0].pipeline;
        assert_eq!(pipeline.commands.len(), 2);
        // First command: cat < in.txt
        assert_eq!(pipeline.commands[0].name.as_deref(), Some("cat"));
        assert_eq!(pipeline.commands[0].redirects.len(), 1);
        assert_eq!(pipeline.commands[0].redirects[0].kind, RedirectKind::StdinFrom);
        // Last command: sort > out.txt
        assert_eq!(pipeline.commands[1].name.as_deref(), Some("sort"));
        assert_eq!(pipeline.commands[1].redirects.len(), 1);
        assert_eq!(pipeline.commands[1].redirects[0].kind, RedirectKind::StdoutOverwrite);
    }

    // ── integration: parse_pipeline with env expansion ────────────────────

    #[test]
    fn test_parse_line_expands_var_in_arg() {
        unsafe { std::env::set_var("CERF_DIR", "/tmp/test"); }
        let cmd = parse_line("cd $CERF_DIR").unwrap();
        assert_eq!(cmd.name.as_deref(), Some("cd"));
        assert_eq!(cmd.args, vec!["/tmp/test"]);
        unsafe { std::env::remove_var("CERF_DIR"); }
    }

    #[test]
    fn test_parse_line_expands_var_in_quoted_arg() {
        unsafe { std::env::set_var("CERF_MSG", "hello world"); }
        let cmd = parse_line("echo \"$CERF_MSG\"").unwrap();
        assert_eq!(cmd.name.as_deref(), Some("echo"));
        assert_eq!(cmd.args, vec!["hello world"]);
        unsafe { std::env::remove_var("CERF_MSG"); }
    }

    #[test]
    fn test_parse_line_expands_path_var() {
        let path_val = std::env::var("PATH").unwrap_or_default();
        let expanded = expand_env_vars("echo $PATH");
        assert!(expanded.contains(&path_val), "expanded line should contain the PATH value");
    }

    // ── shell variable tests ──────────────────────────────────────────────

    #[test]
    fn test_parse_assignment_only() {
        let cmd = parse_line("FOO=bar").unwrap();
        assert!(cmd.name.is_none());
        assert_eq!(cmd.assignments, vec![("FOO".to_string(), "bar".to_string())]);
    }

    #[test]
    fn test_parse_multiple_assignments() {
        let cmd = parse_line("A=1 B=2 C=3").unwrap();
        assert_eq!(cmd.assignments.len(), 3);
        assert_eq!(cmd.assignments[0], ("A".to_string(), "1".to_string()));
        assert_eq!(cmd.assignments[2], ("C".to_string(), "3".to_string()));
    }

    #[test]
    fn test_parse_assignment_with_command() {
        let cmd = parse_line("VAR=val ls -l").unwrap();
        assert_eq!(cmd.name.as_deref(), Some("ls"));
        assert_eq!(cmd.assignments, vec![("VAR".to_string(), "val".to_string())]);
        assert_eq!(cmd.args, vec!["-l"]);
    }

    #[test]
    fn test_parse_assignment_quoted_value() {
        let cmd = parse_line("MSG=\"hello world\" echo").unwrap();
        assert_eq!(cmd.assignments, vec![("MSG".to_string(), "hello world".to_string())]);
        assert_eq!(cmd.name.as_deref(), Some("echo"));
    }
}
