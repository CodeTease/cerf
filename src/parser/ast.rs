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
        Self {
            value: value.into(),
            quoted,
        }
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
pub struct SimpleCommand {
    pub assignments: Vec<(String, String)>,
    pub name: Option<String>,
    pub args: Vec<Arg>,
    pub redirects: Vec<Redirect>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CommandNode {
    Simple(SimpleCommand),
    If {
        branches: Vec<(Vec<CommandEntry>, Vec<CommandEntry>)>,
        else_branch: Option<Vec<CommandEntry>>,
        redirects: Vec<Redirect>,
    },
    FuncDecl {
        name: String,
        body: Vec<CommandEntry>,
    },
    For {
        var: String,
        items: Vec<Arg>,
        body: Vec<CommandEntry>,
        redirects: Vec<Redirect>,
    },
    While {
        cond: Vec<CommandEntry>,
        body: Vec<CommandEntry>,
        redirects: Vec<Redirect>,
    },
    Loop {
        body: Vec<CommandEntry>,
        redirects: Vec<Redirect>,
    },
    Break,
    Continue,
}

impl CommandNode {
    pub fn name(&self) -> Option<&String> {
        match self {
            Self::Simple(s) => s.name.as_ref(),
            _ => None,
        }
    }

    pub fn name_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::Simple(s) => s.name.as_mut(),
            _ => None,
        }
    }

    pub fn args(&self) -> &[Arg] {
        match self {
            Self::Simple(s) => &s.args,
            _ => &[],
        }
    }

    pub fn args_mut(&mut self) -> Option<&mut Vec<Arg>> {
        match self {
            Self::Simple(s) => Some(&mut s.args),
            _ => None,
        }
    }

    pub fn redirects(&self) -> &[Redirect] {
        match self {
            Self::Simple(s) => &s.redirects,
            Self::If { redirects, .. } => redirects,
            Self::For { redirects, .. } => redirects,
            Self::While { redirects, .. } => redirects,
            Self::Loop { redirects, .. } => redirects,
            _ => &[],
        }
    }

    pub fn assignments(&self) -> &[(String, String)] {
        match self {
            Self::Simple(s) => &s.assignments,
            _ => &[],
        }
    }
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
    pub commands: Vec<CommandNode>, // length ≥ 1
    pub negated: bool,
    pub background: bool,
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
    /// `&` — run previous command in the background
    Amp,
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
