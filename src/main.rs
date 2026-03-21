mod builtins;
mod engine;
mod parser;
mod prompt;
mod signals;

use engine::ShellState;
use rustyline::DefaultEditor;
use rustyline::ExternalPrinter;
use rustyline::error::ReadlineError;
use std::env;
use std::sync::atomic::AtomicUsize;

pub static FG_JOB: AtomicUsize = AtomicUsize::new(0);


fn main() -> rustyline::Result<()> {
    signals::init();

    // Initialize job control
    #[cfg(unix)]
    {
        let pid = nix::unistd::getpid();
        let _ = nix::unistd::setpgid(pid, pid);
        let _ = nix::unistd::tcsetpgrp(
            unsafe { std::os::fd::BorrowedFd::borrow_raw(nix::libc::STDIN_FILENO) },
            pid,
        );
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
            let _ = engine::execute_list(entries, &mut state);
        }
        return Ok(());
    }
    else if args.len() == 2 && args[1] == "--version" {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Source the user profile (~/.cerfrc) for interactive sessions.
    source_profile(&mut state);

    let config = rustyline::Config::builder().bracketed_paste(true).build();
    let mut rl = DefaultEditor::with_config(config)?;
    let mut printer_opt = rl.create_external_printer().ok();

    #[cfg(windows)]
    {
        let (tx, rx) = std::sync::mpsc::channel::<engine::job_control::IocpMessage>();
        state.iocp_receiver = Some(rx);
        let handle = state.iocp_handle;

        std::thread::spawn(move || {
            use windows_sys::Win32::System::IO::GetQueuedCompletionStatus;
            use windows_sys::Win32::System::SystemServices::JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO;

            loop {
                let mut num_bytes = 0;
                let mut comp_key = 0;
                let mut overlapped = std::ptr::null_mut();

                let res = unsafe {
                    GetQueuedCompletionStatus(
                        handle as _,
                        &mut num_bytes,
                        &mut comp_key,
                        &mut overlapped,
                        windows_sys::Win32::System::Threading::INFINITE,
                    )
                };

                if res != 0 {
                    let msg = num_bytes;
                    let event_job_id = comp_key;
                    let pid = overlapped as usize as u32;

                    let is_active_zero = msg == JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO;

                    if is_active_zero {
                        let fg = FG_JOB.load(std::sync::atomic::Ordering::Relaxed);
                        if fg != event_job_id {
                            if let Some(ref mut p) = printer_opt {
                                let _ = p.print(format!("\n[{}] Done", event_job_id));
                            } else {
                                // Fallback
                                eprintln!("\n[{}] Done", event_job_id);
                            }
                        }
                    }

                    let _ = tx.send(engine::job_control::IocpMessage {
                        msg,
                        job_id: event_job_id,
                        pid,
                    });
                }
            }
        });
    }

    let mut input_buffer = String::new();
    loop {
        // Poll for any background jobs that have finished
        #[cfg(unix)]
        engine::job_control::update_jobs(&mut state);

        // Ensure shell owns the terminal
        #[cfg(unix)]
        engine::job_control::restore_terminal(&state);

        let prompt = if input_buffer.is_empty() {
            prompt::build_prompt(&mut state)
        } else {
            state
                .get_var_string("PS2")
                .unwrap_or_else(|| "> ".to_string())
        };

        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                let trimmed = line.trim_end();

                // Explicit comma continuation (kept for backwards compat).
                if trimmed.ends_with(',') {
                    input_buffer.push_str(&trimmed[..trimmed.len() - 1]);
                    continue;
                }

                if input_buffer.is_empty() {
                    input_buffer.push_str(&line);
                } else {
                    // Separate accumulated lines with a real newline so
                    // the parser sees them as distinct commands.
                    input_buffer.push('\n');
                    input_buffer.push_str(&line);
                }

                // If the input looks incomplete (unbalanced braces, trailing
                // operator, etc.), keep reading the next line.
                if parser::is_incomplete(&input_buffer) {
                    continue;
                }

                let input = input_buffer.trim().to_string();
                input_buffer.clear();

                if input.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(&input);
                state.add_history(&input);

                if let Some(entries) = parser::parse_pipeline(&input, &state.variables)
                    && let (engine::ExecutionResult::Exit, _) = engine::execute_list(entries, &mut state) { break }
            }
            Err(ReadlineError::Interrupted) => {
                input_buffer.clear();
                continue;
            }
            Err(ReadlineError::Eof) => {
                if !input_buffer.is_empty() {
                    let input = input_buffer.trim().to_string();
                    input_buffer.clear();
                    if let Some(entries) = parser::parse_pipeline(&input, &state.variables) {
                        let _ = engine::execute_list(entries, &mut state);
                    }
                }
                println!("exit");
                break;
            }
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
