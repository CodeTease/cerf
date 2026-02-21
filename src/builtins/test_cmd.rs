use std::fs;
use std::path::Path;

/// The `test` / `[` built-in command.
///
/// Evaluates conditional expressions following POSIX semantics.
/// When invoked as `[`, the last argument must be `]`.
///
/// Supported expressions:
///   String:   -n STRING, -z STRING, STR1 = STR2, STR1 != STR2, STRING (true if non-empty)
///   Integer:  INT1 -eq INT2, INT1 -ne INT2, INT1 -lt INT2,
///             INT1 -le INT2, INT1 -gt INT2, INT1 -ge INT2
///   File:     -e FILE, -f FILE, -d FILE, -r FILE, -w FILE, -x FILE,
///             -s FILE, -L FILE, -h FILE
///   Logic:    ! EXPR, EXPR -a EXPR, EXPR -o EXPR, ( EXPR )
pub fn run(args: &[String], invoked_as_bracket: bool) -> i32 {
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    // When invoked as `[`, the last argument must be `]`.
    let expr_args = if invoked_as_bracket {
        if args.is_empty() || args.last() != Some(&"]") {
            eprintln!("cerf: [: missing closing `]`");
            return 2;
        }
        &args[..args.len() - 1]
    } else {
        &args[..]
    };

    // No arguments → false.
    if expr_args.is_empty() {
        return 1;
    }

    let mut pos = 0;
    match parse_or(expr_args, &mut pos) {
        Ok(result) => {
            if pos != expr_args.len() {
                eprintln!(
                    "cerf: test: unexpected argument `{}`",
                    expr_args[pos]
                );
                2
            } else if result {
                0
            } else {
                1
            }
        }
        Err(e) => {
            eprintln!("cerf: test: {}", e);
            2
        }
    }
}

// ── Recursive-descent parser for test expressions ────────────────────────

/// Parse an `-o` (OR) expression: expr_and ( -o expr_and )*
fn parse_or(args: &[&str], pos: &mut usize) -> Result<bool, String> {
    let mut result = parse_and(args, pos)?;
    while *pos < args.len() && args[*pos] == "-o" {
        *pos += 1;
        let rhs = parse_and(args, pos)?;
        result = result || rhs;
    }
    Ok(result)
}

/// Parse an `-a` (AND) expression: expr_not ( -a expr_not )*
fn parse_and(args: &[&str], pos: &mut usize) -> Result<bool, String> {
    let mut result = parse_not(args, pos)?;
    while *pos < args.len() && args[*pos] == "-a" {
        *pos += 1;
        let rhs = parse_not(args, pos)?;
        result = result && rhs;
    }
    Ok(result)
}

/// Parse a `!` (NOT) expression: !* primary
fn parse_not(args: &[&str], pos: &mut usize) -> Result<bool, String> {
    if *pos < args.len() && args[*pos] == "!" {
        *pos += 1;
        let val = parse_not(args, pos)?;
        Ok(!val)
    } else {
        parse_primary(args, pos)
    }
}

/// Parse a primary expression: parenthesised group, unary test, binary test,
/// or a bare string (non-empty → true).
fn parse_primary(args: &[&str], pos: &mut usize) -> Result<bool, String> {
    if *pos >= args.len() {
        return Err("expected expression".to_string());
    }

    let token = args[*pos];

    // ── Parenthesised group: ( EXPR ) ────────────────────────────────
    if token == "(" {
        *pos += 1;
        let result = parse_or(args, pos)?;
        if *pos >= args.len() || args[*pos] != ")" {
            return Err("missing closing `)`".to_string());
        }
        *pos += 1;
        return Ok(result);
    }

    // ── Unary file tests ─────────────────────────────────────────────
    if matches!(
        token,
        "-e" | "-f" | "-d" | "-r" | "-w" | "-x" | "-s" | "-L" | "-h"
    ) {
        *pos += 1;
        if *pos >= args.len() {
            return Err(format!("expected argument after `{}`", token));
        }
        let path_str = args[*pos];
        *pos += 1;

        // Check if the next token is a binary operator; if so, this wasn't
        // a unary — it's a string used as the LHS of a binary. Back up.
        // (This handles corner cases like `test -f = -f`.)
        if *pos < args.len() && is_binary_op(args[*pos]) {
            // Reinterpret: treat `token` as a plain string (LHS).
            *pos -= 1; // back up to `path_str` which is actually the operator
            // Actually, we need to step back two: token is LHS, path_str is op
            *pos -= 1;
            // Fall through to the binary/string path below.
        } else {
            return eval_unary_file(token, path_str);
        }
    }

    // ── Unary string tests: -n STRING, -z STRING ─────────────────────
    if token == "-n" || token == "-z" {
        *pos += 1;
        if *pos >= args.len() {
            return Err(format!("expected argument after `{}`", token));
        }
        let operand = args[*pos];
        *pos += 1;

        // Same binary-operator lookahead guard.
        if *pos < args.len() && is_binary_op(args[*pos]) {
            *pos -= 1;
            *pos -= 1;
            // Fall through to binary path.
        } else {
            return match token {
                "-n" => Ok(!operand.is_empty()),
                "-z" => Ok(operand.is_empty()),
                _ => unreachable!(),
            };
        }
    }

    // ── Binary tests: STR1 OP STR2 ──────────────────────────────────
    // Look ahead: if args[pos+1] is a binary operator, this is a binary test.
    if *pos + 2 <= args.len() {
        if *pos + 1 < args.len() && is_binary_op(args[*pos + 1]) {
            let lhs = args[*pos];
            let op = args[*pos + 1];
            if *pos + 2 >= args.len() {
                return Err(format!("expected argument after `{}`", op));
            }
            let rhs = args[*pos + 2];
            *pos += 3;
            return eval_binary(lhs, op, rhs);
        }
    }

    // ── Bare string: non-empty → true ────────────────────────────────
    *pos += 1;
    Ok(!token.is_empty())
}

