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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every flag that selects a command, in the order the help text lists them.
    const COMMAND_FLAGS: [&str; 4] = ["--waybar-status", "--toggle-sidetone", "--config", "--help"];

    fn parse(args: &[&str]) -> Result<Cli, ParseError> {
        Cli::parse(args)
    }

    /// Every ordering of `items` (used to prove the parser is order independent).
    fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
        if items.len() <= 1 {
            return vec![items.to_vec()];
        }

        let mut result = Vec::new();
        for index in 0..items.len() {
            let mut rest = items.to_vec();
            let item = rest.remove(index);
            for mut permutation in permutations(&rest) {
                permutation.insert(0, item.clone());
                result.push(permutation);
            }
        }
        result
    }

    // --- Mode -------------------------------------------------------------

    #[test]
    fn normal_mode_has_no_file_suffix() {
        assert_eq!(Mode::Normal.suffix(), "");
    }

    #[test]
    fn test_mode_uses_dot_test_suffix() {
        assert_eq!(Mode::Test.suffix(), ".test");
    }

    #[test]
    fn normal_mode_is_not_test() {
        assert!(!Mode::Normal.is_test());
    }

    #[test]
    fn test_mode_is_test() {
        assert!(Mode::Test.is_test());
    }

    // --- Command::flag ----------------------------------------------------

    #[test]
    fn tui_has_no_flag() {
        assert_eq!(Command::Tui.flag(), None);
    }

    #[test]
    fn waybar_status_flag() {
        assert_eq!(Command::WaybarStatus.flag(), Some("--waybar-status"));
    }

    #[test]
    fn toggle_sidetone_flag() {
        assert_eq!(Command::ToggleSidetone.flag(), Some("--toggle-sidetone"));
    }

    #[test]
    fn config_flag() {
        assert_eq!(Command::Config.flag(), Some("--config"));
    }

    #[test]
    fn help_flag() {
        assert_eq!(Command::Help.flag(), Some("--help"));
    }

    #[test]
    fn command_flags_are_distinct() {
        let mut flags: Vec<&str> = [
            Command::Tui,
            Command::WaybarStatus,
            Command::ToggleSidetone,
            Command::Config,
            Command::Help,
        ]
        .into_iter()
        .filter_map(Command::flag)
        .collect();
        flags.sort_unstable();

        let unique = {
            let mut sorted = flags.clone();
            sorted.dedup();
            sorted
        };

        assert_eq!(flags, unique);
        assert_eq!(flags.len(), 4);
    }

    // --- Defaults and single flags ---------------------------------------

    #[test]
    fn empty_arguments_default_to_tui_in_normal_mode() {
        let cli = Cli::parse(std::iter::empty::<&str>()).unwrap();
        assert_eq!(cli.mode, Mode::Normal);
        assert_eq!(cli.command, Command::Tui);
    }

    #[test]
    fn long_help_flag_selects_help() {
        let cli = parse(&["--help"]).unwrap();
        assert_eq!(cli.command, Command::Help);
        assert_eq!(cli.mode, Mode::Normal);
    }

    #[test]
    fn short_help_flag_selects_help() {
        let cli = parse(&["-h"]).unwrap();
        assert_eq!(cli.command, Command::Help);
    }

    #[test]
    fn config_flag_selects_config() {
        let cli = parse(&["--config"]).unwrap();
        assert_eq!(cli.command, Command::Config);
    }

    #[test]
    fn toggle_sidetone_flag_selects_toggle_sidetone() {
        let cli = parse(&["--toggle-sidetone"]).unwrap();
        assert_eq!(cli.command, Command::ToggleSidetone);
    }

    #[test]
    fn waybar_status_flag_selects_waybar_status() {
        let cli = parse(&["--waybar-status"]).unwrap();
        assert_eq!(cli.command, Command::WaybarStatus);
    }

    #[test]
    fn every_command_flag_round_trips() {
        for flag in COMMAND_FLAGS {
            let cli = parse(&[flag]).unwrap();
            assert_eq!(cli.command.flag(), Some(flag));
            assert_eq!(cli.mode, Mode::Normal, "{flag}");
        }
    }

    // --- `--test` modifier ------------------------------------------------

    #[test]
    fn test_flag_alone_keeps_tui() {
        let cli = parse(&["--test"]).unwrap();
        assert_eq!(cli.mode, Mode::Test);
        assert_eq!(cli.command, Command::Tui);
    }

    #[test]
    fn test_flag_before_command() {
        let cli = parse(&["--test", "--config"]).unwrap();
        assert_eq!(cli.mode, Mode::Test);
        assert_eq!(cli.command, Command::Config);
    }

    #[test]
    fn test_flag_after_command() {
        let cli = parse(&["--config", "--test"]).unwrap();
        assert_eq!(cli.mode, Mode::Test);
        assert_eq!(cli.command, Command::Config);
    }

    #[test]
    fn test_flag_between_repeated_test_flags() {
        let cli = parse(&["--test", "--config", "--test"]).unwrap();
        assert_eq!(cli.mode, Mode::Test);
        assert_eq!(cli.command, Command::Config);
    }

    #[test]
    fn duplicate_test_flag_is_harmless() {
        let cli = parse(&["--test", "--test"]).unwrap();
        assert_eq!(cli.mode, Mode::Test);
        assert_eq!(cli.command, Command::Tui);
    }

    #[test]
    fn test_flag_combines_with_every_command_flag() {
        for flag in COMMAND_FLAGS {
            let cli = parse(&["--test", flag]).unwrap();
            assert_eq!(cli.mode, Mode::Test, "{flag}");
            assert_eq!(cli.command.flag(), Some(flag));
        }
    }

    #[test]
    fn test_flag_never_conflicts_with_a_command() {
        for flag in COMMAND_FLAGS {
            assert!(parse(&[flag, "--test"]).is_ok(), "{flag}");
        }
    }

    // --- De-duplication ---------------------------------------------------

    #[test]
    fn duplicate_command_flag_is_deduplicated() {
        let cli = parse(&["--config", "--config"]).unwrap();
        assert_eq!(cli.command, Command::Config);
    }

    #[test]
    fn short_and_long_help_together_are_deduplicated() {
        let cli = parse(&["-h", "--help"]).unwrap();
        assert_eq!(cli.command, Command::Help);
    }

    #[test]
    fn long_and_short_help_together_are_deduplicated() {
        let cli = parse(&["--help", "-h"]).unwrap();
        assert_eq!(cli.command, Command::Help);
    }

    #[test]
    fn every_duplicated_command_flag_is_deduplicated() {
        for flag in COMMAND_FLAGS {
            let cli = parse(&[flag, flag, flag]).unwrap();
            assert_eq!(cli.command.flag(), Some(flag));
        }
    }

    // --- Unknown arguments ------------------------------------------------

    #[test]
    fn unknown_long_argument_is_rejected() {
        assert_eq!(
            parse(&["--bogus"]),
            Err(ParseError::UnknownArgument("--bogus".to_string()))
        );
    }

    #[test]
    fn unknown_short_argument_is_rejected() {
        assert_eq!(
            parse(&["-x"]),
            Err(ParseError::UnknownArgument("-x".to_string()))
        );
    }

    #[test]
    fn bare_positional_argument_is_rejected() {
        assert_eq!(
            parse(&["status"]),
            Err(ParseError::UnknownArgument("status".to_string()))
        );
    }

    #[test]
    fn test_device_flag_is_not_a_program_argument() {
        assert_eq!(
            parse(&["--test-device"]),
            Err(ParseError::UnknownArgument("--test-device".to_string()))
        );
    }

    #[test]
    fn empty_string_argument_is_rejected() {
        assert_eq!(
            parse(&[""]),
            Err(ParseError::UnknownArgument(String::new()))
        );
    }

    #[test]
    fn unknown_argument_is_preserved_verbatim() {
        for argument in [" --help", "--help ", "--HELP", "--Test", "help", "-H"] {
            assert_eq!(
                parse(&[argument]),
                Err(ParseError::UnknownArgument(argument.to_string())),
                "{argument}"
            );
        }
    }

    #[test]
    fn unknown_argument_after_a_valid_command_is_rejected() {
        assert_eq!(
            parse(&["--config", "--bogus"]),
            Err(ParseError::UnknownArgument("--bogus".to_string()))
        );
    }

    #[test]
    fn unknown_argument_before_a_valid_command_is_rejected() {
        assert_eq!(
            parse(&["--bogus", "--config"]),
            Err(ParseError::UnknownArgument("--bogus".to_string()))
        );
    }

    #[test]
    fn unknown_argument_takes_precedence_over_conflicting_commands() {
        assert_eq!(
            parse(&["--config", "--waybar-status", "--bogus"]),
            Err(ParseError::UnknownArgument("--bogus".to_string()))
        );
    }

    #[test]
    fn unknown_argument_wins_even_when_it_follows_test_flag() {
        assert_eq!(
            parse(&["--test", "--bogus"]),
            Err(ParseError::UnknownArgument("--bogus".to_string()))
        );
    }

    // --- Conflicting commands --------------------------------------------

    #[test]
    fn two_command_flags_conflict() {
        assert_eq!(
            parse(&["--config", "--waybar-status"]),
            Err(ParseError::ConflictingCommands(vec![
                "--config",
                "--waybar-status"
            ]))
        );
    }

    #[test]
    fn conflict_keeps_first_seen_order() {
        assert_eq!(
            parse(&["--waybar-status", "--config"]),
            Err(ParseError::ConflictingCommands(vec![
                "--waybar-status",
                "--config"
            ]))
        );
    }

    #[test]
    fn help_conflicts_with_other_commands() {
        assert_eq!(
            parse(&["--help", "--toggle-sidetone"]),
            Err(ParseError::ConflictingCommands(vec![
                "--help",
                "--toggle-sidetone"
            ]))
        );
    }

    #[test]
    fn short_help_reports_the_long_flag_in_a_conflict() {
        assert_eq!(
            parse(&["-h", "--config"]),
            Err(ParseError::ConflictingCommands(vec!["--help", "--config"]))
        );
    }

    #[test]
    fn conflict_lists_every_requested_command() {
        assert_eq!(
            parse(&["--config", "--waybar-status", "--toggle-sidetone"]),
            Err(ParseError::ConflictingCommands(vec![
                "--config",
                "--waybar-status",
                "--toggle-sidetone"
            ]))
        );
    }

    #[test]
    fn conflict_does_not_list_the_test_modifier() {
        match parse(&["--test", "--config", "--waybar-status"]) {
            Err(ParseError::ConflictingCommands(flags)) => {
                assert_eq!(flags, vec!["--config", "--waybar-status"]);
                assert!(!flags.contains(&"--test"));
            }
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    #[test]
    fn conflict_display_lists_the_flags_in_order() {
        let error = parse(&["--config", "--waybar-status"]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "conflicting commands: --config, --waybar-status"
        );
    }

    // --- ParseError Display ----------------------------------------------

    #[test]
    fn unknown_argument_display_quotes_the_argument() {
        let error = ParseError::UnknownArgument("--bogus".to_string());
        assert_eq!(error.to_string(), "unknown argument '--bogus'");
    }

    #[test]
    fn unknown_argument_display_keeps_the_raw_text() {
        let error = ParseError::UnknownArgument(" --bogus ".to_string());
        assert_eq!(error.to_string(), "unknown argument ' --bogus '");
    }

    #[test]
    fn unknown_empty_argument_display() {
        let error = ParseError::UnknownArgument(String::new());
        assert_eq!(error.to_string(), "unknown argument ''");
    }

    #[test]
    fn conflicting_commands_display_for_three_flags() {
        let error = ParseError::ConflictingCommands(vec![
            "--config",
            "--waybar-status",
            "--toggle-sidetone",
        ]);
        assert_eq!(
            error.to_string(),
            "conflicting commands: --config, --waybar-status, --toggle-sidetone"
        );
    }

    #[test]
    fn parse_error_implements_the_error_trait() {
        let error: Box<dyn std::error::Error> = Box::new(ParseError::UnknownArgument("x".into()));
        assert_eq!(error.to_string(), "unknown argument 'x'");
    }

    // --- Accepted input shapes -------------------------------------------

    #[test]
    fn parse_accepts_owned_strings() {
        let args = vec!["--test".to_string(), "--waybar-status".to_string()];
        let cli = Cli::parse(args).unwrap();
        assert_eq!(cli.mode, Mode::Test);
        assert_eq!(cli.command, Command::WaybarStatus);
    }

    #[test]
    fn parse_accepts_an_arbitrary_iterator() {
        let cli = Cli::parse(["--config", "--test"].into_iter().map(str::to_string)).unwrap();
        assert_eq!(cli.mode, Mode::Test);
        assert_eq!(cli.command, Command::Config);
    }

    // --- Order independence ----------------------------------------------

    #[test]
    fn help_flag_order_does_not_matter() {
        for permutation in permutations(&["-h", "--help"]) {
            let cli = parse(&permutation).unwrap();
            assert_eq!(cli.command, Command::Help, "{permutation:?}");
        }
    }

    #[test]
    fn test_modifier_position_does_not_matter() {
        for permutation in permutations(&["--test", "--config"]) {
            let cli = parse(&permutation).unwrap();
            assert_eq!(cli.mode, Mode::Test, "{permutation:?}");
            assert_eq!(cli.command, Command::Config, "{permutation:?}");
        }
    }

    #[test]
    fn repeated_identical_arguments_are_order_independent() {
        for permutation in permutations(&["--test", "--toggle-sidetone", "--test"]) {
            let cli = parse(&permutation).unwrap();
            assert_eq!(cli.mode, Mode::Test, "{permutation:?}");
            assert_eq!(cli.command, Command::ToggleSidetone, "{permutation:?}");
        }
    }

    // --- Help text --------------------------------------------------------

    #[test]
    fn help_text_starts_with_the_binary_name() {
        assert!(help_text().starts_with("headset-tui - HeadsetControl integration for Waybar"));
    }

    #[test]
    fn help_text_includes_the_usage_header() {
        assert!(help_text().contains("USAGE:"));
        assert!(help_text().contains("headset-tui [OPTIONS]"));
    }

    #[test]
    fn help_text_mentions_every_command_flag() {
        let text = help_text();
        for flag in COMMAND_FLAGS {
            assert!(text.contains(flag), "help text does not mention {flag}");
        }
    }

    #[test]
    fn help_text_mentions_the_short_help_flag() {
        assert!(help_text().contains("-h, --help"));
    }

    #[test]
    fn help_text_mentions_the_test_modifier() {
        assert!(help_text().contains("--test"));
    }

    #[test]
    fn help_text_explains_that_test_mode_is_simulated() {
        assert!(help_text().contains("simulated"));
    }

    #[test]
    fn help_text_mentions_the_mock_device_option() {
        assert!(help_text().contains("--test-device"));
    }

    #[test]
    fn help_text_documents_the_tui_default() {
        assert!(help_text().contains("Launch the interactive TUI"));
    }

    #[test]
    fn help_text_does_not_use_the_old_binary_name() {
        assert!(!help_text().contains("wb-headset"));
    }
}
