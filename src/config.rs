//! Configuration and file locations.

use crate::cli::Mode;
use crate::headset::SIDETONE_MAX;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// Keybindings.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct KeyConfig {
    pub dec1: char,
    pub dec10: char,
    pub inc1: char,
    pub inc10: char,
    pub full: char,
    pub none: char,
    pub quit: char,
    pub config: char,
    pub verbose: char,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            dec1: 'a',
            dec10: 's',
            inc1: 'd',
            inc10: 'w',
            full: 'f',
            none: 'e',
            quit: 'q',
            config: 'c',
            verbose: 'v',
        }
    }
}

/// Persisted configuration.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub keys: KeyConfig,
    #[serde(default)]
    pub default_sidetone: u8,
}

/// `~/.config/headset-tui`.
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config/headset-tui")
}

/// `~/.cache`.
pub fn cache_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".cache")
}

/// Path builder for a config file inside `dir`.
pub fn config_path_in(dir: &Path, mode: Mode) -> PathBuf {
    dir.join(format!("config{}.toml", mode.suffix()))
}

/// Path builder for a sidetone state file inside `dir`.
pub fn state_path_in(dir: &Path, mode: Mode) -> PathBuf {
    dir.join(format!("headset-tui-sidetone-state{}", mode.suffix()))
}

/// Config file used at runtime.
///
/// Test mode uses `config.test.toml`, so `--config --test` never touches the
/// real configuration.
pub fn config_path(mode: Mode) -> PathBuf {
    config_path_in(&config_dir(), mode)
}

/// Sidetone toggle state file used at runtime.
pub fn state_path(mode: Mode) -> PathBuf {
    state_path_in(&cache_dir(), mode)
}

/// Load the configuration, creating it with defaults when it does not exist.
pub fn load(mode: Mode) -> Config {
    load_or_create(&config_path(mode))
}

/// Load a config file, creating it with defaults when missing or unreadable.
pub fn load_or_create(path: &Path) -> Config {
    if let Some(config) = read(path) {
        return config;
    }

    let config = Config::default();
    let _ = save_to_path(path, &config);
    config
}

/// Read a config file, or `None` when it is missing or invalid.
pub fn read(path: &Path) -> Option<Config> {
    let content = fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

/// Write a config file, creating its directory when missing.
pub fn save_to_path(path: &Path, config: &Config) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(config).map_err(io::Error::other)?;

    fs::write(path, content)
}

/// Interactively edit the configuration for `mode`.
pub fn interactive_config(
    mode: Mode,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<()> {
    interactive_config_at(&config_path(mode), input, output)
}

/// Interactively edit the configuration stored at `path`.
///
/// Reads answers line by line, so the whole dialog can be exercised in tests.
/// End of input discards the changes instead of looping forever.
pub fn interactive_config_at(
    path: &Path,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<()> {
    let mut config = read(path).unwrap_or_default();

    writeln!(output, "\n=== Headset Control Configuration ===\n")?;

    loop {
        write!(
            output,
            "Default sidetone level (0-128): [{}] ",
            config.default_sidetone
        )?;
        output.flush()?;

        let Some(line) = read_line(input)? else {
            writeln!(output, "\nInput closed, changes discarded")?;
            return Ok(());
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }

        match trimmed.parse::<u8>() {
            Ok(level) if level <= SIDETONE_MAX => {
                config.default_sidetone = level;
                break;
            }
            Ok(_) => writeln!(output, "Please enter a value between 0 and 128")?,
            Err(_) => writeln!(output, "Invalid input")?,
        }
    }

    writeln!(output, "\n=== Key Bindings ===\n")?;

    let keys_to_configure = [
        ("dec1", "Decrease sidetone by 1", &mut config.keys.dec1),
        ("dec10", "Decrease sidetone by 10", &mut config.keys.dec10),
        ("inc1", "Increase sidetone by 1", &mut config.keys.inc1),
        ("inc10", "Increase sidetone by 10", &mut config.keys.inc10),
        ("full", "Set sidetone to full (128)", &mut config.keys.full),
        ("none", "Set sidetone to none (0)", &mut config.keys.none),
        ("config", "Open configuration menu", &mut config.keys.config),
        ("verbose", "Toggle verbose mode", &mut config.keys.verbose),
        ("quit", "Quit the application", &mut config.keys.quit),
    ];

    for (key_name, description, key) in keys_to_configure {
        write!(output, "{description} ({key_name}): [{}] ", *key)?;
        output.flush()?;

        let Some(line) = read_line(input)? else {
            writeln!(output, "\nInput closed, changes discarded")?;
            return Ok(());
        };

        let trimmed = line.trim();
        if let Some(character) = trimmed.chars().next() {
            *key = character.to_lowercase().next().unwrap_or(character);
        }
    }

    writeln!(output, "\nSave changes? (w - write, esc - discard)")?;

    loop {
        write!(output, "> ")?;
        output.flush()?;

        let Some(line) = read_line(input)? else {
            writeln!(output, "Changes discarded")?;
            return Ok(());
        };

        match line.trim().to_lowercase().as_str() {
            "w" => {
                save_to_path(path, &config)?;
                writeln!(output, "Configuration saved to {}", path.display())?;
                return Ok(());
            }
            "" | "esc" => {
                writeln!(output, "Changes discarded")?;
                return Ok(());
            }
            _ => {}
        }
    }
}

/// Read one line; `None` signals end of input.
fn read_line(input: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut line = String::new();

    if input.read_line(&mut line)? == 0 {
        Ok(None)
    } else {
        Ok(Some(line))
    }
}
