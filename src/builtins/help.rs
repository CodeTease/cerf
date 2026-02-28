use std::process::Command;
use crate::engine::{ExecutionResult, ShellState};
use crate::builtins::registry::{CommandInfo, BUILTINS, find_command};
use crate::engine::path::find_executable;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "sys.help",
    description: "Display information about builtin commands.",
    usage: "sys.help [pattern ...]\n\nDisplay information about builtin commands. If PATTERN is specified,\ngives detailed help on all commands matching PATTERN, otherwise prints\na list of the builtins and their descriptions.",
    run: help_runner,
};

pub fn help_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    let mut exit_code = 0;

    if args.is_empty() {
        let mut help_text = String::new();
        help_text.push_str("cerf, version 0.1.0\n");
        help_text.push_str("These shell commands are defined internally. Type `help` to see this list.\n");
        help_text.push_str("Type `help name` to find out more about the function `name`.\n");
        help_text.push_str("Use `man -k` or `info` to find out more about commands not in this list.\n\n");

        let max_len = BUILTINS.iter().map(|b| b.name.len()).max().unwrap_or(0);

        for builtin in BUILTINS {
            help_text.push_str(&format!(" {:<width$}  {}\n", builtin.name, builtin.description, width = max_len));
        }
        print!("{}", help_text);
    } else {
        for arg in args {
            if let Some(cmd) = find_command(arg) {
                println!("{}: {}", cmd.name, cmd.description);
                println!("{}", cmd.usage);
            } else {
                // OS Fallback
                #[cfg(unix)]
                {
                    if find_executable("man").is_some() {
                        let mut command = Command::new("man");
                        command.arg(arg);
                        match command.status() {
                            Ok(status) if status.success() => {},
                            _ => {
                                // Fallback to `<cmd> --help` if `man` fails
                                try_help_flag(arg);
                            }
                        }
                    } else {
                        try_help_flag(arg);
                    }
                }
                
                #[cfg(windows)]
                {
                    try_help_flag(arg);
                }
                exit_code = 127; // Will be overwritten if successful, or kept if not a known builtin/command
            }
        }
    }
    (ExecutionResult::KeepRunning, exit_code)
}

fn try_help_flag(cmd_name: &str) {
    if find_executable(cmd_name).is_some() {
        let mut command = Command::new(cmd_name);
        command.arg("--help");
        let _ = command.status();
    } else {
        eprintln!("cerf: help: no help topics match `{}`", cmd_name);
    }
}
