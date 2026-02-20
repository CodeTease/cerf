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
