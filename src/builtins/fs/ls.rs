use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.ls",
    description: "List directory contents.",
    usage: "fs.ls [flags] [path ...]\n\nList information about the FILEs (the current directory by default).\n\nFlags:\n  -a             do not ignore entries starting with .\n  -A             do not list implied . and ..\n  -F             append indicator (one of */=@|) to entries\n  -l             use a long listing format",
    run: runner,
};

pub fn runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    runner_inner(args, state, &mut std::io::stdout(), &mut std::io::stderr())
}

pub fn runner_inner<W: std::io::Write, E: std::io::Write>(
    args: &[String], 
    _state: &mut ShellState, 
    stdout: &mut W, 
    stderr: &mut E
) -> (ExecutionResult, i32) {
    let mut all = false;
    let mut almost_all = false;
    let mut classify = false;
    let mut long_format = false;
    let mut targets = Vec::new();

    for arg in args {
        if arg.starts_with('-') && arg.len() > 1 {
            for c in arg[1..].chars() {
                match c {
                    'a' => { all = true; almost_all = false; }
                    'A' => { almost_all = true; all = false; }
                    'F' => classify = true,
                    'l' => long_format = true,
                    _ => {}
                }
            }
        } else {
            targets.push(arg.clone());
        }
    }

    let targets = if targets.is_empty() {
        vec![".".to_string()]
    } else {
        targets
    };

    let mut exit_code = 0;
    let multiple = targets.len() > 1;

    for (i, target) in targets.iter().enumerate() {
        let path = expand_home(target);
        if !path.exists() {
            let _ = writeln!(stderr, "cerf: fs.ls: cannot access '{}': No such file or directory", target);
            exit_code = 1;
            continue;
        }

        if path.is_dir() {
            if multiple {
                if i > 0 { let _ = writeln!(stdout); }
                let _ = writeln!(stdout, "{}:", target);
            }
            match fs::read_dir(&path) {
                Ok(read_dir) => {
                    let mut dir_entries: Vec<(PathBuf, String)> = Vec::new();
                    if all {
                        dir_entries.push((path.join("."), ".".to_string()));
                        dir_entries.push((path.join(".."), "..".to_string()));
                    }
                    for entry in read_dir.filter_map(|e| e.ok()) {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if all || almost_all || !name.starts_with('.') {
                            dir_entries.push((entry.path(), name));
                        }
                    }
                    dir_entries.sort_by(|a, b| a.1.cmp(&b.1));

                    if long_format {
                        display_long(stdout, &dir_entries, classify, true);
                    } else {
                        let names: Vec<String> = dir_entries.into_iter().map(|(p, name)| {
                            let symbol = if let Ok(m) = fs::symlink_metadata(&p) {
                                get_symbol(&p, m.file_type(), classify)
                            } else {
                                ""
                            };
                            format!("{}{}", name, symbol)
                        }).collect();
                        let _ = writeln!(stdout, "{}", names.join("  "));
                    }
                }
                Err(e) => {
                    let _ = writeln!(stderr, "cerf: fs.ls: cannot open directory '{}': {}", target, e);
                    exit_code = 1;
                }
            }
        } else {
            if long_format {
                display_long(stdout, &[(path.clone(), target.clone())], classify, false);
            } else {
                let symbol = if let Ok(m) = fs::symlink_metadata(&path) {
                    get_symbol(&path, m.file_type(), classify)
                } else {
                    ""
                };
                let _ = writeln!(stdout, "{}{}", target, symbol);
            }
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
}

fn display_long<W: std::io::Write>(stdout: &mut W, entries: &[(PathBuf, String)], classify: bool, show_total: bool) {
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
            let mtime = m.modified().unwrap_or_else(|_| std::time::SystemTime::now());
            let dt: DateTime<Local> = mtime.into();
            let time_str = dt.format("%b %e %H:%M").to_string();
            let symbol = get_symbol(p, m.file_type(), classify);
            
            let mut display_name = name.clone();
            if m.file_type().is_symlink() {
                if let Ok(target) = fs::read_link(p) {
                    display_name = format!("{} -> {}", display_name, target.display());
                }
            }

            total_blocks += (m.len() + 511) / 512;
            max_nlink = max_nlink.max(nlink);
            max_size = max_size.max(size);
            max_user = max_user.max(user.len());
            max_group = max_group.max(group.len());

            infos.push((mode, nlink, user, group, size, time_str, display_name, symbol));
        }
    }

    if show_total && !infos.is_empty() {
        let _ = writeln!(stdout, "total {}", total_blocks);
    }

    let nlink_width = max_nlink.to_string().len();
    let size_width = max_size.to_string().len();

    for (mode, nlink, user, group, size, time, name, symbol) in infos {
        let _ = writeln!(
            stdout,
            "{} {:>nlink_width$} {:<max_user$} {:<max_group$} {:>size_width$} {} {}{}",
            mode, nlink, user, group, size, time, name, symbol
        );
    }
}

