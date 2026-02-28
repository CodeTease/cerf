use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum JobState {
    Running,
    Stopped,
    Done(i32),
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobState::Running => write!(f, "Running"),
            JobState::Stopped => write!(f, "Stopped"),
            JobState::Done(code) => write!(f, "Done({})", code),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub state: JobState,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: usize,
    pub pgid: u32,
    #[cfg(windows)]
    pub job_handle: isize,
    pub command: String,
    pub processes: Vec<ProcessInfo>,
    pub reported_done: bool,
}

impl Job {
    pub fn is_stopped(&self) -> bool {
        let all_suspended = self.processes.iter().all(|p| matches!(p.state, JobState::Stopped | JobState::Done(_)));
        let any_stopped = self.processes.iter().any(|p| matches!(p.state, JobState::Stopped));
        all_suspended && any_stopped
    }
    
    pub fn is_done(&self) -> bool {
        self.processes.iter().all(|p| matches!(p.state, JobState::Done(_)))
    }

    pub fn state(&self) -> JobState {
        if self.is_done() {
            // Find last process exit code
            let code = self.processes.last().map(|p| match p.state {
                JobState::Done(c) => c,
                _ => 0,
            }).unwrap_or(0);
            JobState::Done(code)
        } else if self.is_stopped() {
            JobState::Stopped
        } else {
            JobState::Running
        }
    }
}

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
    
    // Job control
    pub jobs: HashMap<usize, Job>,
    pub next_job_id: usize,
    pub current_job: Option<usize>,
    pub previous_job: Option<usize>,
    #[cfg(unix)]
    pub shell_pgid: Option<nix::unistd::Pid>,
    #[cfg(unix)]
    pub shell_term: Option<std::os::fd::RawFd>,
    #[cfg(windows)]
    pub iocp_handle: isize,
    #[cfg(windows)]
    pub iocp_receiver: Option<std::sync::mpsc::Receiver<crate::engine::job_control::IocpMessage>>,
}

impl ShellState {
    pub fn new() -> Self {
        let variables = init_env_vars();

        let mut state = ShellState {
            previous_dir: None,
            dir_stack: Vec::new(),
            aliases: init_default_aliases(),
            variables,
            set_options: HashSet::new(),
            history: Vec::new(),
            jobs: HashMap::new(),
            next_job_id: 1,
            current_job: None,
            previous_job: None,
            #[cfg(unix)]
            shell_pgid: None,
            #[cfg(unix)]
            shell_term: Some(nix::libc::STDIN_FILENO),
            #[cfg(windows)]
            iocp_handle: unsafe {
                windows_sys::Win32::System::IO::CreateIoCompletionPort(
                    windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE,
                    std::ptr::null_mut(),
                    0,
                    1,
                ) as isize
            },
            #[cfg(windows)]
            iocp_receiver: None,
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

/// Initialize the default aliases for backward POSIX compatibility with renamed `<type>.<action>` builtins.
fn init_default_aliases() -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    let mappings = [
        ("cd", "dir.cd"),
        ("pwd", "dir.pwd"),
        ("pushd", "dir.pushd"),
        ("popd", "dir.popd"),
        ("dirs", "dir.dirs"),
        ("jobs", "job.list"),
        ("fg", "job.fg"),
        ("bg", "job.bg"),
        ("wait", "job.wait"),
        ("kill", "job.kill"),
        ("tether", "job.tether"),
        ("untether", "job.untether"),
        ("export", "env.export"),
        ("unset", "env.unset"),
        ("set", "env.set"),
        ("source", "env.source"),
        (".", "env.source"),
        ("alias", "alias.set"),
        ("unalias", "alias.unset"),
        ("exit", "sys.exit"),
        ("clear", "sys.clear"),
        ("exec", "sys.exec"),
        ("history", "sys.history"),
        ("help", "sys.help"),
        ("type", "sys.type"),
        ("echo", "io.echo"),
        ("read", "io.read"),
        ("true", "test.true"),
        ("false", "test.false"),
        ("test", "test.check"),
        ("[", "test.check"),
    ];
    for (name, target) in mappings {
        aliases.insert(name.to_string(), target.to_string());
    }
    aliases
}
