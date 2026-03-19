use crate::builtins::registry::CommandInfo;
use crate::engine::state::{ExecutionResult, ShellState, Variable};
use std::io::{self, BufRead};

pub const COMMAND_INFO_MAPFILE: CommandInfo = CommandInfo {
    name: "io.mapfile",
    description: "Read lines from standard input into an array variable.",
    usage: "io.mapfile [-t] [array]\n\nRead lines from standard input into an indexed array variable.",
    run: mapfile_runner,
};

pub fn mapfile_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    let mut remove_newline = false;
    let mut array_name = "MAPFILE".to_string();

    let mut i = 0;
    while i < args.len() {
        if args[i] == "-t" {
            remove_newline = true;
            i += 1;
        } else if args[i].starts_with('-') {
            eprintln!("cerf: mapfile: invalid option {}", args[i]);
            return (ExecutionResult::KeepRunning, 1);
        } else {
            array_name = args[i].clone();
            break;
        }
    }

    let mut lines = Vec::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    loop {
        let mut line = String::new();
        match handle.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                if remove_newline {
                    while line.ends_with('\n') || line.ends_with('\r') {
                        line.pop();
                    }
                }
                lines.push(line);
            }
            Err(e) => {
                eprintln!("cerf: mapfile: read error: {}", e);
                return (ExecutionResult::KeepRunning, 1);
            }
        }
    }

    state.set_var(&array_name, Variable::new_array(lines));

    (ExecutionResult::KeepRunning, 0)
}
