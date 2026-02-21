mod parser;
mod engine;
mod builtins;
mod signals;

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::env;
use engine::ShellState;

fn get_prompt() -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let home = dirs::home_dir();
    
    let path_str = if let Some(home) = home {
        if cwd.starts_with(&home) {
            let relative = cwd.strip_prefix(&home).unwrap();
            if relative.as_os_str().is_empty() {
                "~".to_string()
            } else {
                let sep = std::path::MAIN_SEPARATOR;
                format!("~{}{}", sep, relative.display())
            }
        } else {
            cwd.display().to_string()
        }
    } else {
        cwd.display().to_string()
    };
    
    format!("cf {} > ", path_str)
}

fn main() -> rustyline::Result<()> {
    signals::init();
    
    // Initialize job control 
    #[cfg(unix)]
    {
        let pid = nix::unistd::getpid();
        let _ = nix::unistd::setpgid(pid, pid);
        let _ = nix::unistd::tcsetpgrp(unsafe { std::os::fd::BorrowedFd::borrow_raw(nix::libc::STDIN_FILENO) }, pid);
    }
    
    let mut state = ShellState::new();

    #[cfg(unix)]
    {
        state.shell_pgid = Some(nix::unistd::Pid::from_raw(nix::unistd::getpid().as_raw()));
    }

    let args: Vec<String> = env::args().collect();
    if args.len() >= 3 && args[1] == "-c" {
        let input = &args[2];
        if let Some(entries) = parser::parse_pipeline(input, &state.variables) {
            engine::execute_list(entries, &mut state);
        }
        return Ok(());
    }

    // Source the user profile (~/.cerfrc) for interactive sessions.
    source_profile(&mut state);

    let mut rl = DefaultEditor::new()?;

    loop {
        // Poll for any background jobs that have finished
        #[cfg(unix)]
        engine::job_control::update_jobs(&mut state);
        
        // Ensure shell owns the terminal
        #[cfg(unix)]
        engine::job_control::restore_terminal(&state);

        let prompt = get_prompt();
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(input);
                state.add_history(input);

                if let Some(entries) = parser::parse_pipeline(input, &state.variables) {
                    match engine::execute_list(entries, &mut state) {
                        engine::ExecutionResult::Exit => break,
                        engine::ExecutionResult::KeepRunning => {},
                    }
                }
            },
            Err(ReadlineError::Interrupted) => {
                continue;
            },
            Err(ReadlineError::Eof) => {
                println!("exit");
                break;
            },
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

/// Source `~/.cerfrc` if it exists.
fn source_profile(state: &mut ShellState) {
    if let Some(home) = dirs::home_dir() {
        let rc_path = home.join(".cerfrc");
        if rc_path.exists() {
            let path_str = rc_path.to_string_lossy().to_string();
            builtins::source::run(&[path_str], state);
        }
    }
}
