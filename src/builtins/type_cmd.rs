use std::env;

/// Return the type description for a single command name.
pub fn type_of(cmd: &str) -> String {
    let builtins = ["cd", "pwd", "exit", "clear", "echo", "type"];

    if builtins.contains(&cmd) {
        return format!("{} is a shell builtin", cmd);
    }

    // Check PATH
    if let Ok(paths) = env::var("PATH") {
        for path in env::split_paths(&paths) {
            let mut exe_path = path.join(cmd);

            #[cfg(windows)]
            {
                let extensions = ["", ".exe", ".cmd", ".bat"];
                for ext in extensions {
                    exe_path.set_extension(ext);
                    if exe_path.is_file() {
                        return format!("{} is {}", cmd, exe_path.display());
                    }
                }
            }

            #[cfg(unix)]
            {
                if exe_path.is_file() {
                    return format!("{} is {}", cmd, exe_path.display());
                }
            }
        }
    }

    format!("cerf: type: {}: not found", cmd)
}

pub fn run(args: &[String]) {
    if args.is_empty() {
        return;
    }

    for cmd in args {
        let desc = type_of(cmd);
        if desc.starts_with("cerf: type:") {
            eprintln!("{}", desc);
        } else {
            println!("{}", desc);
        }
    }
}
