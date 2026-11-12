mod config;
mod input;
mod ui;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use std::io;
use std::process::{Command, Stdio};

use config::KeyConfig;
use input::{handle_input, Action};
use ui::{draw, UiState};

#[derive(Debug, Serialize, Deserialize)]
struct HeadsetControlOutput {
    device_count: u32,
    devices: Vec<Device>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Device {
    status: String,
    device: String,
    product: String,
    #[serde(default)]
    capabilities_str: Vec<String>,
    battery: Battery,
}

#[derive(Debug, Serialize, Deserialize)]
struct Battery {
    status: String,
    level: i32,
}

/// Flag passed to `headsetcontrol` to make it use its built-in mock device.
const TEST_DEVICE_FLAG: &str = "--test-device";

/// Arguments `headset-tui` accepts. Anything else is rejected instead of silently
/// falling through to the TUI.
const KNOWN_FLAGS: [&str; 6] = [
    "--test",
    "--help",
    "-h",
    "--config",
    "--toggle-sidetone",
    "--waybar-status",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has_flag = |flag: &str| args.iter().any(|arg| arg == flag);

    if let Some(unknown) = args.iter().find(|arg| !KNOWN_FLAGS.contains(&arg.as_str())) {
        eprintln!("headset-tui: unknown argument '{}'", unknown);
        eprintln!("Run 'headset-tui --help' for usage.");
        std::process::exit(2);
    }

    // `--test` makes every `headsetcontrol` call use its simulated device.
    let test_mode = has_flag("--test");

    if has_flag("--help") || has_flag("-h") {
        print_help();
        return Ok(());
    }

    if has_flag("--config") {
        config::interactive_config()?;
        return Ok(());
    }

    if has_flag("--toggle-sidetone") {
        toggle_sidetone(test_mode)?;
        return Ok(());
    }

    if has_flag("--waybar-status") {
        return waybar_status(test_mode);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut keys = config::load_config();
    let default_sidetone = config::load_default_sidetone();
    let mut state = UiState {
        sidetone: default_sidetone,
        battery: 0,
        battery_status: String::new(),
        verbose_mode: false,
        device_name: String::new(),
        has_sidetone: true,
        test_mode,
    };

    // Apply default sidetone to headset if it's not 0
    if default_sidetone > 0 {
        set_sidetone(default_sidetone, test_mode);
    }

    update_battery(&mut state, test_mode);

    let result = run_app(&mut terminal, &mut keys, &mut state, test_mode);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    keys: &mut KeyConfig,
    state: &mut UiState,
    test_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| {
            draw(f, state, keys);
        })?;

        match handle_input(keys) {
            Action::DecBy1 => {
                if state.sidetone > 0 {
                    state.sidetone -= 1;
                    set_sidetone(state.sidetone, test_mode);
                }
            }
            Action::DecBy10 => {
                state.sidetone = state.sidetone.saturating_sub(10);
                set_sidetone(state.sidetone, test_mode);
            }
            Action::IncBy1 => {
                if state.sidetone < 128 {
                    state.sidetone += 1;
                    set_sidetone(state.sidetone, test_mode);
                }
            }
            Action::IncBy10 => {
                state.sidetone = (state.sidetone + 10).min(128);
                set_sidetone(state.sidetone, test_mode);
            }
            Action::SetFull => {
                state.sidetone = 128;
                set_sidetone(128, test_mode);
            }
            Action::SetNone => {
                state.sidetone = 0;
                set_sidetone(0, test_mode);
            }
            Action::ToggleVerbose => {
                state.verbose_mode = !state.verbose_mode;
            }
            Action::Config => {
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                terminal.show_cursor()?;

                config::interactive_config()?;

                enable_raw_mode()?;
                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                terminal.hide_cursor()?;
                terminal.clear()?;

                *keys = config::load_config();
                update_battery(state, test_mode);
            }
            Action::Quit => break,
            Action::None => {
                update_battery(state, test_mode);
            }
        }
    }

    Ok(())
}

fn toggle_sidetone(test_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
    let default_sidetone = config::load_default_sidetone();

    if default_sidetone == 0 {
        return Ok(());
    }

    // Use a simple state file to track if sidetone is currently on.
    // In test mode a separate file is used so the real headset state stays untouched.
    let file_name = if test_mode {
        ".cache/headset-tui-sidetone-state.test"
    } else {
        ".cache/headset-tui-sidetone-state"
    };
    let state_file = dirs::home_dir()
        .map(|p| p.join(file_name))
        .unwrap_or_default();

    let is_on = std::fs::read_to_string(&state_file)
        .map(|s| s.trim() == "on")
        .unwrap_or(false);

    if is_on {
        // Turn off
        set_sidetone(0, test_mode);
        save_toggle_state(&state_file, "off");
    } else {
        // Turn on
        set_sidetone(default_sidetone, test_mode);
        save_toggle_state(&state_file, "on");
    }

    Ok(())
}

