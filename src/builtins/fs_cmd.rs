use std::fs;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;

pub const COMMAND_INFO_MKDIR: CommandInfo = CommandInfo {
    name: "fs.mkdir",
    description: "Create directories.",
    usage: "fs.mkdir [dir ...]\n\nCreate the DIRECTORY(ies), if they do not already exist.",
    run: mkdir_runner,
};

pub const COMMAND_INFO_RM: CommandInfo = CommandInfo {
    name: "fs.rm",
    description: "Remove files or directories.",
    usage: "fs.rm [-r] [file ...]\n\nRemove (unlink) the FILE(s).",
    run: rm_runner,
};

pub const COMMAND_INFO_TOUCH: CommandInfo = CommandInfo {
    name: "fs.touch",
    description: "Update the access and modification times of each FILE to the current time.",
    usage: "fs.touch [file ...]\n\nUpdate the access and modification times of each FILE to the current time. A FILE argument that does not exist is created empty.",
    run: touch_runner,
};

pub const COMMAND_INFO_CP: CommandInfo = CommandInfo {
    name: "fs.cp",
    description: "Copy files and directories.",
    usage: "fs.cp source destination\n\nCopy SOURCE to DEST.",
    run: cp_runner,
};

pub const COMMAND_INFO_MV: CommandInfo = CommandInfo {
    name: "fs.mv",
    description: "Move (rename) files.",
    usage: "fs.mv source destination\n\nRename SOURCE to DEST.",
    run: mv_runner,
};

pub const COMMAND_INFO_CAT: CommandInfo = CommandInfo {
    name: "fs.cat",
    description: "Concatenate and print files.",
    usage: "fs.cat [file ...]\n\nConcatenate FILE(s) to standard output.",
    run: cat_runner,
};

pub const COMMAND_INFO_LS: CommandInfo = CommandInfo {
    name: "fs.ls",
    description: "List directory contents.",
    usage: "fs.ls [path ...]\n\nList information about the FILEs (the current directory by default).",
    run: ls_runner,
};

pub const COMMAND_INFO_LESS: CommandInfo = CommandInfo {
    name: "fs.less",
    description: "Opposite of more.",
    usage: "fs.less [file]\n\nView the FILE, paging it.",
    run: less_runner,
};

pub const COMMAND_INFO_MORE: CommandInfo = CommandInfo {
    name: "fs.more",
    description: "Opposite of less.",
    usage: "fs.more [file]\n\nView the FILE, paging it.",
    run: less_runner,
};

pub const COMMAND_INFO_STAT: CommandInfo = CommandInfo {
    name: "fs.stat",
    description: "Display file or file system status.",
    usage: "fs.stat [file ...]\n\nDisplay file or file system status.",
    run: stat_runner,
};

pub const COMMAND_INFO_DU: CommandInfo = CommandInfo {
    name: "fs.du",
    description: "Estimate file space usage.",
    usage: "fs.du [path ...]\n\nSummarize disk usage of each FILE, recursively for directories.",
    run: du_runner,
};

pub const COMMAND_INFO_DF: CommandInfo = CommandInfo {
    name: "fs.df",
    description: "Report file system disk space usage.",
    usage: "fs.df\n\nShow information about the file system on which each FILE resides.",
    run: df_runner,
};

