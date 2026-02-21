use nom::{
    branch::alt,
    bytes::complete::is_not,
    character::complete::{char, multispace0, multispace1},
    sequence::{delimited, preceded},
    IResult,
    Parser,
};

use super::ast::{Arg, Connector, ParsedCommand, Pipeline, Redirect, RedirectKind};

// ── Low-level nom parsers ──────────────────────────────────────────────────

/// A raw parsed segment: the text content and whether it came from quotes.
type Segment = (String, bool);

/// Parse a double-quoted string: `"…"` — returns the content without quotes.
fn parse_double_quoted(input: &str) -> IResult<&str, Segment> {
    let (input, content) = delimited(char('"'), is_not("\""), char('"')).parse(input)?;
    Ok((input, (content.to_string(), true)))
}

/// Parse a single-quoted string: `'…'` — returns the content without quotes.
/// Single quotes suppress ALL special characters (POSIX behaviour).
fn parse_single_quoted(input: &str) -> IResult<&str, Segment> {
    let (input, content) = delimited(char('\''), is_not("'"), char('\'')).parse(input)?;
    Ok((input, (content.to_string(), true)))
}

/// Parse an unquoted run of ordinary characters.
/// Stops at whitespace, quotes (`"` or `'`), and shell meta-characters.
fn parse_unquoted(input: &str) -> IResult<&str, Segment> {
    let (input, content) = is_not(" \t\r\n\"';& |><")(input)?;
    Ok((input, (content.to_string(), false)))
}

/// Parse one "word" (shell argument/token).
///
/// A word is one or more adjacent segments where each segment is one of:
/// - an unquoted run (no whitespace / meta-chars)
/// - a `'…'` single-quoted string
/// - a `"…"` double-quoted string
///
/// Adjacent segments are concatenated, so `foo'bar baz'"qux"` → `foobar bazqux`.
/// This matches POSIX sh tokenisation.
///
/// Returns `Arg { value, quoted }` where `quoted` is `true` only when the
/// **entire** word consists of a single quoted segment (e.g. `"hello"`).
pub fn parse_arg(input: &str) -> IResult<&str, Arg> {
    // We need at least one segment.
    let (mut rest, first) = alt((
        parse_double_quoted,
        parse_single_quoted,
        parse_unquoted,
    ))
    .parse(input)?;

    let mut value = first.0;
    let mut segment_count = 1u32;
    let first_quoted = first.1;

    // Greedily consume further adjacent segments (no whitespace between them).
    loop {
        match alt((parse_double_quoted, parse_single_quoted, parse_unquoted)).parse(rest) {
            Ok((after, segment)) => {
                value.push_str(&segment.0);
                segment_count += 1;
                // If any later segment differs in quote-state, it's mixed.
                rest = after;
            }
            Err(_) => break,
        }
    }

    // The word is considered "fully quoted" only when it is exactly one
    // quoted segment (e.g., `"*.txt"` or `'*.txt'`).
    let quoted = segment_count == 1 && first_quoted;

    Ok((rest, Arg { value, quoted }))
}

/// Parse one "word" as a plain `String` (used for redirect targets and
/// assignment values where quoting metadata is irrelevant).
pub fn parse_word(input: &str) -> IResult<&str, String> {
    let (rest, arg) = parse_arg(input)?;
    Ok((rest, arg.value))
}

// ── Redirect parsing ──────────────────────────────────────────────────────

/// Parse a single redirect operator (`>>`, `>`, or `<`) followed by a filename.
fn parse_redirect(input: &str) -> IResult<&str, Redirect> {
    let (input, _) = multispace0(input)?;
    let (input, kind) = alt((
        nom::combinator::map(nom::bytes::complete::tag(">>"), |_| RedirectKind::StdoutAppend),
        nom::combinator::map(char('>'), |_| RedirectKind::StdoutOverwrite),
        nom::combinator::map(char('<'), |_| RedirectKind::StdinFrom),
    ))
    .parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, file) = parse_word(input)?;
    Ok((input, Redirect { kind, file }))
}

// ── Assignment parsing ────────────────────────────────────────────────────

