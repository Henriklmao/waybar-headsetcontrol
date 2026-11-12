//! Waybar JSON output.

use crate::cli::Mode;
use crate::headset::{BatteryState, HeadsetStatus};
use serde_json::Value;

/// Nerd Font headset glyph used as the Waybar icon.
pub const ICON: &str = "󰋎";

/// One Waybar `custom` module result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaybarOutput {
    pub text: String,
    pub class: String,
    pub tooltip: Option<String>,
}

impl WaybarOutput {
    /// Serialize into the JSON Waybar expects.
    pub fn to_json(&self) -> String {
        let mut object = serde_json::Map::new();
        object.insert("text".to_string(), Value::String(self.text.clone()));
        object.insert("class".to_string(), Value::String(self.class.clone()));

        if let Some(tooltip) = &self.tooltip {
            object.insert("tooltip".to_string(), Value::String(tooltip.clone()));
        }

        Value::Object(object).to_string()
    }
}

/// Output for a headset with a known battery state.
pub fn status_output(status: &HeadsetStatus) -> WaybarOutput {
    if status.battery_state == BatteryState::Unavailable {
        return WaybarOutput {
            text: String::new(),
            class: "headset-unavailable".to_string(),
            tooltip: None,
        };
    }

    let (color, class) = match status.battery_state {
        BatteryState::Charging => ("#0066ff", "headset-charging"),
        BatteryState::Good => ("#90ee90", "headset-good"),
        BatteryState::Medium => ("#ffff00", "headset-medium"),
        BatteryState::Low => ("#ff0000", "headset-low"),
        BatteryState::Unavailable => unreachable!("handled above"),
    };

    let tooltip = if status.battery_state == BatteryState::Charging {
        "󰂄 Charging".to_string()
    } else if status.battery_level >= 50 {
        format!("󱊣 {}%", status.battery_level)
    } else if status.battery_level >= 15 {
        format!("󱊢 {}%", status.battery_level)
    } else {
        format!("󱊡 {}%", status.battery_level)
    };

    WaybarOutput {
        text: format!("<span foreground=\"{color}\" font_weight=\"bold\">{ICON}</span>"),
        class: class.to_string(),
        tooltip: Some(tooltip),
    }
}

/// Output shown when no headset (or, in test mode, no mock device) is available.
pub fn error_output(mode: Mode) -> WaybarOutput {
    let tooltip = if mode.is_test() {
        "Test device not available"
    } else {
        "Headset not found"
    };

    WaybarOutput {
        text: format!("<span foreground=\"#ff0000\" font_weight=\"bold\">{ICON}</span>"),
        class: "headset-error".to_string(),
        tooltip: Some(tooltip.to_string()),
    }
}
