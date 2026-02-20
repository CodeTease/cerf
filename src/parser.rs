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

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedCommand {
    pub name: String,
    pub args: Vec<String>,
}

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

fn parse_quoted_string(input: &str) -> IResult<&str, String> {
    let (input, content) = delimited(char('"'), is_not("\""), char('"')).parse(input)?;
    Ok((input, content.to_string()))
}

fn parse_unquoted_string(input: &str) -> IResult<&str, String> {
    let (input, content) = is_not(" \t\r\n\"")(input)?;
    Ok((input, content.to_string()))
}

fn parse_arg(input: &str) -> IResult<&str, String> {
    alt((parse_quoted_string, parse_unquoted_string)).parse(input)
}

fn parse_command_internal(input: &str) -> IResult<&str, ParsedCommand> {
    let (input, _) = multispace0(input)?;
    let (input, name) = parse_arg(input)?;

    // Arguments are separated by whitespace
    let (input, args) = many0(preceded(multispace1, parse_arg)).parse(input)?;
    let (input, _) = multispace0(input)?;

    Ok((input, ParsedCommand { name, args }))
}

pub fn parse_line(input: &str) -> Option<ParsedCommand> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    // Expand environment variables before handing the line to nom.
    let expanded = expand_env_vars(input);

    match parse_command_internal(&expanded) {
        Ok((_, cmd)) => Some(cmd),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── existing parser tests ──────────────────────────────────────────────

    #[test]
    fn test_parse_simple() {
        let input = "ls -la";
        let cmd = parse_line(input).unwrap();
        assert_eq!(cmd.name, "ls");
        assert_eq!(cmd.args, vec!["-la"]);
    }

    #[test]
    fn test_parse_quoted() {
        let input = "echo \"hello world\"";
        let cmd = parse_line(input).unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["hello world"]);
    }

    #[test]
    fn test_parse_mixed() {
        let input = "cd \"My Documents\" backup";
        let cmd = parse_line(input).unwrap();
        assert_eq!(cmd.name, "cd");
        assert_eq!(cmd.args, vec!["My Documents", "backup"]);
    }

    #[test]
    fn test_extra_spaces() {
        let input = "  ls   -la  ";
        let cmd = parse_line(input).unwrap();
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

    // ── expand_env_vars unit tests ─────────────────────────────────────────

    #[test]
    fn test_expand_known_var() {
        unsafe { std::env::set_var("CERF_TEST_VAR", "hello"); }
        assert_eq!(expand_env_vars("$CERF_TEST_VAR"), "hello");
        assert_eq!(expand_env_vars("${CERF_TEST_VAR}"), "hello");
        unsafe { std::env::remove_var("CERF_TEST_VAR"); }
    }

    #[test]
    fn test_expand_missing_var_is_empty() {
        // Ensure the variable definitely does not exist.
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
        // A lone $ with no identifier after it should be left as-is.
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

    // ── integration: parse_line with env expansion ─────────────────────────

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
        // After expansion the quotes wrap the already-expanded string.
        assert_eq!(cmd.args, vec!["hello world"]);
        unsafe { std::env::remove_var("CERF_MSG"); }
    }

    #[test]
    fn test_parse_line_expands_path_var() {
        // Verify that $PATH is expanded by `expand_env_vars`.
        // We don't pipe through the full parser because PATH may contain spaces
        // (on Windows: "C:\Program Files\...") which nom would split as multiple
        // arguments – that's expected shell-splitting behaviour, not a bug.
        let path_val = std::env::var("PATH").unwrap_or_default();
        let expanded = expand_env_vars("echo $PATH");
        assert!(expanded.contains(&path_val), "expanded line should contain the PATH value");
    }
}

