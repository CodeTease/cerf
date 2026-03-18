use std::fs;
use std::path::Path;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.ls",
    description: "List directory contents.",
    usage: "fs.ls [flags] [path ...]\n\nList information about the FILEs (the current directory by default).\n\nFlags:\n  -F             append indicator (one of */=@|) to entries",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    let mut classify = false;
    let mut targets = Vec::new();

    for arg in args {
        if arg.starts_with('-') && arg.len() > 1 {
            if arg.contains('F') {
                classify = true;
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
            eprintln!("cerf: fs.ls: cannot access '{}': No such file or directory", target);
            exit_code = 1;
            continue;
        }

        if path.is_dir() {
            if multiple {
                if i > 0 { println!(); }
                println!("{}:", target);
            }
            match fs::read_dir(&path) {
                Ok(entries) => {
                    let mut names: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| {
                            let name = e.file_name().to_string_lossy().into_owned();
                            let symbol = if let Ok(ft) = e.file_type() {
                                get_symbol(&e.path(), ft, classify)
                            } else {
                                ""
                            };
                            format!("{}{}", name, symbol)
                        })
                        .collect();
                    names.sort();
                    println!("{}", names.join("  "));
                }
                Err(e) => {
                    eprintln!("cerf: fs.ls: cannot open directory '{}': {}", target, e);
                    exit_code = 1;
                }
            }
        } else {
            let symbol = if let Ok(m) = fs::symlink_metadata(&path) {
                get_symbol(&path, m.file_type(), classify)
            } else {
                ""
            };
            println!("{}{}", target, symbol);
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
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
