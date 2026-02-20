use std::collections::HashMap;
use std::path::PathBuf;

pub struct ShellState {
    pub previous_dir: Option<PathBuf>,
    /// All currently-defined aliases. Maps alias name → replacement string.
    pub aliases: HashMap<String, String>,
    /// All currently-defined shell variables.
    pub variables: HashMap<String, String>,
}

impl ShellState {
    pub fn new() -> Self {
        let variables = init_env_vars();

        ShellState {
            previous_dir: None,
            aliases: HashMap::new(),
            variables,
        }
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
