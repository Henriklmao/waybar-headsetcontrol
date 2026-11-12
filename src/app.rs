//! Application state and the actions that mutate it.

use crate::cli::Mode;
use crate::headset::{BatteryState, Headset, HeadsetError, HeadsetStatus, SIDETONE_MAX};
use crate::input::Action;
use std::io;
use std::path::Path;

/// State rendered by the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiState {
    pub sidetone: u8,
    /// Battery percentage, or `None` when it cannot be read.
    pub battery: Option<i32>,
    pub battery_state: BatteryState,
    pub device_name: String,
    pub has_sidetone: bool,
    pub verbose_mode: bool,
    pub mode: Mode,
}

impl UiState {
    pub fn new(sidetone: u8, mode: Mode) -> Self {
        Self {
            sidetone: sidetone.min(SIDETONE_MAX),
            battery: None,
            battery_state: BatteryState::Unavailable,
            device_name: String::new(),
            has_sidetone: true,
            verbose_mode: false,
            mode,
        }
    }
}

/// What the event loop should do after an action was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Quit,
    OpenConfig,
}

/// Copy a status (or its absence) into the UI state.
pub fn apply_status(state: &mut UiState, result: Result<HeadsetStatus, HeadsetError>) {
    match result {
        Ok(status) => {
            state.device_name = status.device_name;
            state.battery = Some(status.battery_level);
            state.battery_state = status.battery_state;
            state.has_sidetone = status.has_sidetone;
        }
        Err(_) => {
            state.battery = None;
            state.battery_state = BatteryState::Unavailable;
            state.device_name = "Unknown Device".to_string();
            state.has_sidetone = false;
        }
    }
}

/// Apply one key action and report what the event loop should do next.
///
/// The sidetone shown in the UI always follows the key press, even when the
/// headset rejects the write: the gauge is a level selector, not a read-back of
/// the device (HeadsetControl cannot read sidetone on all devices).
pub fn apply_action(action: Action, state: &mut UiState, headset: &dyn Headset) -> Outcome {
    match action {
        Action::DecBy1 => shift(state, headset, -1),
        Action::DecBy10 => shift(state, headset, -10),
        Action::IncBy1 => shift(state, headset, 1),
        Action::IncBy10 => shift(state, headset, 10),
        Action::SetFull => set_level(state, headset, SIDETONE_MAX),
        Action::SetNone => set_level(state, headset, 0),
        Action::ToggleVerbose => state.verbose_mode = !state.verbose_mode,
        Action::Config => return Outcome::OpenConfig,
        Action::Quit => return Outcome::Quit,
        Action::None => {}
    }

    Outcome::Continue
}

fn shift(state: &mut UiState, headset: &dyn Headset, delta: i16) {
    let level = (i16::from(state.sidetone) + delta).clamp(0, i16::from(SIDETONE_MAX)) as u8;
    set_level(state, headset, level);
}

fn set_level(state: &mut UiState, headset: &dyn Headset, level: u8) {
    if state.sidetone == level {
        return;
    }

    let _ = headset.set_sidetone(level);
    state.sidetone = level;
}

/// Read whether sidetone is currently toggled on.
pub fn toggle_state(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|value| value.trim() == "on")
        .unwrap_or(false)
}

/// Toggle sidetone between off and `default_sidetone`.
///
/// Returns the new on/off state. A `default_sidetone` of 0 means "no default
/// configured", so nothing is changed and the previous state is returned.
pub fn toggle_sidetone(
    headset: &dyn Headset,
    default_sidetone: u8,
    state_path: &Path,
) -> io::Result<bool> {
    let was_on = toggle_state(state_path);

    if default_sidetone == 0 {
        return Ok(was_on);
    }

    let level = if was_on {
        0
    } else {
        default_sidetone.min(SIDETONE_MAX)
    };
    headset
        .set_sidetone(level)
        .map_err(|err| io::Error::other(err.to_string()))?;

    let now_on = level > 0;
    save_toggle_state(state_path, now_on)?;
    Ok(now_on)
}

/// Persist the toggle state, creating its directory when missing.
pub fn save_toggle_state(path: &Path, on: bool) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, if on { "on" } else { "off" })
}
