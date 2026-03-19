mod ast;
mod combinators;
mod expand;

// Re-export the public surface so that `crate::parser::*` keeps working
// for all existing callers (engine.rs, main.rs, etc.).
pub use ast::{Arg, CommandEntry, CommandNode, Connector, Pipeline, Redirect, RedirectKind};
pub use combinators::is_reserved_word;
pub use expand::expand_vars;

// ── Public API ────────────────────────────────────────────────────────────

pub fn join_continuations(input: &str) -> String {
    let mut result = String::new();
    let mut lines = input.lines().peekable();
    while let Some(line) = lines.next() {
        let mut current = line.to_string();
        while current.trim_end().ends_with(',') && lines.peek().is_some() {
            let trimmed = current.trim_end();
            current = trimmed[..trimmed.len() - 1].to_string();
            if let Some(next_line) = lines.next() {
                current.push_str(next_line);
            }
        }
        result.push_str(&current);
        result.push('\n');
    }
    if input.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    result.trim_end_matches('\n').to_string()
}

/// Check whether the input looks incomplete and the shell should keep
/// reading more lines before attempting to parse.
///
/// This is a lightweight heuristic (no full parse) that catches the most
/// common multi-line patterns:
/// - Unbalanced `{` / `}` braces (control-flow blocks)
/// - A trailing connector / pipe (`|`, `&&`, `||`)
/// - A trailing comma (Cerf's explicit line-continuation character)
pub fn is_incomplete(input: &str) -> bool {
    let joined = join_continuations(input);
    let s = joined.trim();
    if s.is_empty() {
        return false;
    }

    // 1. Trailing comma → explicit continuation.
    if s.ends_with(',') {
        return true;
    }

    // 2. Trailing connector / pipe → next line has the RHS.
    if s.ends_with('|') || s.ends_with("&&") || s.ends_with("||") {
        return true;
    }

    // 3. Unbalanced braces (skip characters inside quotes).
    let mut depth: i32 = 0;
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                for c in chars.by_ref() {
                    if c == '"' {
                        break;
                    }
                }
            }
            '\'' => {
                for c in chars.by_ref() {
                    if c == '\'' {
                        break;
                    }
                }
            }
            '#' => {
                // skip line comments
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
    }
    depth > 0
}

/// Parse an entire input line into a list of [`CommandEntry`] items.
///
/// Returns `None` if the line is empty or a comment.
/// Returns `Some(entries)` where `entries` has at least one element.
pub fn parse_input(
    input: &str,
    shell_vars: &std::collections::HashMap<String, crate::engine::state::Variable>,
) -> Option<Vec<CommandEntry>> {
    let preprocessed = join_continuations(input);
    let expanded = expand_vars(&preprocessed, shell_vars);
    let s = expanded.trim();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }

    match combinators::parse_command_list(s) {
        Ok((rem, entries)) => {
            if !rem.trim().is_empty() {
                eprintln!("cerf: syntax error near unexpected token '{}'", rem.trim());
                return None;
            }
            if entries.is_empty() {
                None
            } else {
                Some(entries)
            }
        }
        Err(_) => {
            eprintln!("cerf: syntax error: incomplete or invalid command");
            None
        }
    }
}

/// Backwards-compatible alias — kept so call-sites in main.rs don't break.
pub fn parse_pipeline(
    input: &str,
    shell_vars: &std::collections::HashMap<String, crate::engine::state::Variable>,
) -> Option<Vec<CommandEntry>> {
    parse_input(input, shell_vars)
}

pub fn parse_line(input: &str) -> Option<CommandNode> {
    parse_line_with_vars(input, &std::collections::HashMap::new())
}

