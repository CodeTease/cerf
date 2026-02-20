use std::io::Write;

use crate::engine::ShellState;

/// Run the `history` builtin.
///
/// Prints all recorded history entries, numbered starting from 1.
pub fn run(state: &ShellState, stdout_redirect: Option<std::fs::File>) {
    let entries = &state.history;

    if let Some(mut f) = stdout_redirect {
        for (i, entry) in entries.iter().enumerate() {
            let _ = writeln!(f, "  {}  {}", i + 1, entry);
        }
    } else {
        for (i, entry) in entries.iter().enumerate() {
            println!("  {}  {}", i + 1, entry);
        }
    }
}
