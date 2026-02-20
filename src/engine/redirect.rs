use std::fs::{File, OpenOptions};

use crate::parser::{Redirect, RedirectKind};
use super::path::expand_home;

/// Open a file for an output redirect (stdout).
pub fn open_stdout_redirect(redirect: &Redirect) -> Result<File, String> {
    match redirect.kind {
        RedirectKind::StdoutOverwrite => {
            let path = expand_home(&redirect.file);
            File::create(&path)
                .map_err(|e| format!("cerf: {}: {}", path.display(), e))
        }
        RedirectKind::StdoutAppend => {
            let path = expand_home(&redirect.file);
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| format!("cerf: {}: {}", path.display(), e))
        }
        _ => Err("not a stdout redirect".to_string()),
    }
}

/// Open a file for an input redirect (stdin).
pub fn open_stdin_redirect(redirect: &Redirect) -> Result<File, String> {
    let path = expand_home(&redirect.file);
    File::open(&path)
        .map_err(|e| format!("cerf: {}: {}", path.display(), e))
}

/// Find the first stdin and last stdout redirect from a list.
pub fn resolve_redirects(redirects: &[Redirect]) -> (Option<&Redirect>, Option<&Redirect>) {
    let stdin_redir = redirects.iter().rfind(|r| r.kind == RedirectKind::StdinFrom);
    let stdout_redir = redirects.iter().rfind(|r| {
        r.kind == RedirectKind::StdoutOverwrite || r.kind == RedirectKind::StdoutAppend
    });
    (stdin_redir, stdout_redir)
}
