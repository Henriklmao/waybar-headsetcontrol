//! HeadsetControl protocol: device status, errors and backends.

use crate::cli::Mode;
use serde::Deserialize;
use std::fmt;
use std::process::{Command, Stdio};

/// Flag passed to `headsetcontrol` to make it use its built-in mock device.
pub const TEST_DEVICE_FLAG: &str = "--test-device";

/// Highest sidetone level HeadsetControl accepts.
pub const SIDETONE_MAX: u8 = 128;

/// Battery state as reported (or derived) by HeadsetControl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryState {
    Charging,
    Good,
    Medium,
    Low,
    Unavailable,
}

impl BatteryState {
    /// Map a HeadsetControl battery status and level onto a [`BatteryState`].
    pub fn from_headsetcontrol(status: &str, level: i32) -> Self {
        match status {
            "BATTERY_UNAVAILABLE" => BatteryState::Unavailable,
            "BATTERY_CHARGING" => BatteryState::Charging,
            _ if level > 50 => BatteryState::Good,
            _ if level >= 15 => BatteryState::Medium,
            _ => BatteryState::Low,
        }
    }
}

/// Everything the application needs to know about the connected headset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadsetStatus {
    pub device_name: String,
    pub battery_level: i32,
    pub battery_state: BatteryState,
    pub has_sidetone: bool,
}

/// Device access used by the application.
pub trait Headset {
    fn status(&self) -> Result<HeadsetStatus, HeadsetError>;
    fn set_sidetone(&self, level: u8) -> Result<(), HeadsetError>;
}

/// Backend that shells out to the `headsetcontrol` binary.
#[derive(Debug, Clone, Copy)]
pub struct HeadsetControl {
    mode: Mode,
}

impl HeadsetControl {
    pub fn new(mode: Mode) -> Self {
        Self { mode }
    }

    /// The command used for every HeadsetControl call.
    ///
    /// In test mode `--test-device` is prepended so HeadsetControl talks to its
    /// simulated device instead of real hardware.
    pub fn command(&self) -> Command {
        let mut command = Command::new("headsetcontrol");

        if self.mode.is_test() {
            command.arg(TEST_DEVICE_FLAG);
        }

        command
    }
}

impl Headset for HeadsetControl {
    fn status(&self) -> Result<HeadsetStatus, HeadsetError> {
        let output = self
            .command()
            .arg("-o")
            .arg("json")
            .output()
            .map_err(|err| HeadsetError::Unavailable(err.to_string()))?;

        // headsetcontrol exits non-zero on real errors (e.g. an older build
        // without `--test-device`); don't report those as a parse failure.
        if !output.status.success() {
            return Err(HeadsetError::CommandFailed(
                output.status.code().unwrap_or(-1),
            ));
        }

        parse_status(&output.stdout)
    }

    fn set_sidetone(&self, level: u8) -> Result<(), HeadsetError> {
        let status = self
            .command()
            .arg("-s")
            .arg(level.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|err| HeadsetError::Unavailable(err.to_string()))?;

        if status.success() {
            Ok(())
        } else {
            Err(HeadsetError::CommandFailed(status.code().unwrap_or(-1)))
        }
    }
}

/// Raw `headsetcontrol -o json` document (only the fields we consume).
#[derive(Debug, Deserialize)]
struct HeadsetControlOutput {
    devices: Vec<Device>,
}

#[derive(Debug, Deserialize)]
struct Device {
    product: String,
    #[serde(default)]
    capabilities_str: Vec<String>,
    battery: Battery,
}

#[derive(Debug, Deserialize)]
struct Battery {
    status: String,
    level: i32,
}

/// Parse `headsetcontrol -o json` output into a [`HeadsetStatus`].
pub fn parse_status(json: &[u8]) -> Result<HeadsetStatus, HeadsetError> {
    let output: HeadsetControlOutput =
        serde_json::from_slice(json).map_err(|err| HeadsetError::Parse(err.to_string()))?;

    let device = output.devices.first().ok_or(HeadsetError::NoDevice)?;

    Ok(HeadsetStatus {
        device_name: device.product.clone(),
        battery_level: device.battery.level,
        battery_state: BatteryState::from_headsetcontrol(
            &device.battery.status,
            device.battery.level,
        ),
        has_sidetone: device
            .capabilities_str
            .iter()
            .any(|capability| capability == "sidetone"),
    })
}

