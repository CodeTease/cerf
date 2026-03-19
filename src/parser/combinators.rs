use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::is_not,
    character::complete::{char, line_ending, multispace0, multispace1, space0, space1},
    multi::many1,
    sequence::{delimited, preceded},
};

use super::ast::{
    Arg, CommandEntry, CommandNode, Connector, Pipeline, Redirect, RedirectKind, SimpleCommand,
};

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
    let (mut rest, first) =
        alt((parse_double_quoted, parse_single_quoted, parse_unquoted)).parse(input)?;

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
        nom::combinator::map(nom::bytes::complete::tag(">>"), |_| {
            RedirectKind::StdoutAppend
        }),
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
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }
    let (input, _) = char('=')(input)?;
    let (input, value) = match parse_word(input) {
        Ok((rest, val)) => (rest, val),
        Err(_) => (input, String::new()),
    };
    Ok((input, (name.to_string(), value)))
}

// ── Single command (with redirects) ───────────────────────────────────────

pub const RESERVED_WORDS: &[&str] = &[
    "if", "elif", "else", "func", "for", "in", "while", "loop", "break", "continue", "{", "}", "!",
];

pub fn is_reserved_word(word: &str) -> bool {
    RESERVED_WORDS.contains(&word)
}

pub fn parse_simple_command(input: &str) -> IResult<&str, SimpleCommand> {
    let (mut rest, _) = space0(input)?;

    let mut assignments: Vec<(String, String)> = Vec::new();

    // Parse zero or more assignments first.
    loop {
        if let Ok((after_assign, assign)) = parse_assignment(rest) {
            assignments.push(assign);
            let (after_space, _) = space0(after_assign)?;
            rest = after_space;
        } else {
            break;
        }
    }

    // Parse the command name (optional if assignments are present).
    let (after_name, name) = match parse_arg(rest) {
        Ok((after, arg)) => {
            if !arg.quoted && is_reserved_word(&arg.value) {
                // Return an error if a reserved word is used unquoted as a command name.
                return Err(nom::Err::Error(nom::error::Error::new(
                    rest,
                    nom::error::ErrorKind::Tag,
                )));
            }
            (after, Some(arg.value))
        }
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
        // Check if the next word is a brace
        if let Ok((r2, _)) = space0::<_, nom::error::Error<&str>>(rest)
            && (r2.starts_with('{') || r2.starts_with('}')) {
                break;
            }

        // Try redirects first (they start with > or <)
        if let Ok((after_redir, redir)) = parse_redirect(rest) {
            redirects.push(redir);
            rest = after_redir;
            continue;
        }

        // Try an argument preceded by whitespace
        if let Ok((after_arg, arg)) = preceded(space1, parse_arg).parse(rest) {
            args.push(arg);
            rest = after_arg;
            continue;
        }

        // Nothing left to consume for this command
        break;
    }

    let (rest, _) = space0(rest)?;

    Ok((
        rest,
        SimpleCommand {
            assignments,
            name,
            args,
            redirects,
        },
    ))
}

// ── Block, If, and Func parsing ──────────────────────────────────────────

/// Parse a block of commands `{ ... }`, returning `Vec<CommandEntry>`.
fn parse_block_body(input: &str) -> IResult<&str, Vec<CommandEntry>> {
    let (input, _) = delimited(multispace0, char('{'), multispace0).parse(input)?;

    // If empty block
    if input.starts_with('}') {
        let (input, _) = char('}')(input)?;
        return Ok((input, Vec::new()));
    }

    let (input, entries) = parse_command_list(input)?;
    let (input, _) = delimited(multispace0, char('}'), multispace0).parse(input)?;
    Ok((input, entries))
}

