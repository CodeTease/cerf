mod ast;
mod combinators;
mod expand;

// Re-export the public surface so that `crate::parser::*` keeps working
// for all existing callers (engine.rs, main.rs, etc.).
pub use ast::{Arg, CommandEntry, CommandNode, Connector, Pipeline, Redirect, RedirectKind};
pub use expand::expand_vars;



// ── Public API ────────────────────────────────────────────────────────────

/// Parse an entire input line into a list of [`CommandEntry`] items.
///
/// Returns `None` if the line is empty or a comment.
/// Returns `Some(entries)` where `entries` has at least one element.
pub fn parse_input(input: &str, shell_vars: &std::collections::HashMap<String, crate::engine::state::Variable>) -> Option<Vec<CommandEntry>> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    // Expand environment variables before handing the line to nom.
    let expanded = expand_vars(input, shell_vars);
    let s = expanded.trim();

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
        },
    }
}


/// Backwards-compatible alias — kept so call-sites in main.rs don't break.
pub fn parse_pipeline(input: &str, shell_vars: &std::collections::HashMap<String, crate::engine::state::Variable>) -> Option<Vec<CommandEntry>> {
    parse_input(input, shell_vars)
}

pub fn parse_line(input: &str) -> Option<CommandNode> {
    parse_line_with_vars(input, &std::collections::HashMap::new())
}

pub fn parse_line_with_vars(input: &str, vars: &std::collections::HashMap<String, crate::engine::state::Variable>) -> Option<CommandNode> {
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
    fn test_semicolon_two_commands() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("echo hello ; echo world", &vars).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].connector, None);
        assert_eq!(entries[0].pipeline.commands[0].name().map(|n| n.as_str()), Some("echo"));
        assert_eq!(entries[1].connector, Some(Connector::Semi));
        assert_eq!(entries[1].pipeline.commands[0].name().map(|n| n.as_str()), Some("echo"));
    }

    #[test]
    fn test_and_operator() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("make && make install", &vars).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].connector, None);
        assert_eq!(entries[0].pipeline.commands[0].name().map(|n| n.as_str()), Some("make"));
        assert_eq!(entries[1].connector, Some(Connector::And));
        assert_eq!(entries[1].pipeline.commands[0].name().map(|n| n.as_str()), Some("make"));
        assert_eq!(arg_values(entries[1].pipeline.commands[0].args()), vec!["install"]);
    }

    #[test]
    fn test_or_operator() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("cat file.txt || echo missing", &vars).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].pipeline.commands[0].name().map(|n| n.as_str()), Some("cat"));
        assert_eq!(entries[1].connector, Some(Connector::Or));
        assert_eq!(entries[1].pipeline.commands[0].name().map(|n| n.as_str()), Some("echo"));
        assert_eq!(arg_values(entries[1].pipeline.commands[0].args()), vec!["missing"]);
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
        assert_eq!(pipeline.commands[1].name().map(|n| n.as_str()), Some("grep"));
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
        assert_eq!(pipeline.commands[1].name().map(|n| n.as_str()), Some("sort"));
        assert_eq!(pipeline.commands[2].name().map(|n| n.as_str()), Some("uniq"));
    }

    #[test]
    fn test_not_operator() {
        let vars = std::collections::HashMap::new();
        let entries = parse_pipeline("! ls", &vars).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].pipeline.negated);
        assert_eq!(entries[0].pipeline.commands[0].name().map(|n| n.as_str()), Some("ls"));

        let entries = parse_pipeline("!  ls -la", &vars).unwrap();
        assert_eq!(entries[0].pipeline.commands[0].name().map(|n| n.as_str()), Some("ls"));
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
        assert_eq!(entries[0].pipeline.commands[0].name().map(|n| n.as_str()), Some("ls"));
        assert_eq!(entries[0].pipeline.commands[1].name().map(|n| n.as_str()), Some("grep"));
        // Second entry is a simple command: echo done
        assert_eq!(entries[1].connector, Some(Connector::And));
        assert_eq!(entries[1].pipeline.commands.len(), 1);
        assert_eq!(entries[1].pipeline.commands[0].name().map(|n| n.as_str()), Some("echo"));
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
        assert_eq!(pipeline.commands[0].redirects()[0].kind, RedirectKind::StdinFrom);
        // Last command: sort > out.txt
        assert_eq!(pipeline.commands[1].name().map(|n| n.as_str()), Some("sort"));
        assert_eq!(pipeline.commands[1].redirects().len(), 1);
        assert_eq!(pipeline.commands[1].redirects()[0].kind, RedirectKind::StdoutOverwrite);
    }

    // ── integration: parse_pipeline with env expansion ────────────────────

    #[test]
    fn test_parse_line_expands_var_in_arg() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("CERF_DIR".to_string(), crate::engine::state::Variable::new_string("/tmp/test".to_string()));
        let cmd = parse_line_with_vars("cd $CERF_DIR", &vars).unwrap();
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("cd"));
        assert_eq!(arg_values(cmd.args()), vec!["/tmp/test"]);
    }

    #[test]
    fn test_parse_line_expands_var_in_quoted_arg() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("CERF_MSG".to_string(), crate::engine::state::Variable::new_string("hello world".to_string()));
        let cmd = parse_line_with_vars("echo \"$CERF_MSG\"", &vars).unwrap();
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("echo"));
        assert_eq!(arg_values(cmd.args()), vec!["hello world"]);
    }

    #[test]
    fn test_parse_line_expands_path_var() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("PATH".to_string(), crate::engine::state::Variable::new_string("some_path".to_string()));
        let expanded = expand_vars("echo $PATH", &vars);
        assert!(expanded.contains("some_path"), "expanded line should contain the PATH value");
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
        assert_eq!(cmd.assignments(), &[("MSG".to_string(), "hello world".to_string())]);
        assert_eq!(cmd.name().map(|n| n.as_str()), Some("echo"));
    }

    // ── control flow tests ──────────────────────────────────────────────────
    
    #[test]
    fn test_parse_if_simple() {
        let cmd = parse_line("if true { echo ok }").unwrap();
        match cmd {
            CommandNode::If { branches, else_branch } => {
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
            CommandNode::If { branches, else_branch } => {
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
}