/// Errors from any [`Headset`] backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadsetError {
    /// HeadsetControl could not be started.
    Unavailable(String),
    /// HeadsetControl ran but reported no supported device.
    NoDevice,
    /// HeadsetControl's JSON output could not be understood.
    Parse(String),
    /// HeadsetControl exited with a non-zero status.
    CommandFailed(i32),
}

impl fmt::Display for HeadsetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeadsetError::Unavailable(reason) => {
                write!(f, "headsetcontrol could not be started: {reason}")
            }
            HeadsetError::NoDevice => write!(f, "no supported headset found"),
            HeadsetError::Parse(reason) => {
                write!(f, "could not parse headsetcontrol output: {reason}")
            }
            HeadsetError::CommandFailed(code) => {
                write!(f, "headsetcontrol exited with status {code}")
            }
        }
    }
}

impl std::error::Error for HeadsetError {}

/// In-memory backend for unit tests.
#[cfg(test)]
pub mod mock {
    use super::*;
    use std::cell::RefCell;

    /// Records every sidetone write and returns a canned status.
    pub struct MockHeadset {
        status: RefCell<Result<HeadsetStatus, HeadsetError>>,
        pub sidetone_calls: RefCell<Vec<u8>>,
        pub fail_set: bool,
    }

    impl MockHeadset {
        pub fn new(status: Result<HeadsetStatus, HeadsetError>) -> Self {
            Self {
                status: RefCell::new(status),
                sidetone_calls: RefCell::new(Vec::new()),
                fail_set: false,
            }
        }

        /// A healthy mock device: 42%, sidetone support, named "Test Device".
        pub fn available() -> Self {
            Self::new(Ok(HeadsetStatus {
                device_name: "Test Device".to_string(),
                battery_level: 42,
                battery_state: BatteryState::Medium,
                has_sidetone: true,
            }))
        }

        /// A mock with no device attached.
        pub fn missing() -> Self {
            Self::new(Err(HeadsetError::NoDevice))
        }

        pub fn calls(&self) -> Vec<u8> {
            self.sidetone_calls.borrow().clone()
        }
    }

    impl Headset for MockHeadset {
        fn status(&self) -> Result<HeadsetStatus, HeadsetError> {
            self.status.borrow().clone()
        }