/// Returns true if `s` is a binary test operator.
fn is_binary_op(s: &str) -> bool {
    matches!(
        s,
        "=" | "==" | "!=" | "-eq" | "-ne" | "-lt" | "-le" | "-gt" | "-ge"
    )
}

// ── Evaluators ───────────────────────────────────────────────────────────

fn eval_unary_file(op: &str, path_str: &str) -> Result<bool, String> {
    let path = Path::new(path_str);
    let meta = fs::symlink_metadata(path); // doesn't follow symlinks

    Ok(match op {
        "-e" => path.exists(),
        "-f" => path.is_file(),
        "-d" => path.is_dir(),
        "-s" => meta.map(|m| m.len() > 0).unwrap_or(false),
        "-r" => is_readable(path),
        "-w" => is_writable(path),
        "-x" => is_executable(path),
        "-L" | "-h" => meta
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false),
        _ => unreachable!(),
    })
}

fn eval_binary(lhs: &str, op: &str, rhs: &str) -> Result<bool, String> {
    match op {
        // String comparisons
        "=" | "==" => Ok(lhs == rhs),
        "!=" => Ok(lhs != rhs),

        // Integer comparisons
        "-eq" | "-ne" | "-lt" | "-le" | "-gt" | "-ge" => {
            let a = lhs
                .parse::<i64>()
                .map_err(|_| format!("integer expression expected: `{}`", lhs))?;
            let b = rhs
                .parse::<i64>()
                .map_err(|_| format!("integer expression expected: `{}`", rhs))?;
            Ok(match op {
                "-eq" => a == b,
                "-ne" => a != b,
                "-lt" => a < b,
                "-le" => a <= b,
                "-gt" => a > b,
                "-ge" => a >= b,
                _ => unreachable!(),
            })
        }

        _ => Err(format!("unknown binary operator `{}`", op)),
    }
}

// ── Platform-specific file permission helpers ────────────────────────────

#[cfg(unix)]
fn is_readable(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    if let Ok(meta) = fs::metadata(path) {
        let mode = meta.mode();
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        if uid == 0 {
            return true; // root can read everything
        }
        if meta.uid() == uid {
            mode & 0o400 != 0
        } else if meta.gid() == gid {
            mode & 0o040 != 0
        } else {
            mode & 0o004 != 0
        }
    } else {
        false
    }
}

#[cfg(unix)]
fn is_writable(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    if let Ok(meta) = fs::metadata(path) {
        let mode = meta.mode();
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        if uid == 0 {
            return true;
        }
        if meta.uid() == uid {
            mode & 0o200 != 0
        } else if meta.gid() == gid {
            mode & 0o020 != 0
        } else {
            mode & 0o002 != 0
        }
    } else {
        false
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    if let Ok(meta) = fs::metadata(path) {
        let mode = meta.mode();
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        if uid == 0 {
            return mode & 0o111 != 0;
        }
        if meta.uid() == uid {
            mode & 0o100 != 0
        } else if meta.gid() == gid {
            mode & 0o010 != 0
        } else {
            mode & 0o001 != 0
        }
    } else {
        false
    }
}

#[cfg(windows)]
fn is_readable(path: &Path) -> bool {
    // On Windows, if the file exists we assume it's readable.
    path.exists()
}

#[cfg(windows)]
fn is_writable(path: &Path) -> bool {
    // On Windows, check the read-only attribute.
    fs::metadata(path)
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    // On Windows, check common executable extensions.
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            let ext = ext.to_lowercase();
            matches!(ext.as_str(), "exe" | "cmd" | "bat" | "com" | "ps1")
        })
        .unwrap_or(false)
}
