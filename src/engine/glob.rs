use crate::parser::Arg;

/// Expand glob patterns in a list of arguments.
///
/// For each argument:
/// - If `quoted == true` → push the raw value unchanged (quoted args are never
///   glob-expanded, matching POSIX shell behaviour).
/// - If the value contains glob meta-characters (`*`, `?`, `[`) → call
///   `glob::glob()` on it.
///   - If matches are found → push all matches (sorted lexicographically).
///   - If no matches → push the original pattern unchanged (bash default).
/// - Otherwise → push the raw value unchanged.
pub fn expand_globs(args: &[Arg]) -> Vec<String> {
    let mut expanded: Vec<String> = Vec::new();

    for arg in args {
        if arg.quoted || !contains_glob_chars(&arg.value) {
            expanded.push(arg.value.clone());
            continue;
        }

        // Attempt glob expansion.
        match glob::glob(&arg.value) {
            Ok(paths) => {
                let mut matches: Vec<String> = paths
                    .filter_map(|entry| entry.ok())
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();

                if matches.is_empty() {
                    // No matches — keep the original pattern (bash behaviour).
                    expanded.push(arg.value.clone());
                } else {
                    matches.sort();
                    expanded.append(&mut matches);
                }
            }
            Err(_) => {
                // Invalid pattern — keep as-is.
                expanded.push(arg.value.clone());
            }
        }
    }

    expanded
}

/// Does `s` contain any glob meta-characters?
fn contains_glob_chars(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_glob_chars_passes_through() {
        let args = vec![Arg::plain("hello"), Arg::plain("-la")];
        let result = expand_globs(&args);
        assert_eq!(result, vec!["hello", "-la"]);
    }

    #[test]
    fn test_quoted_arg_not_expanded() {
        let args = vec![Arg::new("*.rs", true)];
        let result = expand_globs(&args);
        assert_eq!(result, vec!["*.rs"]);
    }

    #[test]
    fn test_glob_no_matches_kept_as_is() {
        let args = vec![Arg::plain("*.this_extension_should_not_exist_xyzzy")];
        let result = expand_globs(&args);
        assert_eq!(result, vec!["*.this_extension_should_not_exist_xyzzy"]);
    }
}
