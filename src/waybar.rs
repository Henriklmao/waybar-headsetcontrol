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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn status(state: BatteryState, level: i32) -> HeadsetStatus {
        HeadsetStatus {
            device_name: "Test Device".to_string(),
            battery_level: level,
            battery_state: state,
            has_sidetone: true,
        }
    }

    fn icon_span(color: &str) -> String {
        format!("<span foreground=\"{color}\" font_weight=\"bold\">{ICON}</span>")
    }

    // --- charging ----------------------------------------------------------

    #[test]
    fn charging_output_uses_blue_icon_and_charging_tooltip() {
        let output = status_output(&status(BatteryState::Charging, 0));

        assert_eq!(output.text, icon_span("#0066ff"));
        assert_eq!(output.class, "headset-charging");
        assert_eq!(output.tooltip.as_deref(), Some("󰂄 Charging"));
    }

    #[test]
    fn charging_tooltip_ignores_the_level() {
        for level in [0, 14, 50, 100] {
            let output = status_output(&status(BatteryState::Charging, level));

            assert_eq!(output.tooltip.as_deref(), Some("󰂄 Charging"));
            assert_eq!(output.class, "headset-charging");
        }
    }

    // --- good --------------------------------------------------------------

    #[test]
    fn good_output_uses_green_icon_and_full_battery_glyph() {
        let output = status_output(&status(BatteryState::Good, 100));

        assert_eq!(output.text, icon_span("#90ee90"));
        assert_eq!(output.class, "headset-good");
        assert_eq!(output.tooltip.as_deref(), Some("󱊣 100%"));
    }

    #[test]
    fn good_output_at_the_lower_boundary() {
        let output = status_output(&status(BatteryState::Good, 51));

        assert_eq!(output.class, "headset-good");
        assert_eq!(output.tooltip.as_deref(), Some("󱊣 51%"));
    }

    // --- medium ------------------------------------------------------------

    #[test]
    fn medium_output_at_boundary_50_keeps_the_full_battery_tooltip() {
        let output = status_output(&status(BatteryState::Medium, 50));

        assert_eq!(output.text, icon_span("#ffff00"));
        assert_eq!(output.class, "headset-medium");
        // The class uses `> 50` but the tooltip uses `>= 50`.
        assert_eq!(output.tooltip.as_deref(), Some("󱊣 50%"));
    }

    #[test]
    fn medium_output_below_50_uses_the_half_battery_glyph() {
        let output = status_output(&status(BatteryState::Medium, 49));

        assert_eq!(output.class, "headset-medium");
        assert_eq!(output.tooltip.as_deref(), Some("󱊢 49%"));
    }

    #[test]
    fn medium_output_at_lower_boundary_15() {
        let output = status_output(&status(BatteryState::Medium, 15));

        assert_eq!(output.text, icon_span("#ffff00"));
        assert_eq!(output.class, "headset-medium");
        assert_eq!(output.tooltip.as_deref(), Some("󱊢 15%"));
    }

    // --- low ---------------------------------------------------------------

    #[test]
    fn low_output_uses_red_icon_and_low_battery_glyph() {
        let output = status_output(&status(BatteryState::Low, 14));

        assert_eq!(output.text, icon_span("#ff0000"));
        assert_eq!(output.class, "headset-low");
        assert_eq!(output.tooltip.as_deref(), Some("󱊡 14%"));
    }

    #[test]
    fn low_output_at_zero() {
        let output = status_output(&status(BatteryState::Low, 0));

        assert_eq!(output.class, "headset-low");
        assert_eq!(output.tooltip.as_deref(), Some("󱊡 0%"));
    }

    #[test]
    fn low_output_with_a_negative_level_is_shown_verbatim() {
        let output = status_output(&status(BatteryState::Low, -1));

        assert_eq!(output.class, "headset-low");
        assert_eq!(output.tooltip.as_deref(), Some("󱊡 -1%"));
    }

    // --- unavailable -------------------------------------------------------

    #[test]
    fn unavailable_output_has_no_text_and_no_tooltip() {
        let output = status_output(&status(BatteryState::Unavailable, 100));

        assert_eq!(output.text, "");
        assert_eq!(output.class, "headset-unavailable");
        assert_eq!(output.tooltip, None);
    }

    #[test]
    fn unavailable_output_does_not_depend_on_the_level() {
        for level in [-1, 0, 15, 50, 100] {
            let output = status_output(&status(BatteryState::Unavailable, level));

            assert_eq!(output.class, "headset-unavailable");
            assert!(output.text.is_empty());
            assert!(output.tooltip.is_none());
        }
    }

    // --- error output ------------------------------------------------------

    #[test]
    fn error_output_in_normal_mode_reports_a_missing_headset() {
        let output = error_output(Mode::Normal);

        assert_eq!(output.text, icon_span("#ff0000"));
        assert_eq!(output.class, "headset-error");
        assert_eq!(output.tooltip.as_deref(), Some("Headset not found"));
    }

    #[test]
    fn error_output_in_test_mode_mentions_the_test_device() {
        let output = error_output(Mode::Test);

        assert_eq!(output.text, icon_span("#ff0000"));
        assert_eq!(output.class, "headset-error");
        assert_eq!(output.tooltip.as_deref(), Some("Test device not available"));
    }

    #[test]
    fn error_text_matches_the_status_output_icon_layout() {
        let output = error_output(Mode::Test);

        assert!(output.text.starts_with("<span foreground="));
        assert!(output.text.contains("font_weight=\"bold\""));
        assert!(output.text.contains(ICON));
    }

    // --- icon --------------------------------------------------------------

    #[test]
    fn icon_is_a_non_empty_single_glyph() {
        assert!(!ICON.is_empty());
        assert_eq!(ICON.chars().count(), 1);
    }

    // --- to_json -----------------------------------------------------------

    #[test]
    fn to_json_contains_text_and_class() {
        let output = status_output(&status(BatteryState::Good, 90));
        let raw = output.to_json();
        let value: Value = serde_json::from_str(&raw).unwrap();

        assert_eq!(value["text"], Value::String(output.text.clone()));
        assert_eq!(value["class"], Value::String("headset-good".to_string()));
        assert_eq!(value["tooltip"], Value::String("󱊣 90%".to_string()));
    }

    #[test]
    fn to_json_matches_the_expected_object() {
        let output = status_output(&status(BatteryState::Low, 3));
        let value: Value = serde_json::from_str(&output.to_json()).unwrap();

        assert_eq!(
            value,
            json!({
                "text": icon_span("#ff0000"),
                "class": "headset-low",
                "tooltip": "󱊡 3%",
            })
        );
    }

    #[test]
    fn to_json_omits_the_tooltip_when_absent() {
        let output = status_output(&status(BatteryState::Unavailable, 0));
        let raw = output.to_json();
        let value: Value = serde_json::from_str(&raw).unwrap();

        assert!(value.get("tooltip").is_none());
        assert!(!raw.contains("tooltip"));
        assert_eq!(value, json!({ "text": "", "class": "headset-unavailable" }));
    }

    #[test]
    fn to_json_includes_the_tooltip_when_present() {
        let output = error_output(Mode::Normal);
        let value: Value = serde_json::from_str(&output.to_json()).unwrap();

        assert_eq!(
            value["tooltip"],
            Value::String("Headset not found".to_string())
        );
    }

    #[test]
    fn to_json_round_trips_the_quoted_span() {
        let output = status_output(&status(BatteryState::Charging, 75));
        let raw = output.to_json();

        // The span's attribute quotes must be escaped in the JSON document...
        assert!(raw.contains("\\\""));
        // ...and come back unchanged when the document is parsed.
        let value: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["text"], Value::String(output.text.clone()));
        assert_eq!(
            value["text"].as_str().unwrap(),
            "<span foreground=\"#0066ff\" font_weight=\"bold\">󰋎</span>"
        );
    }

    #[test]
    fn to_json_escapes_special_characters_in_the_tooltip() {
        let output = WaybarOutput {
            text: "line1\nline2".to_string(),
            class: "custom".to_string(),
            tooltip: Some("quote \" and backslash \\".to_string()),
        };
        let value: Value = serde_json::from_str(&output.to_json()).unwrap();

        assert_eq!(value["text"], Value::String("line1\nline2".to_string()));
        assert_eq!(
            value["tooltip"],
            Value::String("quote \" and backslash \\".to_string())
        );
    }

    #[test]
    fn every_state_serializes_to_a_valid_json_document() {
        for state in [
            BatteryState::Charging,
            BatteryState::Good,
            BatteryState::Medium,
            BatteryState::Low,
            BatteryState::Unavailable,
        ] {
            let output = status_output(&status(state, 50));
            let value: Value = serde_json::from_str(&output.to_json()).unwrap();

            assert!(value.get("text").is_some(), "{state:?}");
            assert!(value.get("class").is_some(), "{state:?}");
        }
    }
}
