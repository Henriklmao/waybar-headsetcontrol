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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headset::mock::MockHeadset;
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        tempfile::tempdir().expect("create temp dir")
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).expect("read state file")
    }

    fn status(has_sidetone: bool) -> HeadsetStatus {
        HeadsetStatus {
            device_name: "HyperX Cloud".to_string(),
            battery_level: 73,
            battery_state: BatteryState::Good,
            has_sidetone,
        }
    }

    fn ui_state(sidetone: u8) -> UiState {
        UiState::new(sidetone, Mode::Normal)
    }

    // -- UiState::new ------------------------------------------------------

    #[test]
    fn new_state_has_expected_defaults() {
        let state = UiState::new(37, Mode::Normal);

        assert_eq!(state.sidetone, 37);
        assert_eq!(state.battery, None);
        assert_eq!(state.battery_state, BatteryState::Unavailable);
        assert_eq!(state.device_name, "");
        assert!(state.has_sidetone);
        assert!(!state.verbose_mode);
        assert_eq!(state.mode, Mode::Normal);
    }

    #[test]
    fn new_state_clamps_sidetone_above_max() {
        assert_eq!(UiState::new(200, Mode::Normal).sidetone, SIDETONE_MAX);
        assert_eq!(UiState::new(129, Mode::Normal).sidetone, SIDETONE_MAX);
    }

    #[test]
    fn new_state_keeps_levels_at_or_below_max() {
        assert_eq!(UiState::new(128, Mode::Normal).sidetone, 128);
        assert_eq!(UiState::new(64, Mode::Normal).sidetone, 64);
        assert_eq!(UiState::new(0, Mode::Normal).sidetone, 0);
    }

    #[test]
    fn new_state_stores_mode() {
        assert_eq!(UiState::new(10, Mode::Test).mode, Mode::Test);
    }

    // -- apply_status ------------------------------------------------------

    #[test]
    fn apply_status_copies_device_name() {
        let mut state = ui_state(10);

        apply_status(&mut state, Ok(status(true)));

        assert_eq!(state.device_name, "HyperX Cloud");
    }

    #[test]
    fn apply_status_copies_battery_level_and_state() {
        let mut state = ui_state(10);

        apply_status(&mut state, Ok(status(true)));

        assert_eq!(state.battery, Some(73));
        assert_eq!(state.battery_state, BatteryState::Good);
    }

    #[test]
    fn apply_status_copies_sidetone_capability_true() {
        let mut state = ui_state(10);

        apply_status(&mut state, Ok(status(true)));

        assert!(state.has_sidetone);
    }

    #[test]
    fn apply_status_copies_sidetone_capability_false() {
        let mut state = ui_state(10);

        apply_status(&mut state, Ok(status(false)));

        assert!(!state.has_sidetone);
    }

    #[test]
    fn apply_status_error_reports_unknown_device() {
        let mut state = ui_state(10);

        apply_status(&mut state, Err(HeadsetError::NoDevice));

        assert_eq!(state.device_name, "Unknown Device");
    }

    #[test]
    fn apply_status_error_clears_battery() {
        let mut state = ui_state(10);

        apply_status(&mut state, Err(HeadsetError::NoDevice));

        assert_eq!(state.battery, None);
        assert_eq!(state.battery_state, BatteryState::Unavailable);
    }

    #[test]
    fn apply_status_error_disables_sidetone() {
        let mut state = ui_state(10);

        apply_status(&mut state, Err(HeadsetError::CommandFailed(1)));

        assert!(!state.has_sidetone);
    }

    #[test]
    fn apply_status_error_keeps_sidetone_level_and_mode() {
        let mut state = UiState::new(55, Mode::Test);

        apply_status(&mut state, Err(HeadsetError::Parse("oops".to_string())));

        assert_eq!(state.sidetone, 55);
        assert_eq!(state.mode, Mode::Test);
    }

    #[test]
    fn apply_status_success_after_error_recovers() {
        let mut state = ui_state(10);
        apply_status(&mut state, Err(HeadsetError::NoDevice));

        apply_status(&mut state, Ok(status(true)));

        assert_eq!(state.device_name, "HyperX Cloud");
        assert_eq!(state.battery, Some(73));
        assert_eq!(state.battery_state, BatteryState::Good);
        assert!(state.has_sidetone);
    }

    #[test]
    fn apply_status_error_after_success_clears_fields() {
        let mut state = ui_state(10);
        apply_status(&mut state, Ok(status(true)));

        apply_status(
            &mut state,
            Err(HeadsetError::Unavailable("gone".to_string())),
        );

        assert_eq!(state.battery, None);
        assert_eq!(state.battery_state, BatteryState::Unavailable);
        assert_eq!(state.device_name, "Unknown Device");
        assert!(!state.has_sidetone);
    }

    // -- apply_action: level changes ---------------------------------------

    #[test]
    fn dec_by_1_lowers_level() {
        let headset = MockHeadset::available();
        let mut state = ui_state(10);

        let outcome = apply_action(Action::DecBy1, &mut state, &headset);

        assert_eq!(outcome, Outcome::Continue);
        assert_eq!(state.sidetone, 9);
        assert_eq!(headset.calls(), vec![9]);
    }

    #[test]
    fn dec_by_10_lowers_level() {
        let headset = MockHeadset::available();
        let mut state = ui_state(60);

        apply_action(Action::DecBy10, &mut state, &headset);

        assert_eq!(state.sidetone, 50);
        assert_eq!(headset.calls(), vec![50]);
    }

    #[test]
    fn inc_by_1_raises_level() {
        let headset = MockHeadset::available();
        let mut state = ui_state(10);

        apply_action(Action::IncBy1, &mut state, &headset);

        assert_eq!(state.sidetone, 11);
        assert_eq!(headset.calls(), vec![11]);
    }

    #[test]
    fn inc_by_10_raises_level() {
        let headset = MockHeadset::available();
        let mut state = ui_state(20);

        apply_action(Action::IncBy10, &mut state, &headset);

        assert_eq!(state.sidetone, 30);
        assert_eq!(headset.calls(), vec![30]);
    }

    #[test]
    fn dec_by_10_from_five_clamps_to_zero() {
        let headset = MockHeadset::available();
        let mut state = ui_state(5);

        apply_action(Action::DecBy10, &mut state, &headset);

        assert_eq!(state.sidetone, 0);
        assert_eq!(headset.calls(), vec![0]);
    }

    #[test]
    fn inc_by_10_from_120_clamps_to_max() {
        let headset = MockHeadset::available();
        let mut state = ui_state(120);

        apply_action(Action::IncBy10, &mut state, &headset);

        assert_eq!(state.sidetone, SIDETONE_MAX);
        assert_eq!(headset.calls(), vec![SIDETONE_MAX]);
    }

    #[test]
    fn inc_by_1_from_127_clamps_to_max() {
        let headset = MockHeadset::available();
        let mut state = ui_state(127);

        apply_action(Action::IncBy1, &mut state, &headset);

        assert_eq!(state.sidetone, 128);
        assert_eq!(headset.calls(), vec![128]);
    }

    #[test]
    fn dec_by_1_at_zero_does_not_call_headset() {
        let headset = MockHeadset::available();
        let mut state = ui_state(0);

        let outcome = apply_action(Action::DecBy1, &mut state, &headset);

        assert_eq!(outcome, Outcome::Continue);
        assert_eq!(state.sidetone, 0);
        assert!(headset.calls().is_empty());
    }

    #[test]
    fn inc_by_1_at_max_does_not_call_headset() {
        let headset = MockHeadset::available();
        let mut state = ui_state(SIDETONE_MAX);

        apply_action(Action::IncBy1, &mut state, &headset);

        assert_eq!(state.sidetone, SIDETONE_MAX);
        assert!(headset.calls().is_empty());
    }

    #[test]
    fn inc_by_10_at_max_does_not_call_headset() {
        let headset = MockHeadset::available();
        let mut state = ui_state(SIDETONE_MAX);

        apply_action(Action::IncBy10, &mut state, &headset);

        assert_eq!(state.sidetone, SIDETONE_MAX);
        assert!(headset.calls().is_empty());
    }

    #[test]
    fn dec_by_10_at_zero_does_not_call_headset() {
        let headset = MockHeadset::available();
        let mut state = ui_state(0);

        apply_action(Action::DecBy10, &mut state, &headset);

        assert_eq!(state.sidetone, 0);
        assert!(headset.calls().is_empty());
    }

    // -- apply_action: absolute values -------------------------------------

    #[test]
    fn set_full_sets_max_level() {
        let headset = MockHeadset::available();
        let mut state = ui_state(12);

        let outcome = apply_action(Action::SetFull, &mut state, &headset);

        assert_eq!(outcome, Outcome::Continue);
        assert_eq!(state.sidetone, SIDETONE_MAX);
        assert_eq!(headset.calls(), vec![SIDETONE_MAX]);
    }

    #[test]
    fn set_full_at_max_does_not_call_headset() {
        let headset = MockHeadset::available();
        let mut state = ui_state(SIDETONE_MAX);

        apply_action(Action::SetFull, &mut state, &headset);

        assert_eq!(state.sidetone, SIDETONE_MAX);
        assert!(headset.calls().is_empty());
    }

    #[test]
    fn set_none_sets_zero() {
        let headset = MockHeadset::available();
        let mut state = ui_state(12);

        let outcome = apply_action(Action::SetNone, &mut state, &headset);

        assert_eq!(outcome, Outcome::Continue);
        assert_eq!(state.sidetone, 0);
        assert_eq!(headset.calls(), vec![0]);
    }

    #[test]
    fn set_none_at_zero_does_not_call_headset() {
        let headset = MockHeadset::available();
        let mut state = ui_state(0);

        apply_action(Action::SetNone, &mut state, &headset);

        assert_eq!(state.sidetone, 0);
        assert!(headset.calls().is_empty());
    }

    // -- apply_action: recording and failure behaviour ---------------------

    #[test]
    fn every_changed_level_is_written_in_order() {
        let headset = MockHeadset::available();
        let mut state = ui_state(50);

        apply_action(Action::IncBy10, &mut state, &headset);
        apply_action(Action::DecBy1, &mut state, &headset);
        apply_action(Action::DecBy10, &mut state, &headset);
        apply_action(Action::SetFull, &mut state, &headset);
        apply_action(Action::SetNone, &mut state, &headset);

        assert_eq!(state.sidetone, 0);
        assert_eq!(headset.calls(), vec![60, 59, 49, 128, 0]);
    }

    #[test]
    fn backend_failure_still_updates_state() {
        let mut headset = MockHeadset::available();
        headset.fail_set = true;
        let mut state = ui_state(10);

        let outcome = apply_action(Action::DecBy1, &mut state, &headset);

        assert_eq!(outcome, Outcome::Continue);
        assert_eq!(state.sidetone, 9);
        assert_eq!(headset.calls(), vec![9]);
    }

    #[test]
    fn backend_failure_still_updates_state_for_absolute_actions() {
        let mut headset = MockHeadset::available();
        headset.fail_set = true;
        let mut state = ui_state(10);

        apply_action(Action::SetFull, &mut state, &headset);

        assert_eq!(state.sidetone, 128);
        assert_eq!(headset.calls(), vec![128]);
    }

    // -- apply_action: non-level actions -----------------------------------

    #[test]
    fn toggle_verbose_flips_flag_and_continues() {
        let headset = MockHeadset::available();
        let mut state = ui_state(10);

        assert_eq!(
            apply_action(Action::ToggleVerbose, &mut state, &headset),
            Outcome::Continue
        );
        assert!(state.verbose_mode);

        assert_eq!(
            apply_action(Action::ToggleVerbose, &mut state, &headset),
            Outcome::Continue
        );
        assert!(!state.verbose_mode);

        assert!(headset.calls().is_empty());
        assert_eq!(state.sidetone, 10);
    }

    #[test]
    fn config_opens_config_without_backend_calls() {
        let headset = MockHeadset::available();
        let mut state = ui_state(10);

        let outcome = apply_action(Action::Config, &mut state, &headset);

        assert_eq!(outcome, Outcome::OpenConfig);
        assert!(headset.calls().is_empty());
        assert_eq!(state, ui_state(10));
    }

    #[test]
    fn quit_returns_quit_without_backend_calls() {
        let headset = MockHeadset::available();
        let mut state = ui_state(10);

        let outcome = apply_action(Action::Quit, &mut state, &headset);

        assert_eq!(outcome, Outcome::Quit);
        assert!(headset.calls().is_empty());
        assert_eq!(state, ui_state(10));
    }

    #[test]
    fn none_continues_without_changing_state() {
        let headset = MockHeadset::available();
        let mut state = ui_state(42);
        let before = state.clone();

        let outcome = apply_action(Action::None, &mut state, &headset);

        assert_eq!(outcome, Outcome::Continue);
        assert_eq!(state, before);
        assert!(headset.calls().is_empty());
    }

    // -- toggle_state ------------------------------------------------------

    #[test]
    fn toggle_state_missing_file_is_off() {
        let dir = temp_dir();

        assert!(!toggle_state(&dir.path().join("missing")));
    }

    #[test]
    fn toggle_state_on_is_true() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "on").unwrap();

        assert!(toggle_state(&path));
    }

    #[test]
    fn toggle_state_off_is_false() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "off").unwrap();

        assert!(!toggle_state(&path));
    }

    #[test]
    fn toggle_state_ignores_surrounding_whitespace_and_newlines() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "  on\n\t ").unwrap();

        assert!(toggle_state(&path));
    }

    #[test]
    fn toggle_state_unrelated_content_is_false() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "maybe").unwrap();

        assert!(!toggle_state(&path));
    }

    #[test]
    fn toggle_state_empty_file_is_false() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "").unwrap();

        assert!(!toggle_state(&path));
    }

    #[test]
    fn toggle_state_is_case_sensitive() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "ON").unwrap();

        assert!(!toggle_state(&path));
    }

    // -- toggle_sidetone ---------------------------------------------------

    #[test]
    fn toggle_sidetone_without_default_returns_off_when_missing() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        let headset = MockHeadset::available();

        let result = toggle_sidetone(&headset, 0, &path).unwrap();

        assert!(!result);
        assert!(headset.calls().is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn toggle_sidetone_without_default_returns_on_and_keeps_file() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "on").unwrap();
        let headset = MockHeadset::available();

        let result = toggle_sidetone(&headset, 0, &path).unwrap();

        assert!(result);
        assert!(headset.calls().is_empty());
        assert_eq!(read(&path), "on");
    }

    #[test]
    fn toggle_sidetone_when_off_writes_default_and_marks_on() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "off").unwrap();
        let headset = MockHeadset::available();

        let result = toggle_sidetone(&headset, 50, &path).unwrap();

        assert!(result);
        assert_eq!(headset.calls(), vec![50]);
        assert_eq!(read(&path), "on");
    }

    #[test]
    fn toggle_sidetone_when_missing_writes_default_and_marks_on() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        let headset = MockHeadset::available();

        let result = toggle_sidetone(&headset, 64, &path).unwrap();

        assert!(result);
        assert_eq!(headset.calls(), vec![64]);
        assert_eq!(read(&path), "on");
    }

    #[test]
    fn toggle_sidetone_when_on_writes_zero_and_marks_off() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "on").unwrap();
        let headset = MockHeadset::available();

        let result = toggle_sidetone(&headset, 50, &path).unwrap();

        assert!(!result);
        assert_eq!(headset.calls(), vec![0]);
        assert_eq!(read(&path), "off");
    }

    #[test]
    fn toggle_sidetone_twice_returns_to_start() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        let headset = MockHeadset::available();

        assert!(toggle_sidetone(&headset, 60, &path).unwrap());
        assert!(!toggle_sidetone(&headset, 60, &path).unwrap());
        assert!(toggle_sidetone(&headset, 60, &path).unwrap());

        assert_eq!(headset.calls(), vec![60, 0, 60]);
        assert_eq!(read(&path), "on");
    }

    #[test]
    fn toggle_sidetone_clamps_default_above_max() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        let headset = MockHeadset::available();

        toggle_sidetone(&headset, 200, &path).unwrap();

        assert_eq!(headset.calls(), vec![SIDETONE_MAX]);
        assert_eq!(read(&path), "on");
    }

    #[test]
    fn toggle_sidetone_backend_failure_returns_error_for_off() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "off").unwrap();
        let mut headset = MockHeadset::available();
        headset.fail_set = true;

        let error = toggle_sidetone(&headset, 40, &path).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(headset.calls(), vec![40]);
        assert_eq!(read(&path), "off");
    }

    #[test]
    fn toggle_sidetone_backend_failure_keeps_on_state() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "on").unwrap();
        let mut headset = MockHeadset::available();
        headset.fail_set = true;

        assert!(toggle_sidetone(&headset, 40, &path).is_err());
        assert_eq!(read(&path), "on");
    }

    #[test]
    fn toggle_sidetone_creates_missing_parent_directories() {
        let dir = temp_dir();
        let path = dir.path().join("nested").join("deeper").join("state");
        let headset = MockHeadset::available();

        let result = toggle_sidetone(&headset, 30, &path).unwrap();

        assert!(result);
        assert!(path.exists());
        assert_eq!(read(&path), "on");
    }

    // -- save_toggle_state -------------------------------------------------

    #[test]
    fn save_toggle_state_writes_on() {
        let dir = temp_dir();
        let path = dir.path().join("state");

        save_toggle_state(&path, true).unwrap();

        assert_eq!(read(&path), "on");
    }

    #[test]
    fn save_toggle_state_writes_off() {
        let dir = temp_dir();
        let path = dir.path().join("state");

        save_toggle_state(&path, false).unwrap();

        assert_eq!(read(&path), "off");
    }

    #[test]
    fn save_toggle_state_creates_nested_directories() {
        let dir = temp_dir();
        let path = dir.path().join("a").join("b").join("c").join("state");

        save_toggle_state(&path, true).unwrap();

        assert!(path.exists());
        assert_eq!(read(&path), "on");
    }

    #[test]
    fn save_toggle_state_overwrites_existing_content() {
        let dir = temp_dir();
        let path = dir.path().join("state");
        std::fs::write(&path, "on\n").unwrap();

        save_toggle_state(&path, false).unwrap();
        assert_eq!(read(&path), "off");

        save_toggle_state(&path, true).unwrap();
        assert_eq!(read(&path), "on");
    }
}
