//! Command line parsing.

use std::fmt;

/// Runtime mode.
///
/// `Test` keeps every side effect away from a real setup: it uses
/// HeadsetControl's simulated device, a separate config file and a separate
/// sidetone state file, so any feature can be tried without hardware and
/// without touching the user's real configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Test,
}

impl Mode {
    /// Suffix appended to config and state files in test mode.
    pub fn suffix(self) -> &'static str {
        match self {
            Mode::Normal => "",
            Mode::Test => ".test",
        }
    }

    pub fn is_test(self) -> bool {
        self == Mode::Test
    }
}

/// What the user asked the binary to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Tui,
    WaybarStatus,
    ToggleSidetone,
    Config,
    Help,
}

impl Command {
    /// The flag that selects this command, if it has one (`Tui` is the default).
    pub fn flag(self) -> Option<&'static str> {
        match self {
            Command::Tui => None,
            Command::WaybarStatus => Some("--waybar-status"),
            Command::ToggleSidetone => Some("--toggle-sidetone"),
            Command::Config => Some("--config"),
            Command::Help => Some("--help"),
        }
    }
}

/// Parsed command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cli {
    pub mode: Mode,
    pub command: Command,
}

/// Why a command line could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// An argument that this binary does not know.
    UnknownArgument(String),
    /// More than one command was requested (e.g. `--config --waybar-status`).
    ConflictingCommands(Vec<&'static str>),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnknownArgument(argument) => {
                write!(f, "unknown argument '{argument}'")
            }
            ParseError::ConflictingCommands(flags) => {
                write!(f, "conflicting commands: {}", flags.join(", "))
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl Cli {
    /// Parse the arguments that follow the binary name.
    ///
    /// `--test` is a modifier and may appear anywhere; every other known flag
    /// selects one command.
    pub fn parse<I, S>(args: I) -> Result<Self, ParseError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut mode = Mode::Normal;
        let mut commands: Vec<Command> = Vec::new();

        for argument in args {
            let argument = argument.as_ref();

            let command = match argument {
                "--test" => {
                    mode = Mode::Test;
                    continue;
                }
                "--help" | "-h" => Command::Help,
                "--config" => Command::Config,
                "--toggle-sidetone" => Command::ToggleSidetone,
                "--waybar-status" => Command::WaybarStatus,
                unknown => return Err(ParseError::UnknownArgument(unknown.to_string())),
            };

            if !commands.contains(&command) {
                commands.push(command);
            }
        }

        let command = match commands.as_slice() {
            [] => Command::Tui,
            [command] => *command,
            _ => {
                return Err(ParseError::ConflictingCommands(
                    commands
                        .iter()
                        .filter_map(|command| command.flag())
                        .collect(),
                ))
            }
        };

        Ok(Cli { mode, command })
    }
}

/// Usage text for `--help`.
pub fn help_text() -> String {
    "\
headset-tui - HeadsetControl integration for Waybar

USAGE:
    headset-tui [OPTIONS]

COMMANDS:
    (none)              Launch the interactive TUI
    --waybar-status     Print the Waybar JSON for the battery module
    --toggle-sidetone   Toggle sidetone between off and the default level
    --config            Edit keybindings and default sidetone
    -h, --help          Show this help

OPTIONS:
    --test              Run the whole tool simulated: HeadsetControl's mock
                        device (--test-device), a separate config file
                        (config.test.toml) and a separate sidetone state file.
                        Works with every command, e.g.
                        `headset-tui --test`, `headset-tui --waybar-status --test`
                        or `headset-tui --config --test`.

NOTE:
    --test requires HeadsetControl with the --test-device option.
    No headset needs to be connected."
        .to_string()
}
