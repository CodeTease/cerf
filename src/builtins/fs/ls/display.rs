use crate::builtins::fs::ls::os::*;
use chrono::{DateTime, Local};
use std::fs;
use std::path::{Path, PathBuf};

pub fn format_size(size: u64, human_readable: bool) -> String {
    if !human_readable {
        return size.to_string();
    }
    let units = ["", "K", "M", "G", "T", "P", "E"];
    let mut s = size as f64;
    let mut i = 0;
    while s >= 1024.0 && i < units.len() - 1 {
        s /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{}", size)
    } else {
        format!("{:.1}{}", s, units[i])
    }
}

pub fn get_symbol(path: &Path, ft: fs::FileType, classify: bool) -> &'static str {
    if !classify {
        return "";
    }

    if ft.is_dir() {
        return "/";
    }

    if ft.is_symlink() {
        return "@";
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if ft.is_fifo() {
            return "|";
        }
        if ft.is_socket() {
            return "=";
        }
    }

    if let Ok(m) = fs::metadata(path)
        && is_executable(path, &m) {
            return "*";
        }

    ""
}

pub fn display_long<W: std::io::Write>(
    stdout: &mut W,
    entries: &[(PathBuf, String)],
    classify: bool,
    show_total: bool,
    human_readable: bool,
) {
    let mut infos = Vec::new();
    let mut total_blocks = 0;
    let mut max_nlink = 0;
    let mut max_size = 0;
    let mut max_user = 0;
    let mut max_group = 0;

    for (p, name) in entries {
        if let Ok(m) = fs::symlink_metadata(p) {
            let mode = get_mode_string(&m);
            let nlink = get_nlink(&m);
            let (user, group) = get_owner_group(&m);
            let size = m.len();
            let size_str = format_size(size, human_readable);
            let mtime = m
                .modified()
                .unwrap_or_else(|_| std::time::SystemTime::now());
            let dt: DateTime<Local> = mtime.into();
            let time_str = dt.format("%b %e %H:%M").to_string();
            let symbol = get_symbol(p, m.file_type(), classify);

            let mut display_name = name.clone();
            if m.file_type().is_symlink()
                && let Ok(target) = fs::read_link(p) {
                    display_name = format!("{} -> {}", display_name, target.display());
                }

            total_blocks += m.len().div_ceil(512);
            max_nlink = max_nlink.max(nlink);
            max_size = max_size.max(size_str.len());
            max_user = max_user.max(user.len());
            max_group = max_group.max(group.len());

            infos.push((
                mode,
                nlink,
                user,
                group,
                size_str,
                time_str,
                display_name,
                symbol,
            ));
        }
    }

    if show_total && !infos.is_empty() {
        if human_readable {
            let total_bytes = total_blocks * 512;
            let _ = writeln!(
                stdout,
                "total {:?}",
                format_size(total_bytes, true).replace("\"", "")
            );
        } else {
            let _ = writeln!(stdout, "total {}", total_blocks);
        }
    }

    let nlink_width = max_nlink.to_string().len();
    let size_width = max_size;

    for (mode, nlink, user, group, size, time, name, symbol) in infos {
        let _ = writeln!(
            stdout,
            "{} {:>nlink_width$} {:<max_user$} {:<max_group$} {:>size_width$} {} {}{}",
            mode, nlink, user, group, size, time, name, symbol
        );
    }
}
