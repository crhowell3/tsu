#[derive(Debug, PartialEq)]
/// Enumeration of possible command flags
///
/// Represents command modifier flags. At the moment, the only modifier is the Force flag ("!")
pub enum CommandFlag {
    /// The modifier flag which tells the command parser that the command should be executed with
    /// force, typically bypassing certain safeguards
    Force,
}

#[derive(Debug, Default, PartialEq)]
/// Representation of a tsu command
///
/// A command that has been parsed from user input in Command mode
pub struct ParsedCommand {
    /// The list of commands to execute
    pub commands: Vec<String>,
    /// The list of arguments that correspond with the provided commands
    pub args: Vec<String>,
    /// Modifier flags
    pub flags: Vec<CommandFlag>,
}

impl ParsedCommand {
    /// Checks if the command has the Force modifier
    ///
    /// # Returns
    /// - `true` if the command has the Force modifier, `false` if otherwise
    pub fn is_forced(&self) -> bool {
        self.flags.contains(&CommandFlag::Force)
    }
}

/// Attempt to parse a command
///
/// # Arguments
/// - `commands`:
/// - `input`:
///
/// # Returns
/// - `Some(ParsedCommand)` if parsing is successful, otherwise `None`
pub fn parse(commands: &[&str], input: &str) -> Option<ParsedCommand> {
    let (flags, input) = parse_flags(input);
    let mut parts = input.splitn(2, ' ');
    let input = parts.next()?;
    let args = parts
        .next()
        .map(|s| s.split(' ').map(|s| s.to_string()).collect())
        .unwrap_or_default();
    let commands = parse_commands(commands, input);

    if commands.is_empty() {
        return None;
    }

    Some(ParsedCommand {
        commands,
        args,
        flags,
    })
}

/// Check for flags and parse them appropriately
///
/// # Arguments
/// - `input`:
///
/// # Returns
/// - A tuple consisting of a vector of command flags as well as the remaining input string
fn parse_flags(input: &str) -> (Vec<CommandFlag>, &str) {
    if let Some(input) = input.strip_suffix("!") {
        (vec![CommandFlag::Force], input)
    } else {
        (vec![], input)
    }
}

/// Parse commands given user input
///
/// # Arguments
/// - `commands`:
/// - `input`:
///
/// # Returns
/// - A vector of parsed commands represented as strings
fn parse_commands(commands: &[&str], input: &str) -> Vec<String> {
    for command in commands {
        if &input == command {
            return vec![command.to_string()];
        }
    }

    let mut result = Vec::new();
    for c in input.chars() {
        if let Some(command) = commands.iter().find(|cmd| cmd.starts_with(c)) {
            result.push(command.to_string());
        }
    }

    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        let commands = ["quit", "write"];
        assert_eq!(
            parse(&commands, "quit"),
            Some(ParsedCommand {
                commands: vec!["quit".to_string()],
                flags: vec![],
                ..Default::default()
            })
        );
        assert_eq!(
            parse(&commands, "q"),
            Some(ParsedCommand {
                commands: vec!["quit".to_string()],
                flags: vec![],
                ..Default::default()
            })
        );
        assert_eq!(
            parse(&commands, "q!"),
            Some(ParsedCommand {
                commands: vec!["quit".to_string()],
                flags: vec![CommandFlag::Force],
                ..Default::default()
            })
        );
        assert_eq!(
            parse(&commands, "wq"),
            Some(ParsedCommand {
                commands: vec!["write".to_string(), "quit".to_string()],
                flags: vec![],
                ..Default::default()
            })
        );
        assert_eq!(
            parse(&commands, "wq!"),
            Some(ParsedCommand {
                commands: vec!["write".to_string(), "quit".to_string()],
                flags: vec![CommandFlag::Force],
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_parse_command() {
        let commands = ["quit", "write"];
        assert_eq!(parse_commands(&commands, "quit"), vec!["quit"]);
        assert_eq!(parse_commands(&commands, "q"), vec!["quit"]);
        assert_eq!(parse_commands(&commands, "w"), vec!["write"]);
        assert_eq!(parse_commands(&commands, "wq"), vec!["write", "quit"]);
    }

    #[test]
    fn test_parse_flags() {
        assert_eq!(parse_flags("q"), (vec![], "q"));
        assert_eq!(parse_flags("q!"), (vec![CommandFlag::Force], "q"));
    }
}