/// Persist the toggle state, creating its directory when missing.
fn save_toggle_state(path: &std::path::Path, value: &str) {
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!(
                "headset-tui: could not create {}: {}",
                parent.display(),
                err
            );
            return;
        }
    }

    if let Err(err) = std::fs::write(path, value) {
        eprintln!("headset-tui: could not write {}: {}", path.display(), err);
    }
}

/// Build a `headsetcontrol` command, optionally targeting the simulated device.
fn headsetcontrol(test_mode: bool) -> Command {
    let mut cmd = Command::new("headsetcontrol");

    if test_mode {
        cmd.arg(TEST_DEVICE_FLAG);
    }

    cmd
}

fn set_sidetone(value: u8, test_mode: bool) {
    let _ = headsetcontrol(test_mode)
        .arg("-s")
        .arg(value.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn update_battery(state: &mut UiState, test_mode: bool) {
    if let Ok(output) = headsetcontrol(test_mode).arg("-o").arg("json").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Ok(hc_output) = serde_json::from_str::<HeadsetControlOutput>(&stdout) {
            if hc_output.device_count > 0 {
                let device = &hc_output.devices[0];

                // Extract device name
                state.device_name = device.product.clone();

                // Extract battery info
                state.battery = device.battery.level;
                state.battery_status = if device.battery.status == "BATTERY_CHARGING" {
                    "Charging".to_string()
                } else if state.battery > 50 {
                    "Good".to_string()
                } else if state.battery >= 15 {
                    "Medium".to_string()
                } else {
                    "Low".to_string()
                };

                // Check sidetone capability
                state.has_sidetone = device.capabilities_str.contains(&"sidetone".to_string());

                return;
            }
        }
    }

    state.battery = -1;
    state.battery_status = "Error".to_string();
    state.device_name = "Unknown Device".to_string();
    state.has_sidetone = false;
}

fn waybar_status(test_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(output) = headsetcontrol(test_mode).arg("-o").arg("json").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Ok(hc_output) = serde_json::from_str::<HeadsetControlOutput>(&stdout) {
            if hc_output.device_count > 0 {
                let device = &hc_output.devices[0];

                let battery_level = device.battery.level;
                let battery_status = &device.battery.status;

                // Don't show anything if battery is unavailable
                if battery_status == "BATTERY_UNAVAILABLE" {
                    println!("{{\"text\": \"\", \"class\": \"headset-unavailable\"}}");
                    return Ok(());
                }

                // Determine color and class
                let (color, class) = if battery_status == "BATTERY_CHARGING" {
                    ("#0066ff", "headset-charging")
                } else if battery_level > 50 {
                    ("#90ee90", "headset-good")
                } else if battery_level >= 15 {
                    ("#ffff00", "headset-medium")
                } else {
                    ("#ff0000", "headset-low")
                };

                let tooltip = if battery_status == "BATTERY_CHARGING" {
                    "󰂄 Charging".to_string()
                } else if battery_level >= 50 {
                    format!("󱊣 {}%", battery_level)
                } else if battery_level >= 15 {
                    format!("󱊢 {}%", battery_level)
                } else {
                    format!("󱊡 {}%", battery_level)
                };

                let text = format!(
                    "<span foreground=\\\"{}\\\" font_weight=\\\"bold\\\">󰋎</span>",
                    color
                );
                println!(
                    "{{\"text\": \"{}\", \"class\": \"{}\", \"tooltip\": \"{}\"}}",
                    text, class, tooltip
                );
                return Ok(());
            }
        }
    }

    println!("{}", waybar_error(test_mode));
    Ok(())
}

/// Waybar JSON shown when no headset (or, in test mode, no mock device) is available.
fn waybar_error(test_mode: bool) -> &'static str {
    if test_mode {
        r##"{"text": "<span foreground=\"#ff0000\" font_weight=\"bold\">󰋎</span>", "class": "headset-error", "tooltip": "Test device not available"}"##
    } else {
        r##"{"text": "<span foreground=\"#ff0000\" font_weight=\"bold\">󰋎</span>", "class": "headset-error", "tooltip": "Headset not found"}"##
    }
}

fn print_help() {
    println!(
        "\
headset-tui - HeadsetControl integration for Waybar

USAGE:
    headset-tui [OPTIONS]

OPTIONS:
    (none)              Launch the interactive TUI
    --waybar-status     Print the Waybar JSON for the battery module
    --toggle-sidetone   Toggle sidetone between off and the default level
    --config            Edit keybindings and default sidetone
    --test              Use HeadsetControl's simulated device instead of a real
                        headset. Only affects commands that talk to a headset,
                        so it combines with any option above, e.g.
                        `headset-tui --test` or `headset-tui --waybar-status --test`
    -h, --help          Show this help

NOTE:
    --test requires HeadsetControl with the --test-device option (mock device).
    No headset needs to be connected."
    );
}
