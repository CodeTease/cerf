use std::fs;
use std::io::{self, BufRead, Write};
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.cat",
    description: "Concatenate and print files.",
    usage: "fs.cat [OPTION]... [FILE]...\n\nConcatenate FILE(s) to standard output.\n\nOptions:\n  -n  Show line number\n  -b  Show line number if not blank\n  -e  Show a $ symbol in the end of line\n  -s  Squeeze consecutive empty lines",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    let mut show_line_num = false;
    let mut show_nonblank_num = false;
    let mut show_ends = false;
    let mut squeeze_blank = false;

    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-n" => show_line_num = true,
            "-b" => {
                show_line_num = true;
                show_nonblank_num = true;
            }
            "-e" => show_ends = true,
            "-s" => squeeze_blank = true,
            _ => files.push(arg),
        }
    }

    if files.is_empty() {
        return (ExecutionResult::KeepRunning, 0);
    }

    let mut exit_code = 0;
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    
    let mut line_counter = 1;
    let mut prev_was_blank = false;

    let fast_path = !show_line_num && !show_ends && !squeeze_blank;

    for arg in files {
        let path = expand_home(arg);
        match fs::File::open(&path) {
            Ok(file) => {
                if fast_path {
                    let mut reader = io::BufReader::new(file);
                    if let Err(e) = io::copy(&mut reader, &mut handle) {
                        eprintln!("cerf: fs.cat: {}: {}", arg, e);
                        exit_code = 1;
                    }
                } else {
                    let mut reader = io::BufReader::new(file);
                    let mut buffer = Vec::new();
                    
                    loop {
                        buffer.clear();
                        match reader.read_until(b'\n', &mut buffer) {
                            Ok(0) => break,
                            Ok(_) => {
                                let is_blank = buffer == b"\n" || buffer == b"\r\n";

                                if squeeze_blank {
                                    if is_blank {
                                        if prev_was_blank {
                                            continue;
                                        }
                                        prev_was_blank = true;
                                    } else {
                                        prev_was_blank = false;
                                    }
                                }

                                if show_line_num && (!show_nonblank_num || !is_blank) {
                                    if let Err(e) = write!(handle, "{:>6}\t", line_counter) {
                                        eprintln!("cerf: fs.cat: {}: {}", arg, e);
                                        exit_code = 1;
                                        break;
                                    }
                                    line_counter += 1;
                                }

                                if show_ends {
                                    if buffer.ends_with(b"\r\n") {
                                        buffer.truncate(buffer.len() - 2);
                                        buffer.extend_from_slice(b"$\r\n");
                                    } else if buffer.ends_with(b"\n") {
                                        buffer.truncate(buffer.len() - 1);
                                        buffer.extend_from_slice(b"$\n");
                                    } else {
                                        buffer.push(b'$');
                                    }
                                }

                                if let Err(e) = handle.write_all(&buffer) {
                                    eprintln!("cerf: fs.cat: {}: {}", arg, e);
                                    exit_code = 1;
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("cerf: fs.cat: {}: {}", arg, e);
                                exit_code = 1;
                                break;
                            }
                        }
                    }
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