/// Parse an `if` command.
fn parse_if_command(input: &str) -> IResult<&str, CommandNode> {
    let (input, _) = nom::bytes::complete::tag("if")(input)?;
    let (input, _) = multispace1(input)?;

    let (input, cond) = parse_command_list(input)?;
    let (input, body) = parse_block_body(input)?;

    let mut branches = vec![(cond, body)];
    let mut rest = input;
    let mut else_branch = None;

    loop {
        let (r, _) = multispace0(rest)?;
        if let Ok((r2, _)) =
            nom::bytes::complete::tag::<_, _, nom::error::Error<&str>>("elif").parse(r)
        {
            let (r3, _) = multispace1(r2)?;
            let (r4, elif_cond) = parse_command_list(r3)?;
            let (r5, elif_body) = parse_block_body(r4)?;
            branches.push((elif_cond, elif_body));
            rest = r5;
        } else if let Ok((r2, _)) =
            nom::bytes::complete::tag::<_, _, nom::error::Error<&str>>("else").parse(r)
        {
            let (r3, _) = multispace0(r2)?;
            let (r4, e_body) = parse_block_body(r3)?;
            else_branch = Some(e_body);
            rest = r4;
            break;
        } else {
            break;
        }
    }

    let mut redirects = Vec::new();
    while let Ok((new_rest, redir)) = parse_redirect(rest) {
        redirects.push(redir);
        rest = new_rest;
    }

    Ok((
        rest,
        CommandNode::If {
            branches,
            else_branch,
            redirects,
        },
    ))
}

/// Parse a `func` declaration.
fn parse_func_decl(input: &str) -> IResult<&str, CommandNode> {
    let (input, _) = nom::bytes::complete::tag("func")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name_str) = is_not(" \t\r\n{")(input)?;
    let name = name_str.trim().to_string();
    let (input, body) = parse_block_body(input)?;

    Ok((input, CommandNode::FuncDecl { name, body }))
}

/// Parse a `for` loop command.
fn parse_for_command(input: &str) -> IResult<&str, CommandNode> {
    let (input, _) = nom::bytes::complete::tag("for")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, var_str) = is_not(" \t\r\n")(input)?;
    let var = var_str.trim().to_string();
    let (input, _) = multispace1(input)?;
    let (input, _) = nom::bytes::complete::tag("in")(input)?;
    let (mut rest, _) = multispace1(input)?;

    let mut items = Vec::new();
    loop {
        if let Ok((r, _)) = multispace0::<_, nom::error::Error<&str>>(rest)
            && r.starts_with('{') {
                rest = r;
                break;
            }

        match parse_arg(rest) {
            Ok((after_arg, arg)) => {
                items.push(arg);
                let (after_space, _) = multispace0(after_arg)?;
                rest = after_space;
            }
            Err(_) => break, // Fallback if no arg can be parsed
        }
    }

    let (mut rest, body) = parse_block_body(rest)?;

    let mut redirects = Vec::new();
    while let Ok((new_rest, redir)) = parse_redirect(rest) {
        redirects.push(redir);
        rest = new_rest;
    }

    Ok((
        rest,
        CommandNode::For {
            var,
            items,
            body,
            redirects,
        },
    ))
}

/// Parse a `while` loop command.
fn parse_while_command(input: &str) -> IResult<&str, CommandNode> {
    let (input, _) = nom::bytes::complete::tag("while")(input)?;
    let (input, _) = multispace1(input)?;

    let (input, cond) = parse_command_list(input)?;
    let (input, body) = parse_block_body(input)?;
    let mut rest = input;
    let mut redirects = Vec::new();
    while let Ok((new_rest, redir)) = parse_redirect(rest) {
        redirects.push(redir);
        rest = new_rest;
    }

    Ok((
        rest,
        CommandNode::While {
            cond,
            body,
            redirects,
        },
    ))
}

/// Parse a `loop` command.
fn parse_loop_command(input: &str) -> IResult<&str, CommandNode> {
    let (input, _) = nom::bytes::complete::tag("loop")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, body) = parse_block_body(input)?;
    let mut rest = input;
    let mut redirects = Vec::new();
    while let Ok((new_rest, redir)) = parse_redirect(rest) {
        redirects.push(redir);
        rest = new_rest;
    }

    Ok((rest, CommandNode::Loop { body, redirects }))
}

/// Parse a `break` command.
fn parse_break_command(input: &str) -> IResult<&str, CommandNode> {
    let (input, _) = nom::bytes::complete::tag("break")(input)?;
    Ok((input, CommandNode::Break))
}

/// Parse a `continue` command.
fn parse_continue_command(input: &str) -> IResult<&str, CommandNode> {
    let (input, _) = nom::bytes::complete::tag("continue")(input)?;
    Ok((input, CommandNode::Continue))
}

/// Parse any command node (if, func, or simple).
pub fn parse_command_node(input: &str) -> IResult<&str, CommandNode> {
    alt((
        parse_if_command,
        parse_func_decl,
        parse_for_command,
        parse_while_command,
        parse_loop_command,
        parse_break_command,
        parse_continue_command,
        nom::combinator::map(parse_simple_command, CommandNode::Simple),
    ))
    .parse(input)
}

