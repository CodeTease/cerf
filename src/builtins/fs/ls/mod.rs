mod display;
mod os;
mod parser;

use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;
use crate::engine::state::{ExecutionResult, ShellState};
use std::fs;
use std::path::PathBuf;

pub use display::{display_long, get_symbol};
pub use parser::LsArgs;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.ls",
    description: "List directory contents.",
    usage: "fs.ls [flags] [path ...]\n\nList information about the FILEs (the current directory by default).\n\nFlags:\n  -a             do not ignore entries starting with .\n  -A             do not list implied . and ..\n  -F             append indicator (one of */=@|) to entries\n  -l             use a long listing format\n  -h             print human readable sizes\n  -t             sort by modification time\n  -S             sort by file size, largest first\n  -r             reverse order while sorting\n  -1             list one file per line",
    run: runner,
};

pub fn runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    runner_inner(args, state, &mut std::io::stdout(), &mut std::io::stderr())
}

pub fn runner_inner<W: std::io::Write, E: std::io::Write>(
    args: &[String],
    _state: &mut ShellState,
    stdout: &mut W,
    stderr: &mut E,
) -> (ExecutionResult, i32) {
    let parsed_args = LsArgs::parse(args);

    let mut exit_code = 0;
    let multiple = parsed_args.targets.len() > 1;

    for (i, target) in parsed_args.targets.iter().enumerate() {
        let path = expand_home(target);
        if !path.exists() {
            let _ = writeln!(
                stderr,
                "cerf: fs.ls: cannot access '{}': No such file or directory",
                target
            );
            exit_code = 1;
            continue;
        }

        if path.is_dir() {
            if multiple {
                if i > 0 {
                    let _ = writeln!(stdout);
                }
                let _ = writeln!(stdout, "{}:", target);
            }
            match fs::read_dir(&path) {
                Ok(read_dir) => {
                    let mut dir_entries: Vec<(PathBuf, String)> = Vec::new();
                    if parsed_args.all {
                        dir_entries.push((path.join("."), ".".to_string()));
                        dir_entries.push((path.join(".."), "..".to_string()));
                    }
                    for entry in read_dir.filter_map(|e| e.ok()) {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if parsed_args.all || parsed_args.almost_all || !name.starts_with('.') {
                            dir_entries.push((entry.path(), name));
                        }
                    }
                    if parsed_args.sort_time {
                        dir_entries.sort_by(|a, b| {
                            let m_a = fs::symlink_metadata(&a.0).and_then(|m| m.modified());
                            let m_b = fs::symlink_metadata(&b.0).and_then(|m| m.modified());
                            match (m_a, m_b) {
                                (Ok(time_a), Ok(time_b)) => time_b.cmp(&time_a),
                                (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                                (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                                (Err(_), Err(_)) => a.1.cmp(&b.1),
                            }
                        });
                    } else if parsed_args.sort_size {
                        dir_entries.sort_by(|a, b| {
                            let s_a = fs::symlink_metadata(&a.0).map(|m| m.len()).unwrap_or(0);
                            let s_b = fs::symlink_metadata(&b.0).map(|m| m.len()).unwrap_or(0);
                            s_b.cmp(&s_a)
                        });
                    } else {
                        dir_entries.sort_by(|a, b| a.1.cmp(&b.1));
                    }

                    if parsed_args.reverse {
                        dir_entries.reverse();
                    }

                    if parsed_args.long_format {
                        display_long(
                            stdout,
                            &dir_entries,
                            parsed_args.classify,
                            true,
                            parsed_args.human_readable,
                        );
                    } else {
                        let names: Vec<String> = dir_entries
                            .into_iter()
                            .map(|(p, name)| {
                                let symbol = if let Ok(m) = fs::symlink_metadata(&p) {
                                    get_symbol(&p, m.file_type(), parsed_args.classify)
                                } else {
                                    ""
                                };
                                format!("{}{}", name, symbol)
                            })
                            .collect();
                        if parsed_args.single_column {
                            for name in names {
                                let _ = writeln!(stdout, "{}", name);
                            }
                        } else {
                            let _ = writeln!(stdout, "{}", names.join("  "));
                        }
                    }
                }
                Err(e) => {
                    let _ = writeln!(
                        stderr,
                        "cerf: fs.ls: cannot open directory '{}': {}",
                        target, e
                    );
                    exit_code = 1;
                }
            }
        } else {
            if parsed_args.long_format {
                display_long(
                    stdout,
                    &[(path.clone(), target.clone())],
                    parsed_args.classify,
                    false,
                    parsed_args.human_readable,
                );
            } else {
                let symbol = if let Ok(m) = fs::symlink_metadata(&path) {
                    get_symbol(&path, m.file_type(), parsed_args.classify)
                } else {
                    ""
                };
                let _ = writeln!(stdout, "{}{}", target, symbol);
            }
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
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
            dir2.path().to_string_lossy().into_owned(),
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
