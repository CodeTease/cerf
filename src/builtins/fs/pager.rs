use std::fs;
use std::io::{self, BufRead, Write};
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;

pub const COMMAND_INFO_LESS: CommandInfo = CommandInfo {
    name: "fs.less",
    description: "Opposite of more.",
    usage: "fs.less [file]\n\nView the FILE, paging it.",
    run: runner,
};

pub const COMMAND_INFO_MORE: CommandInfo = CommandInfo {
    name: "fs.more",
    description: "Opposite of less.",
    usage: "fs.more [file]\n\nView the FILE, paging it.",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
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
