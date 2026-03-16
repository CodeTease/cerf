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
