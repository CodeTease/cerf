use std::env;

pub fn run(args: &[String]) {
    if args.is_empty() {
        return;
    }

    let builtins = ["cd", "pwd", "exit", "clear", "echo", "type"];

    for cmd in args {
        if builtins.contains(&cmd.as_str()) {
            println!("{} is a shell builtin", cmd);
        } else {
            // Check PATH
            let mut found = false;
            if let Ok(paths) = env::var("PATH") {
                for path in env::split_paths(&paths) {
                    let mut exe_path = path.join(cmd);
                    
                    #[cfg(windows)]
                    {
                        let extensions = ["", ".exe", ".cmd", ".bat"];
                        for ext in extensions {
                            exe_path.set_extension(ext);
                            if exe_path.is_file() {
                                println!("{} is {}", cmd, exe_path.display());
                                found = true;
                                break;
                            }
                        }
                    }
                    
                    #[cfg(unix)]
                    {
                        if exe_path.is_file() {
                            println!("{} is {}", cmd, exe_path.display());
                            found = true;
                        }
                    }

                    if found {
                        break;
                    }
                }
            }
            if !found {
                eprintln!("cerf: type: {}: not found", cmd);
            }
        }
    }
}