fn get_mode_string(meta: &fs::Metadata) -> String {
    let mut s = String::new();
    let ft = meta.file_type();
    
    if ft.is_dir() { s.push('d'); }
    else if ft.is_symlink() { s.push('l'); }
    else if ft.is_file() { s.push('-'); }
    else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            if ft.is_block_device() { s.push('b'); }
            else if ft.is_char_device() { s.push('c'); }
            else if ft.is_fifo() { s.push('p'); }
            else if ft.is_socket() { s.push('s'); }
            else { s.push('?'); }
        }
        #[cfg(not(unix))]
        { s.push('?'); }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();
        let chars = [
            (0o400, 'r'), (0o200, 'w'), (0o100, 'x'),
            (0o040, 'r'), (0o020, 'w'), (0o010, 'x'),
            (0o004, 'r'), (0o002, 'w'), (0o001, 'x'),
        ];
        for (m, c) in chars {
            if mode & m != 0 { s.push(c); } else { s.push('-'); }
        }
    }
    #[cfg(windows)]
    {
        let readonly = meta.permissions().readonly();
        s.push('r'); s.push(if readonly { '-' } else { 'w' }); s.push('-');
        s.push('r'); s.push(if readonly { '-' } else { 'w' }); s.push('-');
        s.push('r'); s.push(if readonly { '-' } else { 'w' }); s.push('-');
    }
    #[cfg(not(any(unix, windows)))]
    {
        s.push_str("---------");
    }
    s
}

fn get_nlink(meta: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.nlink()
    }
    #[cfg(windows)]
    {
        1 // number_of_links is currently unstable on Windows
    }
    #[cfg(not(any(unix, windows)))]
    { 1 }
}

#[allow(unused_variables)]
fn get_owner_group(meta: &fs::Metadata) -> (String, String) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        (meta.uid().to_string(), meta.gid().to_string())
    }
    #[cfg(not(unix))]
    {
        ("unknown".to_string(), "unknown".to_string())
    }
}

fn get_symbol(path: &Path, ft: fs::FileType, classify: bool) -> &'static str {
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

    if let Ok(m) = fs::metadata(path) {
        if is_executable(path, &m) {
            return "*";
        }
    }

    ""
}

