use crate::builtins::registry::CommandInfo;
use crate::engine::state::{ExecutionResult, ShellState};
use std::io::{self, Write};

pub const COMMAND_INFO_PRINTF: CommandInfo = CommandInfo {
    name: "io.printf",
    description: "Formats and prints arguments.",
    usage: "io.printf format [arguments]\n\nFormats and prints arguments dynamically according to the FORMAT.",
    run: printf_runner,
};

pub fn printf_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        eprintln!("cerf: printf: usage: printf format [arguments]");
        return (ExecutionResult::KeepRunning, 1);
    }

    let format = &args[0];
    let mut args_iter = args[1..].iter();

    let mut out = String::new();
    let chars: Vec<char> = format.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let mut ch = chars[i];
        if ch == '\\' && i + 1 < chars.len() {
            i += 1;
            match chars[i] {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                '\\' => out.push('\\'),
                'r' => out.push('\r'),
                'a' => out.push('\x07'),
                'e' => out.push('\x1b'),
                c => {
                    out.push('\\');
                    out.push(c);
                }
            }
        } else if ch == '%' && i + 1 < chars.len() {
            i += 1;
            ch = chars[i];

            if ch == '%' {
                out.push('%');
            } else {
                let s = args_iter.next().map(|s| s.as_str()).unwrap_or("");
                match ch {
                    's' | 'b' => out.push_str(s),
                    'd' | 'i' => {
                        let parsed = s.parse::<i64>().unwrap_or(0);
                        out.push_str(&parsed.to_string());
                    }
                    'x' => {
                        let parsed = s.parse::<i64>().unwrap_or(0);
                        out.push_str(&format!("{:x}", parsed));
                    }
                    'X' => {
                        let parsed = s.parse::<i64>().unwrap_or(0);
                        out.push_str(&format!("{:X}", parsed));
                    }
                    'u' => {
                        let parsed = s.parse::<u64>().unwrap_or(0);
                        out.push_str(&parsed.to_string());
                    }
                    _ => {
                        // Unrecognized simple format
                        out.push('%');
                        out.push(ch);
                    }
                }
            }
        } else {
            out.push(ch);
        }
        i += 1;
    }

    // In bash, if args remaining, format is reused.
    // We do a simple version: don't loop format.

    print!("{}", out);
    let _ = io::stdout().flush();
    (ExecutionResult::KeepRunning, 0)
}