pub fn mkdir_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        eprintln!("cerf: fs.mkdir: missing operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let mut exit_code = 0;
    for arg in args {
        let path = expand_home(arg);
        if let Err(e) = fs::create_dir_all(&path) {
            eprintln!("cerf: fs.mkdir: cannot create directory '{}': {}", arg, e);
            exit_code = 1;
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
}

pub fn rm_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        eprintln!("cerf: fs.rm: missing operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let mut recursive = false;
    let mut files = Vec::new();

    for arg in args {
        if arg == "-r" || arg == "-R" {
            recursive = true;
        } else {
            files.push(arg);
        }
    }

    if files.is_empty() {
        eprintln!("cerf: fs.rm: missing operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let mut exit_code = 0;
    for arg in files {
        let path = expand_home(arg);
        if !path.exists() {
            eprintln!("cerf: fs.rm: cannot remove '{}': No such file or directory", arg);
            exit_code = 1;
            continue;
        }

        let res = if path.is_dir() {
            if recursive {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_dir(&path)
            }
        } else {
            fs::remove_file(&path)
        };

        if let Err(e) = res {
            eprintln!("cerf: fs.rm: cannot remove '{}': {}", arg, e);
            exit_code = 1;
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
}

pub fn touch_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        eprintln!("cerf: fs.touch: missing file operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let mut exit_code = 0;
    for arg in args {
        let path = expand_home(arg);
        let res = if path.exists() {
            // Update timestamp - for now just open and close it
            fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .map(|_| ())
        } else {
            fs::File::create(&path).map(|_| ())
        };

        if let Err(e) = res {
            eprintln!("cerf: fs.touch: cannot touch '{}': {}", arg, e);
            exit_code = 1;
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
}

pub fn cp_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.len() < 2 {
        eprintln!("cerf: fs.cp: missing file operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let src = expand_home(&args[0]);
    let dst = expand_home(&args[1]);

    if let Err(e) = fs::copy(&src, &dst) {
        eprintln!("cerf: fs.cp: cannot copy '{}' to '{}': {}", args[0], args[1], e);
        return (ExecutionResult::KeepRunning, 1);
    }

    (ExecutionResult::KeepRunning, 0)
}

pub fn mv_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.len() < 2 {
        eprintln!("cerf: fs.mv: missing file operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let src = expand_home(&args[0]);
    let dst = expand_home(&args[1]);

    if let Err(e) = fs::rename(&src, &dst) {
        eprintln!("cerf: fs.mv: cannot move '{}' to '{}': {}", args[0], args[1], e);
        return (ExecutionResult::KeepRunning, 1);
    }

    (ExecutionResult::KeepRunning, 0)
}

pub fn cat_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    use std::io::{self, Write};
    if args.is_empty() {
        return (ExecutionResult::KeepRunning, 0);
    }

    let mut exit_code = 0;
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for arg in args {
        let path = expand_home(arg);
        match fs::File::open(&path) {
            Ok(mut file) => {
                if let Err(e) = io::copy(&mut file, &mut handle) {
                    eprintln!("cerf: fs.cat: {}: {}", arg, e);
                    exit_code = 1;
                }
            }
            Err(e) => {
                eprintln!("cerf: fs.cat: {}: {}", arg, e);
                exit_code = 1;
            }
        }
    }
    let _ = handle.flush();
    (ExecutionResult::KeepRunning, exit_code)
}

pub fn ls_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    let targets = if args.is_empty() {
        vec![".".to_string()]
    } else {
        args.to_vec()
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
                            if e.path().is_dir() {
                                format!("{}/", name)
                            } else {
                                name
                            }
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
            println!("{}", target);
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
}

pub fn less_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    use std::io::{self, BufRead, Write};
    if args.is_empty() {
        eprintln!("cerf: fs.less: missing file operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let path = expand_home(&args[0]);
    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cerf: fs.less: {}: {}", args[0], e);
            return (ExecutionResult::KeepRunning, 1);
        }
    };

    let reader = io::BufReader::new(file);
    let mut lines = reader.lines();
    let screen_height = 24;

    loop {
        for _ in 0..screen_height {
            match lines.next() {
                Some(Ok(line)) => {
                    println!("{}", line);
                }
                Some(Err(e)) => {
                    eprintln!("cerf: fs.less: error reading file: {}", e);
                    return (ExecutionResult::KeepRunning, 1);
                }
                None => return (ExecutionResult::KeepRunning, 0),
            }
        }

        print!("--More--");
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        if input.trim().to_lowercase() == "q" {
            break;
        }
    }

    (ExecutionResult::KeepRunning, 0)
}

pub fn stat_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    use chrono::{DateTime, Local};

    if args.is_empty() {
        eprintln!("cerf: fs.stat: missing operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let mut exit_code = 0;
    for arg in args {
        let path = expand_home(arg);
        match fs::metadata(&path) {
            Ok(meta) => {
                println!("  File: {}", arg);
                let file_type = if meta.is_dir() {
                    "directory"
                } else if meta.is_file() {
                    "regular file"
                } else if meta.file_type().is_symlink() {
                    "symbolic link"
                } else {
                    "special file"
                };

                println!(
                    "  Size: {:<15} Blocks: {:<10} IO Block: {:<10} {}",
                    meta.len(),
                    (meta.len() + 511) / 512, // Rough estimation of 512-byte blocks
                    4096,                     // IO block size (typical)
                    file_type
                );

                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    let file_attributes = meta.file_attributes();
                    println!("Device: unknown         Inode: unknown         Links: unknown");
                    println!(
                        "Access: ({:o})  Uid: unknown   Gid: unknown",
                        file_attributes & 0o777
                    );
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    println!(
                        "Device: {:<15x} Inode: {:<15} Links: {}",
                        meta.dev(),
                        meta.ino(),
                        meta.nlink()
                    );
                    println!(
                        "Access: ({:04o})  Uid: ({:5})   Gid: ({:5})",
                        meta.mode() & 0o7777,
                        meta.uid(),
                        meta.gid()
                    );
                }

                #[cfg(not(any(windows, unix)))]
                {
                    println!("Device: unknown         Inode: unknown         Links: unknown");
                    println!("Access: unknown         Uid: unknown           Gid: unknown");
                }
                
                if let Ok(atime) = meta.accessed() {
                    let dt: DateTime<Local> = atime.into();
                    println!("Access: {}", dt.format("%Y-%m-%d %H:%M:%S.%f %z"));
                }
                if let Ok(mtime) = meta.modified() {
                    let dt: DateTime<Local> = mtime.into();
                    println!("Modify: {}", dt.format("%Y-%m-%d %H:%M:%S.%f %z"));
                }
                if let Ok(ctime) = meta.created() {
                    let dt: DateTime<Local> = ctime.into();
                    println!("Birth:  {}", dt.format("%Y-%m-%d %H:%M:%S.%f %z"));
                }
            }
            Err(e) => {
                eprintln!("cerf: fs.stat: cannot stat '{}': {}", arg, e);
                exit_code = 1;
            }
        }
    }

    (ExecutionResult::KeepRunning, exit_code)
}

pub fn du_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    use jwalk::WalkDir;

    let targets = if args.is_empty() {
        vec![".".to_string()]
    } else {
        args.to_vec()
    };

    let mut exit_code = 0;
    for target in targets {
        let path = expand_home(&target);
        if !path.exists() {
            eprintln!("cerf: fs.du: cannot access '{}': No such file or directory", target);
            exit_code = 1;
            continue;
        }

        let mut total_size = 0;
        for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(meta) = entry.metadata() {
                    let size = meta.len();
                    total_size += size;
                    // Streaming: print each file size
                    println!("{}\t{}", (size + 1023) / 1024, entry.path().display());
                }
            }
        }
        println!("{}\ttotal", (total_size + 1023) / 1024);
    }

    (ExecutionResult::KeepRunning, exit_code)
}

pub fn df_runner(_args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    use sysinfo::Disks;

    let disks = Disks::new_with_refreshed_list();
    println!("{:<20} {:<10} {:<10} {:<10} {:<5} {}", "Filesystem", "Size", "Used", "Avail", "Use%", "Mounted on");
    
    for disk in &disks {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total - available;
        let use_pct = if total > 0 { (used as f64 / total as f64 * 100.0) as u64 } else { 0 };
        
        println!("{:<20} {:<10} {:<10} {:<10} {:>3}% {}", 
            disk.name().to_string_lossy(),
            format_size(total),
            format_size(used),
            format_size(available),
            use_pct,
            disk.mount_point().display()
        );
    }

    (ExecutionResult::KeepRunning, 0)
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 * 1024 {
        format!("{:.1}T", bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}
