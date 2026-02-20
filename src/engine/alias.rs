use std::collections::HashMap;

use crate::parser::ParsedCommand;

/// Expand aliases on a `ParsedCommand` in-place (bash-style, one level).
///
/// If `cmd.name` matches an alias whose value is a single word, the name is
/// replaced and the replacement's trailing args are prepended to `cmd.args`.
/// If the alias value is a multi-word string, it is re-parsed: the first
/// token becomes the new command name and the rest are prepended to the
/// existing args.
///
/// Returns `true` when an expansion happened.
pub fn expand_alias(cmd: &mut ParsedCommand, aliases: &HashMap<String, String>) -> bool {
    let name = match cmd.name.as_ref() {
        Some(n) => n,
        None => return false,
    };
    if let Some(value) = aliases.get(name) {
        let value = value.clone();
        // Tokenise the alias value with a simple whitespace split that
        // respects single-quoted segments (good enough for shell aliases).
        let tokens = shell_split(&value);
        if tokens.is_empty() {
            return false;
        }
        // The first token is the new command name.
        cmd.name = Some(tokens[0].clone());
        // Any remaining alias tokens are prepended to the original args.
        let mut new_args = tokens[1..].to_vec();
        new_args.extend(cmd.args.drain(..));
        cmd.args = new_args;
        return true;
    }
    false
}

/// Very small shell-word splitter that honours `'…'` quoting.
/// Used only for parsing alias values.
fn shell_split(s: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_single => in_single = true,
            '\'' if in_single  => in_single = false,
            ' ' | '\t' if !in_single => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            other => current.push(other),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}
