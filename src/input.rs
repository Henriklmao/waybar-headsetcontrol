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
    let binding_matches =
        |binding: char| character == binding.to_lowercase().next().unwrap_or(binding);

    if binding_matches(keys.dec1) {
        Action::DecBy1
    } else if binding_matches(keys.dec10) {
        Action::DecBy10
    } else if binding_matches(keys.inc1) {
        Action::IncBy1
    } else if binding_matches(keys.inc10) {
        Action::IncBy10
    } else if binding_matches(keys.full) {
        Action::SetFull
    } else if binding_matches(keys.none) {
        Action::SetNone
    } else if binding_matches(keys.quit) {
        Action::Quit
    } else if binding_matches(keys.verbose) {
        Action::ToggleVerbose
    } else if binding_matches(keys.config) {
        Action::Config
    } else {
        Action::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn press(character: char) -> KeyEvent {
        key(KeyCode::Char(character))
    }

    fn custom_keys() -> KeyConfig {
        KeyConfig {
            dec1: 'j',
            dec10: 'k',
            inc1: 'l',
            inc10: 'i',
            full: 'o',
            none: 'u',
            quit: 'x',
            config: 'y',
            verbose: 'h',
        }
    }

    #[test]
    fn default_bindings_map_to_their_actions() {
        let keys = KeyConfig::default();
        let bindings = [
            ('a', Action::DecBy1),
            ('s', Action::DecBy10),
            ('d', Action::IncBy1),
            ('w', Action::IncBy10),
            ('f', Action::SetFull),
            ('e', Action::SetNone),
            ('q', Action::Quit),
            ('c', Action::Config),
            ('v', Action::ToggleVerbose),
        ];

        for (character, expected) in bindings {
            assert_eq!(
                key_to_action(press(character), &keys),
                expected,
                "{character:?} should map to {expected:?}"
            );
        }
    }

    #[test]
    fn uppercase_letters_map_like_lowercase() {
        let keys = KeyConfig::default();

        for (character, expected) in [
            ('A', Action::DecBy1),
            ('S', Action::DecBy10),
            ('D', Action::IncBy1),
            ('W', Action::IncBy10),
            ('F', Action::SetFull),
            ('E', Action::SetNone),
            ('Q', Action::Quit),
            ('C', Action::Config),
            ('V', Action::ToggleVerbose),
        ] {
            assert_eq!(key_to_action(press(character), &keys), expected);
        }
    }

    #[test]
    fn custom_bindings_are_used() {
        let keys = custom_keys();

        for (character, expected) in [
            ('j', Action::DecBy1),
            ('k', Action::DecBy10),
            ('l', Action::IncBy1),
            ('i', Action::IncBy10),
            ('o', Action::SetFull),
            ('u', Action::SetNone),
            ('x', Action::Quit),
            ('y', Action::Config),
            ('h', Action::ToggleVerbose),
        ] {
            assert_eq!(key_to_action(press(character), &keys), expected);
        }
    }

    #[test]
    fn old_default_letters_stop_mapping_after_rebinding() {
        let keys = custom_keys();

        for character in ['a', 's', 'd', 'w', 'f', 'e', 'q', 'c', 'v'] {
            assert_eq!(
                key_to_action(press(character), &keys),
                Action::None,
                "{character:?} is no longer bound"
            );
        }
    }

    #[test]
    fn unknown_printable_characters_map_to_none() {
        let keys = KeyConfig::default();

        for character in ['z', '0', '7', '?', '!', '/', '\n'] {
            assert_eq!(key_to_action(press(character), &keys), Action::None);
        }
    }

    #[test]
    fn escape_maps_to_quit() {
        assert_eq!(
            key_to_action(key(KeyCode::Esc), &KeyConfig::default()),
            Action::Quit
        );
        // Esc stays bound to Quit even when the quit character was rebound.
        assert_eq!(
            key_to_action(key(KeyCode::Esc), &custom_keys()),
            Action::Quit
        );
    }

    #[test]
    fn non_character_keys_map_to_none() {
        let keys = KeyConfig::default();

        for code in [
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::Backspace,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Delete,
            KeyCode::F(1),
            KeyCode::Insert,
            KeyCode::Home,
        ] {
            assert_eq!(
                key_to_action(key(code), &keys),
                Action::None,
                "{code:?} should not be a binding"
            );
        }
    }

    #[test]
    fn rebound_quit_key_replaces_q() {
        let keys = KeyConfig {
            quit: 'x',
            ..KeyConfig::default()
        };

        assert_eq!(key_to_action(press('q'), &keys), Action::None);
        assert_eq!(key_to_action(press('x'), &keys), Action::Quit);
        assert_eq!(key_to_action(press('X'), &keys), Action::Quit);
    }

    #[test]
    fn space_can_be_bound() {
        let keys = KeyConfig {
            full: ' ',
            ..KeyConfig::default()
        };

        assert_eq!(key_to_action(press(' '), &keys), Action::SetFull);
        // Space is not a default binding, so it is unused before rebinding.
        assert_eq!(
            key_to_action(press(' '), &KeyConfig::default()),
            Action::None
        );
    }

    #[test]
    fn unicode_character_can_be_bound() {
        let keys = KeyConfig {
            dec1: 'ä',
            inc1: 'ö',
            ..KeyConfig::default()
        };

        assert_eq!(key_to_action(press('ä'), &keys), Action::DecBy1);
        // The character is lowercased before matching, so the uppercase umlaut
        // works too.
        assert_eq!(key_to_action(press('Ä'), &keys), Action::DecBy1);
        assert_eq!(key_to_action(press('ö'), &keys), Action::IncBy1);
        assert_eq!(key_to_action(press('a'), &keys), Action::None);
    }

    #[test]
    fn uppercase_bindings_from_a_hand_edited_config_still_match() {
        // The interactive config lowercases input, but a config edited by hand
        // may contain uppercase characters.
        let keys = KeyConfig {
            dec1: 'A',
            quit: 'X',
            ..KeyConfig::default()
        };

        assert_eq!(key_to_action(press('a'), &keys), Action::DecBy1);
        assert_eq!(key_to_action(press('A'), &keys), Action::DecBy1);
        assert_eq!(key_to_action(press('x'), &keys), Action::Quit);
        assert_eq!(key_to_action(press('X'), &keys), Action::Quit);
    }

    #[test]
    fn modifiers_do_not_change_the_mapped_action() {
        // `key_to_action` only looks at `key.code`; every modifier combination
        // maps to the same action as the unmodified character. (A real terminal
        // still reports Shift+A as `KeyCode::Char('A')`, which is also handled.)
        let keys = KeyConfig::default();

        for modifiers in [
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::SHIFT,
            KeyModifiers::CONTROL | KeyModifiers::ALT,
            KeyModifiers::SUPER,
        ] {
            assert_eq!(
                key_to_action(KeyEvent::new(KeyCode::Char('a'), modifiers), &keys),
                Action::DecBy1
            );
            assert_eq!(
                key_to_action(KeyEvent::new(KeyCode::Char('Q'), modifiers), &keys),
                Action::Quit
            );
            assert_eq!(
                key_to_action(KeyEvent::new(KeyCode::Esc, modifiers), &keys),
                Action::Quit
            );
        }
    }

    #[test]
    fn duplicate_bindings_resolve_in_check_order() {
        // The lookup is a fixed if/else chain: DecBy1 is checked before Quit, so
        // binding the same character twice keeps the first action.
        let keys = KeyConfig {
            quit: 'a',
            ..KeyConfig::default()
        };

        assert_eq!(key_to_action(press('a'), &keys), Action::DecBy1);
    }
}
