use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;

pub struct ShellState {
    pub previous_dir: Option<PathBuf>,
    pub dir_stack: Vec<PathBuf>,
    /// All currently-defined aliases. Maps alias name → replacement string.
    pub aliases: HashMap<String, String>,
    /// All currently-defined shell variables.
    pub variables: HashMap<String, String>,
    /// Shell options enabled via `set -o` / `set -e` etc.
    pub set_options: HashSet<String>,
    /// Command history (persisted to `~/.cerf_history`).
    pub history: Vec<String>,
}

impl ShellState {
    pub fn new() -> Self {
        let variables = init_env_vars();

        let mut state = ShellState {
            previous_dir: None,
            dir_stack: Vec::new(),
            aliases: HashMap::new(),
            variables,
            set_options: HashSet::new(),
            history: Vec::new(),
        };
        state.load_history();
        state
    }

    /// Load history entries from `~/.cerf_history` (if it exists).
    pub fn load_history(&mut self) {
        if let Some(path) = Self::history_path() {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    self.history = contents
                        .lines()
                        .filter(|l| !l.is_empty())
                        .map(|l| l.to_string())
                        .collect();
                }
            }
        }
    }

    /// Append a single line to the in-memory history and to `~/.cerf_history`.
    pub fn add_history(&mut self, line: &str) {
        self.history.push(line.to_string());
        if let Some(path) = Self::history_path() {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(f, "{}", line);
            }
        }
    }

    /// Return the path to `~/.cerf_history`.
    fn history_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".cerf_history"))
    }
}

pub enum ExecutionResult {
    KeepRunning,
    Exit,
}

/// Initialize shell variables from the OS environment and set defaults for missing ones.
fn init_env_vars() -> HashMap<String, String> {
    let mut vars: HashMap<String, String> = std::env::vars().collect();

    // 1. Ensure HOME is set
    if !vars.contains_key("HOME") {
        #[cfg(windows)]
        {
            if let Ok(profile) = std::env::var("USERPROFILE") {
                vars.insert("HOME".to_string(), profile);
            }
        }
        #[cfg(not(windows))]
        {
            if let Ok(home) = std::env::var("HOME") {
                vars.insert("HOME".to_string(), home);
            } else if let Some(home_path) = dirs::home_dir() {
                vars.insert("HOME".to_string(), home_path.to_string_lossy().to_string());
            }
        }
    }

    // 2. Ensure PATH is set
    if !vars.contains_key("PATH") {
        #[cfg(windows)]
        vars.insert("PATH".to_string(), "C:\\Windows\\system32;C:\\Windows".to_string());
        #[cfg(not(windows))]
        vars.insert("PATH".to_string(), "/usr/local/bin:/usr/bin:/bin".to_string());
    }

    // 3. Ensure EDITOR is set
    if !vars.contains_key("EDITOR") {
        #[cfg(windows)]
        vars.insert("EDITOR".to_string(), "notepad".to_string());
        #[cfg(not(windows))]
        vars.insert("EDITOR".to_string(), "vi".to_string());
    }

    // Sync environment variables that we just added defaults for
    for (key, val) in &vars {
        if std::env::var(key).is_err() {
            unsafe { std::env::set_var(key, val); }
        }
    }

    vars
}
