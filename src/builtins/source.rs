use std::cell::Cell;
use std::fs;
use std::path::PathBuf;

use crate::engine::expand_home;
use crate::engine::{execute_list, ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::parser;

pub const COMMAND_INFO_SOURCE: CommandInfo = CommandInfo {
    name: "env.source",
    description: "Execute commands from a file in the current shell.",
    usage: "env.source filename [arguments]\n\nExecute commands from a file in the current shell.",
    run,
};


/// Maximum nesting depth for `source` / `.` to prevent infinite loops
/// (e.g. `~/.cerfrc` sourcing itself).
const MAX_SOURCE_DEPTH: u32 = 64;

thread_local! {
    static SOURCE_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Run the `source` / `.` builtin.
///
/// Reads the given file line-by-line, parsing and executing each line in the
/// current shell context (variables, aliases, etc. persist after the file
/// finishes).
pub fn run(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        eprintln!("cerf: source: filename argument required");
        return (ExecutionResult::KeepRunning, 1);
    }

    let path = resolve_path(&args[0]);

    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cerf: source: {}: {}", path.display(), e);
            return (ExecutionResult::KeepRunning, 1);
        }
    };

    // Guard against infinite recursion.
    let depth = SOURCE_DEPTH.with(|d| d.get());
    if depth >= MAX_SOURCE_DEPTH {
        eprintln!(
            "cerf: source: maximum recursion depth ({}) exceeded while sourcing '{}'",
            MAX_SOURCE_DEPTH,
            path.display()
        );
        return (ExecutionResult::KeepRunning, 1);
    }

    SOURCE_DEPTH.with(|d| d.set(depth + 1));

    let mut last_result = ExecutionResult::KeepRunning;
    let mut last_code: i32 = 0;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(entries) = parser::parse_pipeline(trimmed, &state.variables) {
            match execute_list(entries, state) {
                ExecutionResult::Exit => {
                    last_result = ExecutionResult::Exit;
                    break;
                }
                ExecutionResult::KeepRunning => {
                    last_code = 0;
                }
            }
        }
    }

    SOURCE_DEPTH.with(|d| d.set(depth));

    (last_result, last_code)
}

/// Resolve `~` at the start of the path and return an absolute `PathBuf`.
fn resolve_path(raw: &str) -> PathBuf {
    expand_home(raw)
}
