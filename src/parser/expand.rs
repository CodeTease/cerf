
/// Expand variable references in `input` before parsing.
///
/// Substitution rules (mirrors POSIX sh behaviour):
/// - `$$`        → a literal `$`
/// - `$VAR`      → the value of the variable `VAR`
///                 (identifier chars: ASCII alphanumeric + `_`)
/// - `${VAR}`    → same, with brace delimiters
/// - Bare `$` with no following identifier or `{` → kept as-is
pub fn expand_vars(input: &str, shell_vars: &std::collections::HashMap<String, String>) -> String {
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
                let value = shell_vars.get(&var_name).cloned().unwrap_or_default();
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
                let value = shell_vars.get(&var_name).cloned().unwrap_or_default();
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
        let mut vars = std::collections::HashMap::new();
        vars.insert("CERF_TEST_VAR".to_string(), "hello".to_string());
        assert_eq!(expand_vars("$CERF_TEST_VAR", &vars), "hello");
        assert_eq!(expand_vars("${CERF_TEST_VAR}", &vars), "hello");
    }

    #[test]
    fn test_expand_missing_var_is_empty() {
        let vars = std::collections::HashMap::new();
        assert_eq!(expand_vars("$CERF_UNDEFINED_XYZ", &vars), "");
        assert_eq!(expand_vars("${CERF_UNDEFINED_XYZ}", &vars), "");
    }

    #[test]
    fn test_expand_dollar_dollar_escape() {
        let vars = std::collections::HashMap::new();
        assert_eq!(expand_vars("$$", &vars), "$");
        assert_eq!(expand_vars("$$$", &vars), "$$");
        assert_eq!(expand_vars("cost: $$5", &vars), "cost: $5");
    }

    #[test]
    fn test_expand_bare_dollar_kept() {
        let vars = std::collections::HashMap::new();
        assert_eq!(expand_vars("$ ", &vars), "$ ");
        assert_eq!(expand_vars("$", &vars), "$");
    }

    #[test]
    fn test_expand_inline() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("CERF_GREET".to_string(), "world".to_string());
        assert_eq!(expand_vars("hello $CERF_GREET!", &vars), "hello world!");
    }

    #[test]
    fn test_expand_multiple_vars() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("CERF_A".to_string(), "foo".to_string());
        vars.insert("CERF_B".to_string(), "bar".to_string());
        assert_eq!(expand_vars("$CERF_A/$CERF_B", &vars), "foo/bar");
    }

    #[test]
    fn test_expand_no_dollar_unchanged() {
        let vars = std::collections::HashMap::new();
        assert_eq!(expand_vars("ls -la", &vars), "ls -la");
        assert_eq!(expand_vars("", &vars), "");
    }
}
