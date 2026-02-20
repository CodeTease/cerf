use nom::{
    branch::alt,
    bytes::complete::is_not,
    character::complete::{char, multispace0, multispace1},
    sequence::{delimited, preceded},
    IResult,
    multi::many0,
    Parser,
};

use super::ast::{Connector, ParsedCommand};

// ── Low-level nom parsers ──────────────────────────────────────────────────

pub fn parse_quoted_string(input: &str) -> IResult<&str, String> {
    let (input, content) = delimited(char('"'), is_not("\""), char('"')).parse(input)?;
    Ok((input, content.to_string()))
}

pub fn parse_unquoted_string(input: &str) -> IResult<&str, String> {
    // Stop at whitespace, quotes, AND the connector characters ; & |
    let (input, content) = is_not(" \t\r\n\";|&")(input)?;
    Ok((input, content.to_string()))
}

pub fn parse_arg(input: &str) -> IResult<&str, String> {
    alt((parse_quoted_string, parse_unquoted_string)).parse(input)
}

pub fn parse_single_command(input: &str) -> IResult<&str, ParsedCommand> {
    let (input, _) = multispace0(input)?;
    let (input, name) = parse_arg(input)?;

    // Arguments are separated by whitespace
    let (input, args) = many0(preceded(multispace1, parse_arg)).parse(input)?;
    let (input, _) = multispace0(input)?;

    Ok((input, ParsedCommand { name, args }))
}

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