// ── Pipeline expression (cmd | cmd | …) ──────────────────────────────────

/// Parse a pipeline: `[!] command (| command)*`.
pub fn parse_pipeline_expr(input: &str) -> IResult<&str, Pipeline> {
    let (input, _) = multispace0(input)?;

    // Check for logical NOT operator '!'
    let (rest, negated) = if let Some(after_bang) = input.strip_prefix('!') {
        // '!' must be its own token or followed by whitespace
        if after_bang.is_empty() || after_bang.starts_with(char::is_whitespace) {
            (after_bang, true)
        } else {
            (input, false)
        }
    } else {
        (input, false)
    };

    let (mut rest, first) = parse_command_node(rest)?;
    let mut commands = vec![first];

    loop {
        let trimmed = rest.trim_start();
        // A pipe is a single `|` NOT followed by another `|` (that would be `||`).
        if trimmed.starts_with('|') && !trimmed.starts_with("||") {
            let after_pipe = &trimmed[1..];
            match parse_command_node(after_pipe) {
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

    Ok((
        rest,
        Pipeline {
            commands,
            negated,
            background: false,
        },
    ))
}

// ── Connector parsing ─────────────────────────────────────────────────────

/// Parse a connector operator: `&&`, `||`, or `;`.
pub fn parse_connector(input: &str) -> IResult<&str, Connector> {
    let (input, _) = space0(input)?;
    alt((
        // Two-character operators must come before single-character ones.
        nom::combinator::map(nom::bytes::complete::tag("&&"), |_| Connector::And),
        nom::combinator::map(nom::bytes::complete::tag("||"), |_| Connector::Or),
        nom::combinator::map(char(';'), |_| Connector::Semi),
        nom::combinator::map(char('&'), |_| Connector::Amp),
        // Newlines act as implicit semicolons
        nom::combinator::map(many1(preceded(space0, line_ending)), |_| Connector::Semi),
    ))
    .parse(input)
}

// ── Command List parsing ──────────────────────────────────────────────────

/// Parses a list of `CommandEntry`s, separating by connectors. Stops before `}` or EOF.
pub fn parse_command_list(input: &str) -> IResult<&str, Vec<CommandEntry>> {
    let mut rest = input;
    let mut entries = Vec::new();

    // Parse the first pipeline
    let (after_first, first_pipeline) = parse_pipeline_expr(rest)?;
    rest = after_first;

    let mut current_pipeline = first_pipeline;
    let mut current_connector = None;

    loop {
        let (r, _) = space0(rest)?;
        if r.is_empty() || r.starts_with('}') || r.starts_with('{') {
            entries.push(CommandEntry {
                connector: current_connector,
                pipeline: current_pipeline,
            });
            rest = r;
            break;
        }

        // We also check for newlines here if they are not picked up by parse_connector
        // But parse_connector should handle them now.

        let (after_conn, conn) = match parse_connector(r) {
            Ok(v) => v,
            Err(_) => {
                entries.push(CommandEntry {
                    connector: current_connector,
                    pipeline: current_pipeline,
                });
                rest = r;
                break;
            }
        };

        if conn == Connector::Amp {
            current_pipeline.background = true;
            entries.push(CommandEntry {
                connector: current_connector,
                pipeline: current_pipeline,
            });
            rest = after_conn;

            let (r2, _) = multispace0(rest)?;
            if r2.is_empty() || r2.starts_with('}') {
                rest = r2;
                break;
            }

            match parse_pipeline_expr(r2) {
                Ok((after_next, next_pipe)) => {
                    current_pipeline = next_pipe;
                    current_connector = None;
                    rest = after_next;
                    continue;
                }
                Err(_) => {
                    rest = r2;
                    break;
                }
            }
        }

        entries.push(CommandEntry {
            connector: current_connector,
            pipeline: current_pipeline,
        });

        match parse_pipeline_expr(after_conn) {
            Ok((after_next, next_pipe)) => {
                current_pipeline = next_pipe;
                current_connector = Some(conn);
                rest = after_next;
            }
            Err(_) => {
                rest = after_conn;
                break;
            }
        }
    }

    Ok((rest, entries))
}