/// Parse a shell assignment: `VAR=VALUE`.
fn parse_assignment(input: &str) -> IResult<&str, (String, String)> {
    let (input, name) = is_not(" \t\r\n\"';& |=><")(input)?;
    if name.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)));
    }
    let (input, _) = char('=')(input)?;
    let (input, value) = match parse_word(input) {
        Ok((rest, val)) => (rest, val),
        Err(_) => (input, String::new()),
    };
    Ok((input, (name.to_string(), value)))
}

// ── Single command (with redirects) ───────────────────────────────────────

pub fn parse_single_command(input: &str) -> IResult<&str, ParsedCommand> {
    let (mut rest, _) = multispace0(input)?;

    let mut assignments: Vec<(String, String)> = Vec::new();

    // Parse zero or more assignments first.
    loop {
        if let Ok((after_assign, assign)) = parse_assignment(rest) {
            assignments.push(assign);
            let (after_space, _) = multispace0(after_assign)?;
            rest = after_space;
        } else {
            break;
        }
    }

    // Parse the command name (optional if assignments are present).
    let (after_name, name) = match parse_arg(rest) {
        Ok((after, n)) => (after, Some(n.value)),
        Err(e) => {
            if assignments.is_empty() {
                return Err(e);
            } else {
                (rest, None)
            }
        }
    };
    rest = after_name;

    let mut args: Vec<Arg> = Vec::new();
    let mut redirects: Vec<Redirect> = Vec::new();

    // Parse arguments and redirects interleaved, until we hit a connector or
    // pipe or end-of-input.
    loop {
        // Try redirects first (they start with > or <)
        if let Ok((after_redir, redir)) = parse_redirect(rest) {
            redirects.push(redir);
            rest = after_redir;
            continue;
        }

        // Try an argument preceded by whitespace
        if let Ok((after_arg, arg)) = preceded(multispace1, parse_arg).parse(rest) {
            args.push(arg);
            rest = after_arg;
            continue;
        }

        // Nothing left to consume for this command
        break;
    }

    let (rest, _) = multispace0(rest)?;

    Ok((rest, ParsedCommand { assignments, name, args, redirects }))
}

// ── Pipeline expression (cmd | cmd | …) ──────────────────────────────────

/// Parse a pipeline: `[!] command (| command)*`.
pub fn parse_pipeline_expr(input: &str) -> IResult<&str, Pipeline> {
    let (input, _) = multispace0(input)?;
    
    // Check for logical NOT operator '!'
    let (rest, negated) = if input.starts_with('!') {
        // '!' must be its own token or followed by whitespace
        let after_bang = &input[1..];
        if after_bang.is_empty() || after_bang.starts_with(char::is_whitespace) {
            (after_bang, true)
        } else {
            (input, false)
        }
    } else {
        (input, false)
    };

    let (mut rest, first) = parse_single_command(rest)?;
    let mut commands = vec![first];

    loop {
        let trimmed = rest.trim_start();
        // A pipe is a single `|` NOT followed by another `|` (that would be `||`).
        if trimmed.starts_with('|') && !trimmed.starts_with("||") {
            let after_pipe = &trimmed[1..];
            match parse_single_command(after_pipe) {
                Ok((after_cmd, cmd)) => {
                    commands.push(cmd);
                    rest = after_cmd;
                }
                Err(_) => break,
            }
        } else {
            break;
        }
    }

    Ok((rest, Pipeline { commands, negated, background: false }))
}

// ── Connector parsing ─────────────────────────────────────────────────────

/// Parse a connector operator: `&&`, `||`, or `;`.
pub fn parse_connector(input: &str) -> IResult<&str, Connector> {
    let (input, _) = multispace0(input)?;
    alt((
        // Two-character operators must come before single-character ones.
        nom::combinator::map(nom::bytes::complete::tag("&&"), |_| Connector::And),
        nom::combinator::map(nom::bytes::complete::tag("||"), |_| Connector::Or),
        nom::combinator::map(char(';'), |_| Connector::Semi),
        nom::combinator::map(char('&'), |_| Connector::Amp),
    ))
    .parse(input)
}