#[allow(unused_variables)]
fn is_executable(path: &Path, m: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        m.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            matches!(ext.as_str(), "exe" | "bat" | "cmd" | "ps1" | "com")
        } else {
            false
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use tempfile::tempdir;

    #[test]
    fn test_ls_empty_dir() {
        let dir = tempdir().unwrap();
        let mut state = ShellState::new();
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());

        let args = vec![dir.path().to_string_lossy().into_owned()];
        let (res, code) = runner_inner(&args, &mut state, &mut stdout, &mut stderr);

        assert!(matches!(res, ExecutionResult::KeepRunning));
        assert_eq!(code, 0);
        assert_eq!(String::from_utf8(stdout.into_inner()).unwrap(), "\n");
    }

    #[test]
    fn test_ls_with_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), "hello").unwrap();
        fs::write(dir.path().join("file2.txt"), "world").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let mut state = ShellState::new();
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());

        let args = vec![dir.path().to_string_lossy().into_owned()];
        let (res, code) = runner_inner(&args, &mut state, &mut stdout, &mut stderr);

        assert_eq!(code, 0);
        let output = String::from_utf8(stdout.into_inner()).unwrap();
        assert!(output.contains("file1.txt"));
        assert!(output.contains("file2.txt"));
        assert!(output.contains("subdir"));
    }

    #[test]
    fn test_ls_hidden_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".hidden"), "hidden").unwrap();
        fs::write(dir.path().join("visible"), "visible").unwrap();

        let mut state = ShellState::new();
        
        // Default ls: no hidden files
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());
        let args = vec![dir.path().to_string_lossy().into_owned()];
        runner_inner(&args, &mut state, &mut stdout, &mut stderr);
        let output = String::from_utf8(stdout.into_inner()).unwrap();
        assert!(!output.contains(".hidden"));
        assert!(output.contains("visible"));

        // ls -a: all files including . and ..
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());
        let args = vec!["-a".to_string(), dir.path().to_string_lossy().into_owned()];
        runner_inner(&args, &mut state, &mut stdout, &mut stderr);
        let output = String::from_utf8(stdout.into_inner()).unwrap();
        assert!(output.contains(".hidden"));
        assert!(output.contains("visible"));
        assert!(output.contains(".  "));
        assert!(output.contains("..  "));

        // ls -A: almost all (no . and ..)
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());
        let args = vec!["-A".to_string(), dir.path().to_string_lossy().into_owned()];
        runner_inner(&args, &mut state, &mut stdout, &mut stderr);
        let output = String::from_utf8(stdout.into_inner()).unwrap();
        assert!(output.contains(".hidden"));
        assert!(output.contains("visible"));
        assert!(!output.contains(".  ")); 
        assert!(!output.contains("..  "));
    }

    #[test]
    fn test_ls_classify() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file.txt"), "hello").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let mut state = ShellState::new();
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());

        let args = vec!["-F".to_string(), dir.path().to_string_lossy().into_owned()];
        runner_inner(&args, &mut state, &mut stdout, &mut stderr);
        let output = String::from_utf8(stdout.into_inner()).unwrap();
        
        assert!(output.contains("subdir/"));
        assert!(output.contains("file.txt"));
        assert!(!output.contains("file.txt/"));
    }

    #[test]
    fn test_ls_single_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();

        let mut state = ShellState::new();
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());

        let args = vec![file_path.to_string_lossy().into_owned()];
        runner_inner(&args, &mut state, &mut stdout, &mut stderr);
        let output = String::from_utf8(stdout.into_inner()).unwrap();
        
        assert!(output.contains("test.txt"));
    }

    #[test]
    fn test_ls_non_existent() {
        let mut state = ShellState::new();
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());

        let args = vec!["/non/existent/path".to_string()];
        let (_, code) = runner_inner(&args, &mut state, &mut stdout, &mut stderr);
        
        assert_eq!(code, 1);
        let err_output = String::from_utf8(stderr.into_inner()).unwrap();
        assert!(err_output.contains("No such file or directory"));
    }

    #[test]
    fn test_ls_long_format() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("test.txt"), "hello").unwrap();

        let mut state = ShellState::new();
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());

        let args = vec!["-l".to_string(), dir.path().to_string_lossy().into_owned()];
        let (_, code) = runner_inner(&args, &mut state, &mut stdout, &mut stderr);
        
        assert_eq!(code, 0);
        let output = String::from_utf8(stdout.into_inner()).unwrap();
        
        assert!(output.contains("total"));
        assert!(output.contains("test.txt"));
        // Basic check for file info in long format
        assert!(output.contains("-")); // File type
        assert!(output.contains("5")); // File size (hello is 5 bytes)
    }

    #[test]
    fn test_ls_multiple_targets() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();
        fs::write(dir1.path().join("f1.txt"), "1").unwrap();
        fs::write(dir2.path().join("f2.txt"), "2").unwrap();

        let mut state = ShellState::new();
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());

        let args = vec![
            dir1.path().to_string_lossy().into_owned(),
            dir2.path().to_string_lossy().into_owned()
        ];
        runner_inner(&args, &mut state, &mut stdout, &mut stderr);
        let output = String::from_utf8(stdout.into_inner()).unwrap();
        
        // Multi-target output should contain directory names as headers
        assert!(output.contains(&format!("{}:", dir1.path().to_string_lossy())));
        assert!(output.contains(&format!("{}:", dir2.path().to_string_lossy())));
        assert!(output.contains("f1.txt"));
        assert!(output.contains("f2.txt"));
    }
}
