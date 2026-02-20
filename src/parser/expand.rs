use std::env;

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

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
}
