use nom::{
    branch::alt,
    bytes::complete::is_not,
    character::complete::{char, multispace0, multispace1},
    sequence::{delimited, preceded},
    IResult,
    Parser,
};

use super::ast::{Connector, ParsedCommand, Pipeline, Redirect, RedirectKind};

// ── Low-level nom parsers ──────────────────────────────────────────────────

pub fn parse_quoted_string(input: &str) -> IResult<&str, String> {
    let (input, content) = delimited(char('"'), is_not("\""), char('"')).parse(input)?;
    Ok((input, content.to_string()))
}

pub fn parse_unquoted_string(input: &str) -> IResult<&str, String> {
    // Stop at whitespace, quotes, AND the connector / redirect characters ; & | > <
    let (input, content) = is_not(" \t\r\n\";|&><")(input)?;
    Ok((input, content.to_string()))
}

pub fn parse_arg(input: &str) -> IResult<&str, String> {
    alt((parse_quoted_string, parse_unquoted_string)).parse(input)
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
    let (input, file) = parse_arg(input)?;
    Ok((input, Redirect { kind, file }))
}

// ── Single command (with redirects) ───────────────────────────────────────

pub fn parse_single_command(input: &str) -> IResult<&str, ParsedCommand> {
    let (mut rest, _) = multispace0(input)?;

    // Parse the command name first
    let (after_name, name) = parse_arg(rest)?;
    rest = after_name;

    let mut args: Vec<String> = Vec::new();
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

    Ok((rest, ParsedCommand { name, args, redirects }))
}

// ── Pipeline expression (cmd | cmd | …) ──────────────────────────────────

/// Parse a pipeline: `command (| command)*`.
pub fn parse_pipeline_expr(input: &str) -> IResult<&str, Pipeline> {
    let (mut rest, first) = parse_single_command(input)?;
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

    Ok((rest, Pipeline { commands }))
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
    ))
    .parse(input)
}
