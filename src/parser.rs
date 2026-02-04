use nom::{
    branch::alt,
    bytes::complete::{is_not},
    character::complete::{char, multispace0, multispace1},
    sequence::{delimited, preceded},
    IResult,
    multi::many0,
    Parser,
};

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedCommand {
    pub name: String,
    pub args: Vec<String>,
}

fn parse_quoted_string(input: &str) -> IResult<&str, String> {
    let (input, content) = delimited(char('"'), is_not("\""), char('"')).parse(input)?;
    Ok((input, content.to_string()))
}

fn parse_unquoted_string(input: &str) -> IResult<&str, String> {
    let (input, content) = is_not(" \t\r\n\"")(input)?;
    Ok((input, content.to_string()))
}

fn parse_arg(input: &str) -> IResult<&str, String> {
    alt((parse_quoted_string, parse_unquoted_string)).parse(input)
}

fn parse_command_internal(input: &str) -> IResult<&str, ParsedCommand> {
    let (input, _) = multispace0(input)?;
    let (input, name) = parse_arg(input)?;
    
    // Arguments are separated by whitespace
    let (input, args) = many0(preceded(multispace1, parse_arg)).parse(input)?;
    let (input, _) = multispace0(input)?;
    
    Ok((input, ParsedCommand { name, args }))
}

pub fn parse_line(input: &str) -> Option<ParsedCommand> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    match parse_command_internal(input) {
        Ok((_, cmd)) => Some(cmd),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let input = "ls -la";
        let cmd = parse_line(input).unwrap();
        assert_eq!(cmd.name, "ls");
        assert_eq!(cmd.args, vec!["-la"]);
    }

    #[test]
    fn test_parse_quoted() {
        let input = "echo \"hello world\"";
        let cmd = parse_line(input).unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["hello world"]);
    }

    #[test]
    fn test_parse_mixed() {
        let input = "cd \"My Documents\" backup";
        let cmd = parse_line(input).unwrap();
        assert_eq!(cmd.name, "cd");
        assert_eq!(cmd.args, vec!["My Documents", "backup"]);
    }
    
    #[test]
    fn test_extra_spaces() {
        let input = "  ls   -la  ";
        let cmd = parse_line(input).unwrap();
        assert_eq!(cmd.name, "ls");
        assert_eq!(cmd.args, vec!["-la"]);
    }
    
    #[test]
    fn test_empty() {
        assert!(parse_line("").is_none());
        assert!(parse_line("   ").is_none());
    }

    #[test]
    fn test_comment() {
        assert!(parse_line("# comment").is_none());
        assert!(parse_line("   # comment indented").is_none());
    }
}
