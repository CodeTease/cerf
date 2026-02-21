use crate::engine::ShellState;

/// Run the `set` builtin.
///
/// Behaviour (follows POSIX / bash conventions):
///
/// - `set`                → print all shell variables (name=value), sorted
/// - `set -o`             → print all shell options with on/off status
/// - `set -o <option>`    → enable the named shell option
/// - `set +o`             → print all shell options as re-inputtable commands
/// - `set +o <option>`    → disable the named shell option
/// - `set -e` / `set +e`  → short-form to enable / disable `errexit`
/// - `set -u` / `set +u`  → short-form to enable / disable `nounset`
/// - `set -x` / `set +x`  → short-form to enable / disable `xtrace`
/// - `set -f` / `set +f`  → short-form to enable / disable `noglob`
/// - `set -- arg …`      → set positional parameters ($1, $2, …)
pub fn run(args: &[String], state: &mut ShellState) -> i32 {
    // No arguments: print all shell variables, sorted.
    if args.is_empty() {
        let mut pairs: Vec<(&String, &String)> = state.variables.iter().collect();
        pairs.sort_by_key(|(k, _)| (*k).clone());
        for (name, value) in pairs {
            println!("{}={}", name, shell_quote(value));
        }
        return 0;
    }

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        // `--` marks the start of positional parameters.
        if arg == "--" {
            set_positional_params(&args[i + 1..], state);
            return 0;
        }

        // `-o` / `+o` long option form.
        if arg == "-o" {
            if i + 1 < args.len() {
                let opt_name = &args[i + 1];
                if let Err(msg) = set_option_by_name(opt_name, true, state) {
                    eprintln!("cerf: set: {}", msg);
                    return 1;
                }
                i += 2;
                continue;
            } else {
                // `-o` with no argument → print option table (human-readable).
                print_options_table(state);
                return 0;
            }
        }
        if arg == "+o" {
            if i + 1 < args.len() {
                let opt_name = &args[i + 1];
                if let Err(msg) = set_option_by_name(opt_name, false, state) {
                    eprintln!("cerf: set: {}", msg);
                    return 1;
                }
                i += 2;
                continue;
            } else {
                // `+o` with no argument → print as re-inputtable commands.
                print_options_commands(state);
                return 0;
            }
        }

        // Short-form flags like `-eux`, `+eux`.
        if arg.starts_with('-') && arg.len() > 1 && arg.as_bytes()[1] != b'-' {
            for ch in arg[1..].chars() {
                if let Err(msg) = set_option_by_char(ch, true, state) {
                    eprintln!("cerf: set: {}", msg);
                    return 1;
                }
            }
            i += 1;
            continue;
        }
        if arg.starts_with('+') && arg.len() > 1 {
            for ch in arg[1..].chars() {
                if let Err(msg) = set_option_by_char(ch, false, state) {
                    eprintln!("cerf: set: {}", msg);
                    return 1;
                }
            }
            i += 1;
            continue;
        }

        // Anything else is treated as positional parameters (POSIX behaviour).
        set_positional_params(&args[i..], state);
        return 0;
    }

    0
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Map a single-character flag to its long option name and set the option.
fn set_option_by_char(ch: char, enable: bool, state: &mut ShellState) -> Result<(), String> {
    let name = match ch {
        'e' => "errexit",
        'u' => "nounset",
        'x' => "xtrace",
        'f' => "noglob",
        'n' => "noexec",
        'v' => "verbose",
        'h' => "hashall",
        'b' => "notify",
        'C' => "noclobber",
        _ => return Err(format!("set: invalid option: -{}", ch)),
    };
    set_option_by_name(name, enable, state)
}

/// Enable or disable a shell option by its long name.
fn set_option_by_name(name: &str, enable: bool, state: &mut ShellState) -> Result<(), String> {
    match name {
        "errexit" | "nounset" | "xtrace" | "noglob" | "noexec" | "verbose" | "hashall"
        | "notify" | "noclobber" => {
            if enable {
                state.set_options.insert(name.to_string());
            } else {
                state.set_options.remove(name);
            }
            Ok(())
        }
        _ => Err(format!("set: unrecognised option: {}", name)),
    }
}

/// Print a human-readable table of all shell options.
fn print_options_table(state: &ShellState) {
    for name in option_names() {
        let status = if state.set_options.contains(*name) {
            "on"
        } else {
            "off"
        };
        println!("{:<15} {}", name, status);
    }
}

/// Print shell options as `set -o`/`set +o` commands (re-inputtable form).
fn print_options_commands(state: &ShellState) {
    for name in option_names() {
        if state.set_options.contains(*name) {
            println!("set -o {}", name);
        } else {
            println!("set +o {}", name);
        }
    }
}

/// Canonical ordered list of supported option names.
fn option_names() -> &'static [&'static str] {
    &[
        "errexit",
        "hashall",
        "noclobber",
        "noexec",
        "noglob",
        "notify",
        "nounset",
        "verbose",
        "xtrace",
    ]
}

/// Set positional parameters ($1, $2, …) as shell variables.
///
/// Previous positional parameters are cleared first.
fn set_positional_params(params: &[String], state: &mut ShellState) {
    // Remove old positional parameters.
    let mut idx = 1;
    loop {
        let key = idx.to_string();
        if state.variables.remove(&key).is_none() {
            break;
        }
        idx += 1;
    }

    // Remove old count.
    state.variables.remove("#");

    // Set new positional parameters.
    for (i, val) in params.iter().enumerate() {
        state
            .variables
            .insert((i + 1).to_string(), val.clone());
    }
    state
        .variables
        .insert("#".to_string(), params.len().to_string());
}

/// Quote a value for shell display (minimal quoting).
fn shell_quote(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    // If the value contains characters that need quoting, wrap in single quotes.
    if s.contains(|c: char| c.is_whitespace() || "\"'\\$`!#&|;(){}".contains(c)) {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}
