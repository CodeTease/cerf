// ── AST types ──────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParsedCommand {
    pub name: String,
    pub args: Vec<String>,
    pub redirects: Vec<Redirect>,
}

/// I/O redirection attached to a single command.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RedirectKind {
    /// `>  file` — truncate-write stdout to file
    StdoutOverwrite,
    /// `>> file` — append stdout to file
    StdoutAppend,
    /// `<  file` — read stdin from file
    StdinFrom,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Redirect {
    pub kind: RedirectKind,
    pub file: String,
}

/// A pipeline is one or more commands connected by `|`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Pipeline {
    pub commands: Vec<ParsedCommand>, // length ≥ 1
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
/// - `pipeline` is the pipeline to execute.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CommandEntry {
    pub connector: Option<Connector>,
    pub pipeline: Pipeline,
}
