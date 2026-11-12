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
}
