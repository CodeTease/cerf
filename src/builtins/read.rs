use std::io::{self, BufRead, Write};
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "read",
    description: "Read a line from the standard input and split it into fields.",
    usage: "read [-prs] [-a array] [-d delim] [-i text] [-n nchars] [-N nchars] [-t timeout] [-u fd] [name ...]\n\nRead a line from the standard input and split it into fields.",
    run: read_runner,
};

pub fn read_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    match run(args, state) {
        Ok(()) => (ExecutionResult::KeepRunning, 0),
        Err(e) => {
            if !e.is_empty() {
                eprintln!("cerf: read: {}", e);
            }
            (ExecutionResult::KeepRunning, 1)
        }
    }
}

pub fn run(args: &[String], state: &mut ShellState) -> Result<(), String> {
    let mut raw_mode = false;
    let mut prompt = None;
    let mut var_names = Vec::new();

    let mut i = 0;
    while i < args.len() {
        if args[i] == "-r" {
            raw_mode = true;
        } else if args[i] == "-s" {
            // Silently ignoring -s for now to keep basic scripts working
        } else if args[i] == "-p" {
            if i + 1 < args.len() {
                prompt = Some(args[i + 1].clone());
                i += 1;
            } else {
                return Err("read: -p requires an argument".to_string());
            }
        } else if args[i].starts_with('-') {
            return Err(format!("read: invalid option {}", args[i]));
        } else {
            var_names.extend_from_slice(&args[i..]);
            break;
        }
        i += 1;
    }

    if var_names.is_empty() {
        var_names.push("REPLY".to_string());
    }

    if let Some(p) = prompt {
        print!("{}", p);
        let _ = io::stdout().flush();
    }

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut line = String::new();

    match handle.read_line(&mut line) {
        Ok(0) => {
            return Err(String::new()); // EOF silently
        }
        Ok(_) => {}
        Err(e) => {
            return Err(format!("read error: {}", e));
        }
    }

    while line.ends_with('\n') || line.ends_with('\r') {
        line.pop();
    }

    let mut final_line = line;
    if !raw_mode {
        while final_line.ends_with('\\') {
            final_line.pop(); // Remove backslash
            let mut next_line = String::new();
            match handle.read_line(&mut next_line) {
                Ok(0) => break,
                Ok(_) => {
                    while next_line.ends_with('\n') || next_line.ends_with('\r') {
                        next_line.pop();
                    }
                    final_line.push_str(&next_line);
                }
                Err(_) => break,
            }
        }
    }

    let default_ifs = String::from(" \t\n");
    let ifs = state.variables.get("IFS").cloned().unwrap_or(default_ifs);
    let mut remaining = final_line.as_str();

    for (i, var_name) in var_names.iter().enumerate() {
        let val = if i == var_names.len() - 1 {
            if ifs == " \t\n" {
                remaining.trim_start()
            } else {
                remaining
            }
        } else {
            if ifs == " \t\n" {
                remaining = remaining.trim_start();
                if let Some(pos) = remaining.find(|c: char| c.is_whitespace()) {
                    let word = &remaining[..pos];
                    remaining = &remaining[pos..];
                    word
                } else {
                    let word = remaining;
                    remaining = "";
                    word
                }
            } else {
                if let Some(pos) = remaining.find(|c| ifs.contains(c)) {
                    let word = &remaining[..pos];
                    remaining = &remaining[pos + 1..];
                    word
                } else {
                    let word = remaining;
                    remaining = "";
                    word
                }
            }
        };

        state.variables.insert(var_name.clone(), val.to_string());
        if std::env::var(var_name).is_ok() {
            unsafe { std::env::set_var(var_name, val.to_string()); }
        }
    }

    Ok(())
}
