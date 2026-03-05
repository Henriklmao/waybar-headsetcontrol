use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;
use crate::config::KeyConfig;

#[derive(Debug, Clone, Copy)]
pub enum Action {
    DecBy1,
    DecBy10,
    IncBy1,
    IncBy10,
    SetFull,
    SetNone,
    Quit,
    None,
}

pub fn handle_input(keys: &KeyConfig) -> Action {
    if event::poll(Duration::from_millis(100)).unwrap_or(false) {
        if let Ok(Event::Key(key)) = event::read() {
            return key_to_action(key, keys);
        }
    }
    Action::None
}

fn key_to_action(key: KeyEvent, keys: &KeyConfig) -> Action {
    if let KeyCode::Char(c) = key.code {
        let lower = c.to_lowercase().next().unwrap_or(c);
        
        if lower == keys.dec1 {
            Action::DecBy1
        } else if lower == keys.dec10 {
            Action::DecBy10
        } else if lower == keys.inc1 {
            Action::IncBy1
        } else if lower == keys.inc10 {
            Action::IncBy10
        } else if lower == keys.full {
            Action::SetFull
        } else if lower == keys.none {
            Action::SetNone
        } else if lower == keys.quit {
            Action::Quit
        } else {
            Action::None
        }
    } else if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
        Action::Quit
    } else {
        Action::None
    }
}