pub fn parse_line_with_vars(
    input: &str,
    vars: &std::collections::HashMap<String, crate::engine::state::Variable>,
) -> Option<CommandNode> {
    parse_input(input, vars).and_then(|mut v| {
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
    use crate::parser::ast::arg_values;

    // ── single-command tests ───────────────────────────────────────────────

    #[test]
    fn test_parse_simple() {
        let cmd = parse_line("ls -la").unwrap();
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("ls"));
        assert_eq!(arg_values(cmd.args()), vec!["-la"]);
    }

    #[test]
    fn test_parse_quoted() {
        let cmd = parse_line("echo \"hello world\"").unwrap();
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("echo"));
        assert_eq!(arg_values(cmd.args()), vec!["hello world"]);
    }

    #[test]
    fn test_parse_mixed() {
        let cmd = parse_line("cd \"My Documents\" backup").unwrap();
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("cd"));
        assert_eq!(arg_values(cmd.args()), vec!["My Documents", "backup"]);
    }

    #[test]
    fn test_extra_spaces() {
        let cmd = parse_line("  ls   -la  ").unwrap();
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("ls"));
        assert_eq!(arg_values(cmd.args()), vec!["-la"]);
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
    fn test_newline_separator() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("echo hello\necho world", &vars).unwrap();
        // If newlines are separators, we should have 2 entries.
        // If they are just whitespace, we'll have 1 entry with 3 args.
        assert_eq!(entries.len(), 2, "Newline should be a command separator");
    }

    #[test]
    fn test_semicolon_two_commands() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("echo hello ; echo world", &vars).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].connector, None);
        assert_eq!(
            entries[0].pipeline.commands[0].name().map(|n| n.as_str()),
            Some("echo")
        );
        assert_eq!(entries[1].connector, Some(Connector::Semi));
        assert_eq!(
            entries[1].pipeline.commands[0].name().map(|n| n.as_str()),
            Some("echo")
        );
    }

    #[test]
    fn test_and_operator() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("make && make install", &vars).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].connector, None);
        assert_eq!(
            entries[0].pipeline.commands[0].name().map(|n| n.as_str()),
            Some("make")
        );
        assert_eq!(entries[1].connector, Some(Connector::And));
        assert_eq!(
            entries[1].pipeline.commands[0].name().map(|n| n.as_str()),
            Some("make")
        );
        assert_eq!(
            arg_values(entries[1].pipeline.commands[0].args()),
            vec!["install"]
        );
    }

    #[test]
    fn test_or_operator() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("cat file.txt || echo missing", &vars).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].pipeline.commands[0].name().map(|n| n.as_str()),
            Some("cat")
        );
        assert_eq!(entries[1].connector, Some(Connector::Or));
        assert_eq!(
            entries[1].pipeline.commands[0].name().map(|n| n.as_str()),
            Some("echo")
        );
        assert_eq!(
            arg_values(entries[1].pipeline.commands[0].args()),
            vec!["missing"]
        );
    }

    #[test]
    fn test_chained_operators() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("a && b || c ; d", &vars).unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[1].connector, Some(Connector::And));
        assert_eq!(entries[2].connector, Some(Connector::Or));
        assert_eq!(entries[3].connector, Some(Connector::Semi));
    }

    // ── piping tests ──────────────────────────────────────────────────────

    #[test]
    fn test_single_pipe() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("ls | grep foo", &vars).unwrap();
        assert_eq!(entries.len(), 1);
        let pipeline = &entries[0].pipeline;
        assert_eq!(pipeline.commands.len(), 2);
        assert_eq!(pipeline.commands[0].name().map(|n| n.as_str()), Some("ls"));
        assert_eq!(
            pipeline.commands[1].name().map(|n| n.as_str()),
            Some("grep")
        );
        assert_eq!(arg_values(pipeline.commands[1].args()), vec!["foo"]);
    }

    #[test]
    fn test_multi_pipe() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("cat f | sort | uniq", &vars).unwrap();
        assert_eq!(entries.len(), 1);
        let pipeline = &entries[0].pipeline;
        assert_eq!(pipeline.commands.len(), 3);
        assert_eq!(pipeline.commands[0].name().map(|n| n.as_str()), Some("cat"));
        assert_eq!(
            pipeline.commands[1].name().map(|n| n.as_str()),
            Some("sort")
        );
        assert_eq!(
            pipeline.commands[2].name().map(|n| n.as_str()),
            Some("uniq")
        );
    }

    #[test]
    fn test_not_operator() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("! ls", &vars).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].pipeline.negated);
        assert_eq!(
            entries[0].pipeline.commands[0].name().map(|n| n.as_str()),
            Some("ls")
        );

        let entries = parse_pipeline("!  ls -la", &vars).unwrap();
        assert_eq!(
            entries[0].pipeline.commands[0].name().map(|n| n.as_str()),
            Some("ls")
        );
        assert!(entries[0].pipeline.negated);
    }

    #[test]
    fn test_not_with_pipe() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("! ls | grep foo", &vars).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].pipeline.negated);
        assert_eq!(entries[0].pipeline.commands.len(), 2);
    }

    #[test]
    fn test_pipe_with_connectors() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("ls | grep foo && echo done", &vars).unwrap();
        assert_eq!(entries.len(), 2);
        // First entry is a pipeline: ls | grep foo
        assert_eq!(entries[0].pipeline.commands.len(), 2);
        assert_eq!(
            entries[0].pipeline.commands[0].name().map(|n| n.as_str()),
            Some("ls")
        );
        assert_eq!(
            entries[0].pipeline.commands[1].name().map(|n| n.as_str()),
            Some("grep")
        );
        // Second entry is a simple command: echo done
        assert_eq!(entries[1].connector, Some(Connector::And));
        assert_eq!(entries[1].pipeline.commands.len(), 1);
        assert_eq!(
            entries[1].pipeline.commands[0].name().map(|n| n.as_str()),
            Some("echo")
        );
    }

    // ── redirection tests ─────────────────────────────────────────────────

    #[test]
    fn test_redirect_stdout() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("echo hi > out.txt", &vars).unwrap();
        let cmd = &entries[0].pipeline.commands[0];
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("echo"));
        assert_eq!(arg_values(cmd.args()), vec!["hi"]);
        assert_eq!(cmd.redirects().len(), 1);
        assert_eq!(cmd.redirects()[0].kind, RedirectKind::StdoutOverwrite);
        assert_eq!(cmd.redirects()[0].file, "out.txt");
    }

    #[test]
    fn test_redirect_append() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("echo hi >> out.txt", &vars).unwrap();
        let cmd = &entries[0].pipeline.commands[0];
        assert_eq!(cmd.redirects().len(), 1);
        assert_eq!(cmd.redirects()[0].kind, RedirectKind::StdoutAppend);
        assert_eq!(cmd.redirects()[0].file, "out.txt");
    }

    #[test]
    fn test_redirect_stdin() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("sort < in.txt", &vars).unwrap();
        let cmd = &entries[0].pipeline.commands[0];
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("sort"));
        assert_eq!(cmd.redirects().len(), 1);
        assert_eq!(cmd.redirects()[0].kind, RedirectKind::StdinFrom);
        assert_eq!(cmd.redirects()[0].file, "in.txt");
    }

    #[test]
    fn test_pipe_with_redirect() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("cat < in.txt | sort > out.txt", &vars).unwrap();
        let pipeline = &entries[0].pipeline;
        assert_eq!(pipeline.commands.len(), 2);
        // First command: cat < in.txt
        assert_eq!(pipeline.commands[0].name().map(|n| n.as_str()), Some("cat"));
        assert_eq!(pipeline.commands[0].redirects().len(), 1);
        assert_eq!(
            pipeline.commands[0].redirects()[0].kind,
            RedirectKind::StdinFrom
        );
        // Last command: sort > out.txt
        assert_eq!(
            pipeline.commands[1].name().map(|n| n.as_str()),
            Some("sort")
        );
        assert_eq!(pipeline.commands[1].redirects().len(), 1);
        assert_eq!(
            pipeline.commands[1].redirects()[0].kind,
            RedirectKind::StdoutOverwrite
        );
    }

    // ── integration: parse_pipeline with env expansion ────────────────────

    #[test]
    fn test_parse_line_expands_var_in_arg() {
        let mut vars = std::collections::HashMap::new();
        vars.insert(
            "CERF_DIR".to_string(),
            crate::engine::state::Variable::new_string("/tmp/test".to_string()),
        );
        let cmd = parse_line_with_vars("cd $CERF_DIR", &vars).unwrap();
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("cd"));
        assert_eq!(arg_values(cmd.args()), vec!["/tmp/test"]);
    }

    #[test]
    fn test_parse_line_expands_var_in_quoted_arg() {
        let mut vars = std::collections::HashMap::new();
        vars.insert(
            "CERF_MSG".to_string(),
            crate::engine::state::Variable::new_string("hello world".to_string()),
        );
        let cmd = parse_line_with_vars("echo \"$CERF_MSG\"", &vars).unwrap();
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("echo"));
        assert_eq!(arg_values(cmd.args()), vec!["hello world"]);
    }

    #[test]
    fn test_parse_line_expands_path_var() {
        let mut vars = std::collections::HashMap::new();
        vars.insert(
            "PATH".to_string(),
            crate::engine::state::Variable::new_string("some_path".to_string()),
        );
        let expanded = expand_vars("echo $PATH", &vars);
        assert!(
            expanded.contains("some_path"),
            "expanded line should contain the PATH value"
        );
    }

    // ── shell variable tests ──────────────────────────────────────────────

    #[test]
    fn test_parse_assignment_only() {
        let cmd = parse_line("FOO=bar").unwrap();
        assert!(cmd.name().is_none());
        assert_eq!(cmd.assignments(), &[("FOO".to_string(), "bar".to_string())]);
    }

    #[test]
    fn test_parse_multiple_assignments() {
        let cmd = parse_line("A=1 B=2 C=3").unwrap();
        assert_eq!(cmd.assignments().len(), 3);
        assert_eq!(cmd.assignments()[0], ("A".to_string(), "1".to_string()));
        assert_eq!(cmd.assignments()[2], ("C".to_string(), "3".to_string()));
    }

    #[test]
    fn test_parse_assignment_with_command() {
        let cmd = parse_line("VAR=val ls -l").unwrap();
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("ls"));
        assert_eq!(cmd.assignments(), &[("VAR".to_string(), "val".to_string())]);
        assert_eq!(arg_values(cmd.args()), vec!["-l"]);
    }

    #[test]
    fn test_parse_assignment_quoted_value() {
        let cmd = parse_line("MSG=\"hello world\" echo").unwrap();
        assert_eq!(
            cmd.assignments(),
            &[("MSG".to_string(), "hello world".to_string())]
        );
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("echo"));
    }

    // ── control flow tests ──────────────────────────────────────────────────

    #[test]
    fn test_parse_if_simple() {
        let cmd = parse_line("if true { echo ok }").unwrap();
        match cmd {
            CommandNode::If {
                branches,
                else_branch,
                ..
            } => {
                assert_eq!(branches.len(), 1);
                assert!(else_branch.is_none());
            }
            _ => panic!("Expected If node"),
        }
    }

    #[test]
    fn test_parse_if_elif_else() {
        let cmd = parse_line("if cmd1 { echo 1 } elif cmd2 { echo 2 } else { echo 3 }").unwrap();
        match cmd {
            CommandNode::If {
                branches,
                else_branch,
                ..
            } => {
                assert_eq!(branches.len(), 2);
                assert!(else_branch.is_some());
            }
            _ => panic!("Expected If node"),
        }
    }

    #[test]
    fn test_parse_func() {
        let cmd = parse_line("func my_func { echo ok }").unwrap();
        match cmd {
            CommandNode::FuncDecl { name, body } => {
                assert_eq!(name, "my_func");
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected FuncDecl node"),
        }
    }

    #[test]
    fn test_parse_for_loop() {
        let cmd = parse_line("for x in a b c { echo $x }").unwrap();
        match cmd {
            CommandNode::For {
                var, items, body, ..
            } => {
                assert_eq!(var, "x");
                assert_eq!(arg_values(&items), vec!["a", "b", "c"]);
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected For node"),
        }
    }

    #[test]
    fn test_parse_while_loop() {
        let cmd = parse_line("while true { echo ok }").unwrap();
        match cmd {
            CommandNode::While { cond, body, .. } => {
                assert_eq!(cond.len(), 1);
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected While node"),
        }
    }

    #[test]
    fn test_parse_loop() {
        let cmd = parse_line("loop { echo ok }").unwrap();
        match cmd {
            CommandNode::Loop { body, .. } => {
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected Loop node"),
        }
    }

    #[test]
    fn test_join_continuations() {
        let input = "echo hello ,\nworld";
        let joined = join_continuations(input);
        assert_eq!(joined, "echo hello world");

        let input = "ls ,\n  -l ,\n  /tmp";
        let joined = join_continuations(input);
        assert_eq!(joined, "ls   -l   /tmp");
    }

    // ── is_incomplete tests ───────────────────────────────────────────────

    #[test]
    fn test_incomplete_unbalanced_brace() {
        assert!(is_incomplete("if true {"));
        assert!(is_incomplete("if true {\n  echo ok"));
        assert!(!is_incomplete("if true { echo ok }"));
    }

    #[test]
    fn test_incomplete_trailing_pipe() {
        assert!(is_incomplete("ls |"));
        assert!(is_incomplete("echo hello &&"));
        assert!(is_incomplete("echo hello ||"));
        assert!(!is_incomplete("ls | grep foo"));
    }

    #[test]
    fn test_incomplete_trailing_comma() {
        assert!(is_incomplete("echo hello,"));
        assert!(!is_incomplete("echo hello"));
    }

    #[test]
    fn test_complete_multiline_if() {
        assert!(!is_incomplete("if true {\n  echo ok\n}"));
    }

    #[test]
    fn test_complete_simple() {
        assert!(!is_incomplete("echo hello"));
        assert!(!is_incomplete(""));
        assert!(!is_incomplete("   "));
    }

    // ── multi-line parse tests ────────────────────────────────────────────

    #[test]
    fn test_parse_multiline_if() {
        let vars = std::collections::HashMap::new();
        let input = "if true {\n  echo ok\n}";
        let entries = parse_pipeline(input, &vars).unwrap();
        assert_eq!(entries.len(), 1);
        match &entries[0].pipeline.commands[0] {
            CommandNode::If { branches, .. } => {
                assert_eq!(branches.len(), 1);
            }
            _ => panic!("Expected If node"),
        }
    }

    #[test]
    fn test_parse_multiline_for() {
        let vars = std::collections::HashMap::new();
        let input = "for x in a b c {\n  echo $x\n}";
        let entries = parse_pipeline(input, &vars).unwrap();
        assert_eq!(entries.len(), 1);
        match &entries[0].pipeline.commands[0] {
            CommandNode::For {
                var, items, body, ..
            } => {
                assert_eq!(var, "x");
                assert_eq!(arg_values(items), vec!["a", "b", "c"]);
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected For node"),
        }
    }

    #[test]
    fn test_parse_multiline_commands() {
        let vars = std::collections::HashMap::new();
        let input = "echo hello\necho world";
        let entries = parse_pipeline(input, &vars).unwrap();
        assert_eq!(entries.len(), 2);
    }
}
