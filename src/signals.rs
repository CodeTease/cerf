#[cfg(unix)]
use nix::sys::signal::{signal, SigHandler, Signal};

/// Initialize shell signal handlers
#[cfg(unix)]
pub fn init() {
    unsafe {
        // We ignore SIGINT, SIGQUIT, etc. so the shell doesn't exit when these signals are sent
        // to the process group (e.g. via Ctrl+C, Ctrl+\).
        // Note: Rustyline will override SIGINT during readline() calls, which is fine.
        signal(Signal::SIGINT, SigHandler::SigIgn).expect("Failed to ignore SIGINT");
        signal(Signal::SIGQUIT, SigHandler::SigIgn).expect("Failed to ignore SIGQUIT");
        signal(Signal::SIGTSTP, SigHandler::SigIgn).expect("Failed to ignore SIGTSTP");
        signal(Signal::SIGTTIN, SigHandler::SigIgn).expect("Failed to ignore SIGTTIN");
        signal(Signal::SIGTTOU, SigHandler::SigIgn).expect("Failed to ignore SIGTTOU");
    }
}

/// Restore default signal handlers (for child processes)
#[cfg(unix)]
pub fn restore_default() {
    unsafe {
        signal(Signal::SIGINT, SigHandler::SigDfl).expect("Failed to restore SIGINT");
        signal(Signal::SIGQUIT, SigHandler::SigDfl).expect("Failed to restore SIGQUIT");
        signal(Signal::SIGTSTP, SigHandler::SigDfl).expect("Failed to restore SIGTSTP");
        signal(Signal::SIGTTIN, SigHandler::SigDfl).expect("Failed to restore SIGTTIN");
        signal(Signal::SIGTTOU, SigHandler::SigDfl).expect("Failed to restore SIGTTOU");
    }
}

#[cfg(windows)]
pub fn init() {
    // Basic Windows console handling is handled by rustyline for Ctrl-C
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn restore_default() {
    // No-op on Windows
}
