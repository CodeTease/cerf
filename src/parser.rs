use std::env;
use nom::{
    branch::alt,
    bytes::complete::is_not,
    character::complete::{char, multispace0, multispace1},
    sequence::{delimited, preceded},
    IResult,
    multi::many0,
    Parser,
};

// ── AST types ──────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParsedCommand {
    pub name: String,
    pub args: Vec<String>,
}

/// How consecutive commands are joined.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Connector {
    /// `;`  — always run the next command
    Semi,
    /// `&&` — run next only if previous succeeded (exit code 0)
    And,
    /// `||` — run next only if previous failed  (exit code ≠ 0)
    Or,
}

/// A single entry in a command list:
/// - `connector` is `None` for the very first command, `Some(…)` for every
///   subsequent command and describes the operator that precedes it.
/// - `command` is the command to execute.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CommandEntry {
    pub connector: Option<Connector>,
    pub command: ParsedCommand,
}

// ── Environment-variable expansion ────────────────────────────────────────

/// Expand environment variable references in `input` before parsing.
///
/// Substitution rules (mirrors POSIX sh behaviour):
/// - `$$`        → a literal `$`
/// - `$VAR`      → the value of the environment variable `VAR`
///                 (identifier chars: ASCII alphanumeric + `_`)
/// - `${VAR}`    → same, with brace delimiters
/// - Bare `$` with no following identifier or `{` → kept as-is
pub fn expand_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '$' {
            result.push(ch);
            continue;
        }

        match chars.peek() {
            // $$ → literal $
            Some('$') => {
                chars.next();
                result.push('$');
            }
            // ${VAR} style
            Some('{') => {
                chars.next(); // consume '{'
                let var_name: String = chars
                    .by_ref()
                    .take_while(|&c| c != '}')
                    .collect();
                let value = env::var(&var_name).unwrap_or_default();
                result.push_str(&value);
            }
            // $VAR style — identifier starts with alpha or '_'
            Some(&c) if c.is_ascii_alphabetic() || c == '_' => {
                let var_name: String = std::iter::once(chars.next().unwrap())
                    .chain(
                        std::iter::from_fn(|| {
                            chars.next_if(|c| c.is_ascii_alphanumeric() || *c == '_')
                        })
                    )
                    .collect();
                let value = env::var(&var_name).unwrap_or_default();
                result.push_str(&value);
            }
            // Bare $ with no following identifier → keep as-is
            _ => {
                result.push('$');
            }
        }
    }

    result
}

// ── Low-level nom parsers ──────────────────────────────────────────────────

fn parse_quoted_string(input: &str) -> IResult<&str, String> {
    let (input, content) = delimited(char('"'), is_not("\""), char('"')).parse(input)?;
    Ok((input, content.to_string()))
}

fn parse_unquoted_string(input: &str) -> IResult<&str, String> {
    // Stop at whitespace, quotes, AND the connector characters ; & |
    let (input, content) = is_not(" \t\r\n\";|&")(input)?;
    Ok((input, content.to_string()))
}

fn parse_arg(input: &str) -> IResult<&str, String> {
    alt((parse_quoted_string, parse_unquoted_string)).parse(input)
}

fn parse_single_command(input: &str) -> IResult<&str, ParsedCommand> {
    let (input, _) = multispace0(input)?;
    let (input, name) = parse_arg(input)?;

    // Arguments are separated by whitespace
    let (input, args) = many0(preceded(multispace1, parse_arg)).parse(input)?;
    let (input, _) = multispace0(input)?;

    Ok((input, ParsedCommand { name, args }))
}

/// Parse a connector operator: `&&`, `||`, or `;`.
fn parse_connector(input: &str) -> IResult<&str, Connector> {
    let (input, _) = multispace0(input)?;
    alt((
        // Two-character operators must come before single-character ones.
        nom::combinator::map(nom::bytes::complete::tag("&&"), |_| Connector::And),
        nom::combinator::map(nom::bytes::complete::tag("||"), |_| Connector::Or),
        nom::combinator::map(char(';'), |_| Connector::Semi),
    ))
    .parse(input)
}

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

    // ── expand_env_vars unit tests ────────────────────────────────────────

    #[test]
    fn test_expand_known_var() {
        unsafe { std::env::set_var("CERF_TEST_VAR", "hello"); }
        assert_eq!(expand_env_vars("$CERF_TEST_VAR"), "hello");
        assert_eq!(expand_env_vars("${CERF_TEST_VAR}"), "hello");
        unsafe { std::env::remove_var("CERF_TEST_VAR"); }
    }

    #[test]
    fn test_expand_missing_var_is_empty() {
        unsafe { std::env::remove_var("CERF_UNDEFINED_XYZ"); }
        assert_eq!(expand_env_vars("$CERF_UNDEFINED_XYZ"), "");
        assert_eq!(expand_env_vars("${CERF_UNDEFINED_XYZ}"), "");
    }

    #[test]
    fn test_expand_dollar_dollar_escape() {
        assert_eq!(expand_env_vars("$$"), "$");
        assert_eq!(expand_env_vars("$$$"), "$$");
        assert_eq!(expand_env_vars("cost: $$5"), "cost: $5");
    }

    #[test]
    fn test_expand_bare_dollar_kept() {
        assert_eq!(expand_env_vars("$ "), "$ ");
        assert_eq!(expand_env_vars("$"), "$");
    }

    #[test]
    fn test_expand_inline() {
        unsafe { std::env::set_var("CERF_GREET", "world"); }
        assert_eq!(expand_env_vars("hello $CERF_GREET!"), "hello world!");
        unsafe { std::env::remove_var("CERF_GREET"); }
    }

    #[test]
    fn test_expand_multiple_vars() {
        unsafe {
            std::env::set_var("CERF_A", "foo");
            std::env::set_var("CERF_B", "bar");
        }
        assert_eq!(expand_env_vars("$CERF_A/$CERF_B"), "foo/bar");
        unsafe {
            std::env::remove_var("CERF_A");
            std::env::remove_var("CERF_B");
        }
    }

    #[test]
    fn test_expand_no_dollar_unchanged() {
        assert_eq!(expand_env_vars("ls -la"), "ls -la");
        assert_eq!(expand_env_vars(""), "");
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
