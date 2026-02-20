// ── AST types ──────────────────────────────────────────────────────────────

/// A single shell argument with quoting metadata.
///
/// `quoted == true` means the argument was entirely wrapped in quotes
/// (`'…'` or `"…"`), and should **not** undergo glob expansion.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Arg {
    pub value: String,
    pub quoted: bool,
}

impl Arg {
    pub fn new(value: impl Into<String>, quoted: bool) -> Self {
        Self { value: value.into(), quoted }
    }

    pub fn plain(value: impl Into<String>) -> Self {
        Self::new(value, false)
    }
}

/// Helper: extract just the string values from a slice of `Arg`s.
pub fn arg_values(args: &[Arg]) -> Vec<&str> {
    args.iter().map(|a| a.value.as_str()).collect()
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParsedCommand {
    pub assignments: Vec<(String, String)>,
    pub name: Option<String>,
    pub args: Vec<Arg>,
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
    pub negated: bool,
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
