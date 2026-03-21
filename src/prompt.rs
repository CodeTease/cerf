use crate::engine;
use crate::engine::ShellState;
use crate::parser;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

pub fn wrap_ansi_escapes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1B' && i + 1 < chars.len() && chars[i + 1] == '[' {
            // Start of CSI sequence
            out.push('\x01');
            out.push('\x1B');
            out.push('[');
            i += 2;
            while i < chars.len() {
                let c = chars[i];
                out.push(c);
                i += 1;
                if c >= '\x40' && c <= '\x7E' {
                    break;
                }
            }
            out.push('\x02');
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

pub fn build_prompt(state: &mut ShellState) -> String {
    let ps1 = state
        .get_var_string("PS1")
        .unwrap_or_else(|| "\\u@\\h:\\w\\$ ".to_string());

    let mut result = String::new();
    let chars: Vec<char> = ps1.chars().collect();
    let mut i = 0;

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let home = dirs::home_dir();

    // Helper for \w
    let get_w = || -> String {
        if let Some(h) = &home {
            if cwd.starts_with(h) {
                let relative = cwd.strip_prefix(h).unwrap();
                if relative.as_os_str().is_empty() {
                    return "~".to_string();
                } else {
                    let sep = std::path::MAIN_SEPARATOR;
                    return format!("~{}{}", sep, relative.display());
                }
            }
        }
        cwd.display().to_string()
    };

    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '(' {
            // Command substitution
            let mut j = i + 2;
            let mut depth = 1;
            let mut cmd = String::new();
            while j < chars.len() {
                if chars[j] == '$' && j + 1 < chars.len() && chars[j + 1] == '(' {
                    depth += 1;
                    cmd.push(chars[j]);
                    cmd.push(chars[j + 1]);
                    j += 2;
                    continue;
                } else if chars[j] == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                cmd.push(chars[j]);
                j += 1;
            }

            if depth == 0 {
                // Execute cmd
                let temp_file = env::temp_dir().join(format!("cerf_prompt_{}", process::id()));
                
                // To be safe against spaces in temp path:
                let temp_path_str = temp_file.to_string_lossy().to_string();
                let wrapped_cmd = format!("{} > \"{}\"", cmd, temp_path_str);
                
                if let Some(entries) = parser::parse_pipeline(&wrapped_cmd, &state.variables) {
                    let _ = engine::execute_list(entries, state);
                }
                
                if let Ok(output) = fs::read_to_string(&temp_file) {
                    let cleaned = wrap_ansi_escapes(output.trim_end_matches('\n'));
                    result.push_str(&cleaned);
                    let _ = fs::remove_file(&temp_file);
                }
                
                i = j + 1;
                continue;
            } else {
                // Unclosed $(
                result.push('$');
                result.push('(');
                i += 2;
                continue;
            }
        }

        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'u' => {
                    let user = env::var("USER")
                        .or_else(|_| env::var("USERNAME"))
                        .unwrap_or_else(|_| "unknown".to_string());
                    result.push_str(&user);
                }
                'h' => {
                    let host = sysinfo::System::host_name().unwrap_or_else(|| "localhost".to_string());
                    let short_host = host.split('.').next().unwrap_or("localhost");
                    result.push_str(short_host);
                }
                'H' => {
                    let host = sysinfo::System::host_name().unwrap_or_else(|| "localhost".to_string());
                    result.push_str(&host);
                }
                'w' => {
                    result.push_str(&get_w());
                }
                'W' => {
                    if let Some(name) = cwd.file_name() {
                        result.push_str(&name.to_string_lossy());
                    } else {
                        result.push_str(&get_w());
                    }
                }
                's' => {
                    result.push_str("cerf");
                }
                'd' => {
                    result.push_str(&chrono::Local::now().format("%a %b %d").to_string());
                }
                't' => {
                    result.push_str(&chrono::Local::now().format("%H:%M:%S").to_string());
                }
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                'e' => result.push('\x1B'),
                '[' => result.push('\x01'),
                ']' => result.push('\x02'),
                '\\' => result.push('\\'),
                '$' => {
                    let uid = env::var("UID").unwrap_or_default();
                    if uid == "0" {
                        result.push('#');
                    } else {
                        result.push('$');
                    }
                }
                c => {
                    result.push('\\');
                    result.push(c);
                }
            }
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}
