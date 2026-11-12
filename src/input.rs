use crate::config::KeyConfig;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    DecBy1,
    DecBy10,
    IncBy1,
    IncBy10,
    SetFull,
    SetNone,
    ToggleVerbose,
    Config,
    Quit,
    None,
}

/// Poll the terminal for up to 100 ms and map a key press to an [`Action`].
pub fn handle_input(keys: &KeyConfig) -> Action {
    if event::poll(Duration::from_millis(100)).unwrap_or(false) {
        if let Ok(Event::Key(key)) = event::read() {
            return key_to_action(key, keys);
        }
    }
    Action::None
}

/// Map a key event to an action using the configured bindings.
pub fn key_to_action(key: KeyEvent, keys: &KeyConfig) -> Action {
    let KeyCode::Char(character) = key.code else {
        return if key.code == KeyCode::Esc {
            Action::Quit
        } else {
            Action::None
        };
    };

    let character = character.to_lowercase().next().unwrap_or(character);

    if character == keys.dec1 {
        Action::DecBy1
    } else if character == keys.dec10 {
        Action::DecBy10
    } else if character == keys.inc1 {
        Action::IncBy1
    } else if character == keys.inc10 {
        Action::IncBy10
    } else if character == keys.full {
        Action::SetFull
    } else if character == keys.none {
        Action::SetNone
    } else if character == keys.quit {
        Action::Quit
    } else if character == keys.verbose {
        Action::ToggleVerbose
    } else if character == keys.config {
        Action::Config
    } else {
        Action::None
    }
}