        fn set_sidetone(&self, level: u8) -> Result<(), HeadsetError> {
            self.sidetone_calls.borrow_mut().push(level);

            if self.fail_set {
                Err(HeadsetError::CommandFailed(1))
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn mock_records_sidetone_writes() {
        let headset = mock::MockHeadset::available();

        headset.set_sidetone(12).unwrap();
        headset.set_sidetone(0).unwrap();

        assert_eq!(headset.calls(), vec![12, 0]);
        assert_eq!(headset.status().unwrap().battery_level, 42);
    }

    #[test]
    fn mock_can_report_missing_device() {
        let headset = mock::MockHeadset::missing();

        assert_eq!(headset.status(), Err(HeadsetError::NoDevice));
    }

    // --- BatteryState::from_headsetcontrol ---------------------------------

    #[test]
    fn battery_level_boundaries_map_to_expected_states() {
        let cases = [
            (100, BatteryState::Good),
            (51, BatteryState::Good),
            (50, BatteryState::Medium),
            (49, BatteryState::Medium),
            (16, BatteryState::Medium),
            (15, BatteryState::Medium),
            (14, BatteryState::Low),
            (1, BatteryState::Low),
            (0, BatteryState::Low),
            (-1, BatteryState::Low),
            (i32::MIN, BatteryState::Low),
            (i32::MAX, BatteryState::Good),
        ];

        for (level, expected) in cases {
            assert_eq!(
                BatteryState::from_headsetcontrol("BATTERY_AVAILABLE", level),
                expected,
                "level {level} should map to {expected:?}"
            );
        }
    }

    #[test]
    fn battery_available_status_is_derived_from_level() {
        assert_eq!(
            BatteryState::from_headsetcontrol("BATTERY_AVAILABLE", 80),
            BatteryState::Good
        );
        assert_eq!(
            BatteryState::from_headsetcontrol("BATTERY_AVAILABLE", 30),
            BatteryState::Medium
        );
        assert_eq!(
            BatteryState::from_headsetcontrol("BATTERY_AVAILABLE", 5),
            BatteryState::Low
        );
    }

    #[test]
    fn battery_charging_wins_over_any_level() {
        for level in [-1, 0, 14, 15, 50, 100] {
            assert_eq!(
                BatteryState::from_headsetcontrol("BATTERY_CHARGING", level),
                BatteryState::Charging,
                "charging must win at level {level}"
            );
        }
    }

    #[test]
    fn battery_unavailable_wins_over_any_level() {
        for level in [-1, 0, 15, 50, 100] {
            assert_eq!(
                BatteryState::from_headsetcontrol("BATTERY_UNAVAILABLE", level),
                BatteryState::Unavailable,
                "unavailable must win at level {level}"
            );
        }
    }

    #[test]
    fn empty_and_unknown_statuses_fall_back_to_level() {
        for status in ["", "BATTERY_UNKNOWN", "battery_available", "garbage"] {
            assert_eq!(
                BatteryState::from_headsetcontrol(status, 100),
                BatteryState::Good,
                "status {status:?}"
            );
            assert_eq!(
                BatteryState::from_headsetcontrol(status, 50),
                BatteryState::Medium,
                "status {status:?}"
            );
            assert_eq!(
                BatteryState::from_headsetcontrol(status, 14),
                BatteryState::Low,
                "status {status:?}"
            );
        }
    }

    // --- parse_status ------------------------------------------------------

    /// A realistic `headsetcontrol -o json` document.
    const REALISTIC_JSON: &str = r#"
    {
      "devices": [
        {
          "status": "success",
          "device": "SteelSeries Arctis Nova Pro Wireless",
          "vendor": "SteelSeries",
          "product": "Arctis Nova Pro Wireless",
          "vendor_id": "1038",
          "product_id": "12e0",
          "capabilities": ["CAP_SIDETONE", "CAP_BATTERY_STATUS", "CAP_NOTIFICATIONS"],
          "capabilities_str": ["battery", "sidetone", "notifications"],
          "battery": {
            "status": "BATTERY_AVAILABLE",
            "level": 82
          }
        }
      ]
    }
    "#;

    #[test]
    fn parse_status_reads_a_realistic_document() {
        let status = parse_status(REALISTIC_JSON.as_bytes()).unwrap();

        assert_eq!(status.device_name, "Arctis Nova Pro Wireless");
        assert_eq!(status.battery_level, 82);
        assert_eq!(status.battery_state, BatteryState::Good);
        assert!(status.has_sidetone);
    }

    #[test]
    fn parse_status_keeps_the_first_of_multiple_devices() {
        let json = r#"
        {
          "devices": [
            {
              "product": "First Headset",
              "capabilities_str": ["sidetone"],
              "battery": {"status": "BATTERY_CHARGING", "level": 12}
            },
            {
              "product": "Second Headset",
              "capabilities_str": ["battery"],
              "battery": {"status": "BATTERY_AVAILABLE", "level": 99}
            }
          ]
        }
        "#;

        let status = parse_status(json.as_bytes()).unwrap();

        assert_eq!(status.device_name, "First Headset");
        assert_eq!(status.battery_level, 12);
        assert_eq!(status.battery_state, BatteryState::Charging);
        assert!(status.has_sidetone);
    }

    #[test]
    fn parse_status_without_sidetone_capability() {
        let json = r#"
        {
          "devices": [
            {
              "product": "Battery Only",
              "capabilities_str": ["battery", "notifications"],
              "battery": {"status": "BATTERY_AVAILABLE", "level": 40}
            }
          ]
        }
        "#;

        let status = parse_status(json.as_bytes()).unwrap();

        assert!(!status.has_sidetone);
        assert_eq!(status.device_name, "Battery Only");
        assert_eq!(status.battery_state, BatteryState::Medium);
    }

    #[test]
    fn parse_status_without_capabilities_field_defaults_to_no_sidetone() {
        let json = r#"
        {
          "devices": [
            {
              "product": "Minimal Headset",
              "battery": {"status": "BATTERY_AVAILABLE", "level": 100}
            }
          ]
        }
        "#;

        let status = parse_status(json.as_bytes()).unwrap();

        assert!(!status.has_sidetone);
        assert_eq!(status.battery_state, BatteryState::Good);
    }

    #[test]
    fn parse_status_with_empty_devices_is_no_device() {
        assert_eq!(
            parse_status(br#"{"devices": []}"#),
            Err(HeadsetError::NoDevice)
        );
    }

    #[test]
    fn parse_status_accepts_negative_and_zero_levels() {
        let negative = r#"
        {
          "devices": [
            {
              "product": "No Readings",
              "capabilities_str": [],
              "battery": {"status": "BATTERY_UNAVAILABLE", "level": -1}
            }
          ]
        }
        "#;
        let zero = r#"
        {
          "devices": [
            {
              "product": "Drained",
              "capabilities_str": ["sidetone"],
              "battery": {"status": "BATTERY_AVAILABLE", "level": 0}
            }
          ]
        }
        "#;

        let status = parse_status(negative.as_bytes()).unwrap();
        assert_eq!(status.battery_level, -1);
        assert_eq!(status.battery_state, BatteryState::Unavailable);

        let status = parse_status(zero.as_bytes()).unwrap();
        assert_eq!(status.battery_level, 0);
        assert_eq!(status.battery_state, BatteryState::Low);
        assert!(status.has_sidetone);
    }

    #[test]
    fn parse_status_propagates_non_ascii_device_names() {
        let json = r#"
        {
          "devices": [
            {
              "product": "Casque Sans Fil Ünïcode",
              "battery": {"status": "BATTERY_AVAILABLE", "level": 65}
            }
          ]
        }
        "#;

        assert_eq!(
            parse_status(json.as_bytes()).unwrap().device_name,
            "Casque Sans Fil Ünïcode"
        );
    }

    #[test]
    fn parse_status_ignores_unknown_fields() {
        let json = r#"
        {
          "version": "1.0",
          "devices": [
            {
              "product": "Future Proof",
              "future_field": {"nested": true},
              "battery": {"status": "BATTERY_CHARGING", "level": 33, "extra": 1}
            }
          ],
          "extra_top_level": [1, 2, 3]
        }
        "#;

        let status = parse_status(json.as_bytes()).unwrap();

        assert_eq!(status.device_name, "Future Proof");
        assert_eq!(status.battery_state, BatteryState::Charging);
        assert_eq!(status.battery_level, 33);
    }

    #[test]
    fn parse_status_rejects_invalid_json() {
        for json in [
            &b"not json"[..],
            &b"{}"[..],
            &b"[]"[..],
            &b"null"[..],
            &b"{\"devices\": 42}"[..],
        ] {
            assert!(
                matches!(parse_status(json), Err(HeadsetError::Parse(_))),
                "input {json:?} should fail to parse"
            );
        }
    }

    #[test]
    fn parse_status_rejects_truncated_json() {
        let truncated = &REALISTIC_JSON.as_bytes()[..REALISTIC_JSON.len() / 2];

        assert!(matches!(
            parse_status(truncated),
            Err(HeadsetError::Parse(_))
        ));
    }

    #[test]
    fn parse_status_rejects_invalid_utf8() {
        let invalid = b"{\"devices\": [{\"product\": \"\xff\xfe\"}]}";

        assert!(matches!(parse_status(invalid), Err(HeadsetError::Parse(_))));
    }

    #[test]
    fn parse_status_rejects_empty_input() {
        assert!(matches!(parse_status(b""), Err(HeadsetError::Parse(_))));
    }

    #[test]
    fn parse_status_rejects_missing_required_fields() {
        for json in [
            &br#"{"devices": [{"battery": {"status": "BATTERY_AVAILABLE", "level": 50}}]}"#[..],
            &br#"{"devices": [{"product": "No Battery"}]}"#[..],
            &br#"{"devices": [{"product": "No Level", "battery": {"status": "BATTERY_AVAILABLE"}}]}"#[..],
        ] {
            assert!(
                matches!(parse_status(json), Err(HeadsetError::Parse(_))),
                "input {:?} should fail to parse",
                String::from_utf8_lossy(json)
            );
        }
    }

    // --- command line construction ----------------------------------------

    fn args_of(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn command_in_normal_mode_has_no_arguments() {
        let headset = HeadsetControl::new(Mode::Normal);
        let command = headset.command();

        assert_eq!(command.get_program().to_string_lossy(), "headsetcontrol");
        assert!(args_of(&command).is_empty());
    }

    #[test]
    fn command_in_test_mode_prepends_the_test_device_flag() {
        let headset = HeadsetControl::new(Mode::Test);
        let command = headset.command();

        assert_eq!(command.get_program().to_string_lossy(), "headsetcontrol");
        assert_eq!(args_of(&command), vec![TEST_DEVICE_FLAG.to_string()]);
    }

    #[test]
    fn command_is_rebuilt_from_the_same_mode_every_time() {
        let headset = HeadsetControl::new(Mode::Test);

        assert_eq!(args_of(&headset.command()), args_of(&headset.command()));
        assert_eq!(
            args_of(&headset.command()),
            vec![TEST_DEVICE_FLAG.to_string()]
        );
    }

    #[test]
    fn command_never_produces_duplicate_flags() {
        let headset = HeadsetControl::new(Mode::Test);
        let args = args_of(&headset.command());

        assert_eq!(
            args.iter().filter(|arg| *arg == TEST_DEVICE_FLAG).count(),
            1
        );
    }

    // --- constants ---------------------------------------------------------

    #[test]
    fn protocol_constants_are_stable() {
        assert_eq!(TEST_DEVICE_FLAG, "--test-device");
        assert_eq!(SIDETONE_MAX, 128);
    }

    // --- HeadsetError ------------------------------------------------------

    #[test]
    fn error_display_messages_are_stable() {
        assert_eq!(
            HeadsetError::Unavailable("No such file or directory".to_string()).to_string(),
            "headsetcontrol could not be started: No such file or directory"
        );
        assert_eq!(
            HeadsetError::NoDevice.to_string(),
            "no supported headset found"
        );
        assert_eq!(
            HeadsetError::Parse("expected value at line 1".to_string()).to_string(),
            "could not parse headsetcontrol output: expected value at line 1"
        );
        assert_eq!(
            HeadsetError::CommandFailed(3).to_string(),
            "headsetcontrol exited with status 3"
        );
        assert_eq!(
            HeadsetError::CommandFailed(-1).to_string(),
            "headsetcontrol exited with status -1"
        );
    }

    #[test]
    fn errors_have_no_source() {
        for error in [
            HeadsetError::Unavailable("boom".to_string()),
            HeadsetError::NoDevice,
            HeadsetError::Parse("boom".to_string()),
            HeadsetError::CommandFailed(1),
        ] {
            assert!(error.source().is_none(), "{error:?} should have no source");
        }
    }

    #[test]
    fn errors_are_usable_as_boxed_std_errors() {
        let error: Box<dyn std::error::Error> = Box::new(HeadsetError::NoDevice);

        assert_eq!(error.to_string(), "no supported headset found");
    }

    // --- mock backend ------------------------------------------------------

    #[test]
    fn mock_available_exposes_a_healthy_device() {
        let headset = mock::MockHeadset::available();

        assert_eq!(
            headset.status().unwrap(),
            HeadsetStatus {
                device_name: "Test Device".to_string(),
                battery_level: 42,
                battery_state: BatteryState::Medium,
                has_sidetone: true,
            }
        );
        assert!(headset.calls().is_empty());
    }

    #[test]
    fn mock_failing_sidetone_write_still_records_the_call() {
        let mut headset = mock::MockHeadset::available();
        headset.fail_set = true;

        assert_eq!(
            headset.set_sidetone(SIDETONE_MAX),
            Err(HeadsetError::CommandFailed(1))
        );
        assert_eq!(headset.calls(), vec![SIDETONE_MAX]);
    }

    #[test]
    fn mock_status_returns_the_configured_error() {
        let headset = mock::MockHeadset::new(Err(HeadsetError::CommandFailed(9)));

        assert_eq!(headset.status(), Err(HeadsetError::CommandFailed(9)));
    }
}
