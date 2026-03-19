use std::collections::HashMap;

use crate::parser::{Arg, CommandNode};

/// Expand aliases on a `ParsedCommand` in-place (bash-style, one level).
///
/// If `cmd.name` matches an alias whose value is a single word, the name is
/// replaced and the replacement's trailing args are prepended to `cmd.args`.
/// If the alias value is a multi-word string, it is re-parsed: the first
/// token becomes the new command name and the rest are prepended to the
/// existing args.
///
/// Returns `true` when an expansion happened.
pub fn expand_alias(cmd: &mut CommandNode, aliases: &HashMap<String, String>) -> bool {
    // Only simple commands are aliased
    let simple = match cmd {
        CommandNode::Simple(s) => s,
        _ => return false,
    };

    let name = match simple.name.as_ref() {
        Some(n) => n,
        None => return false,
    };
    if let Some(value) = aliases.get(name) {
        let value = value.clone();
        let tokens = shell_split(&value);
        if tokens.is_empty() {
            return false;
        }
        simple.name = Some(tokens[0].clone());
        let mut new_args: Vec<Arg> = tokens[1..].iter().map(Arg::plain).collect();
        new_args.append(&mut simple.args);
        simple.args = new_args;
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
    let chars = s.chars().peekable();

    for ch in chars {
        match ch {
            '\'' if !in_single => in_single = true,
            '\'' if in_single => in_single = false,
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
