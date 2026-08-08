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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--config" {
        config::interactive_config()?;
        return Ok(());
    }

    if args.len() > 1 && args[1] == "--toggle-sidetone" {
        toggle_sidetone()?;
        return Ok(());
    }

    if args.len() > 1 && args[1] == "--install-waybar" {
        return install_waybar();
    }

    if args.len() > 1 && args[1] == "--waybar-status" {
        return waybar_status();
    }

    if args.len() > 1 && args[1] == "--quickshell-status" {
        return quickshell_status();
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
    };

    // Apply default sidetone to headset if it's not 0
    if default_sidetone > 0 {
        set_sidetone(default_sidetone);
    }

    update_battery(&mut state);

    let result = run_app(&mut terminal, &mut keys, &mut state);

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
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| {
            draw(f, state, keys);
        })?;

        match handle_input(keys) {
            Action::DecBy1 => {
                if state.sidetone > 0 {
                    state.sidetone -= 1;
                    set_sidetone(state.sidetone);
                }
            }
            Action::DecBy10 => {
                state.sidetone = state.sidetone.saturating_sub(10);
                set_sidetone(state.sidetone);
            }
            Action::IncBy1 => {
                if state.sidetone < 128 {
                    state.sidetone += 1;
                    set_sidetone(state.sidetone);
                }
            }
            Action::IncBy10 => {
                state.sidetone = (state.sidetone + 10).min(128);
                set_sidetone(state.sidetone);
            }
            Action::SetFull => {
                state.sidetone = 128;
                set_sidetone(128);
            }
            Action::SetNone => {
                state.sidetone = 0;
                set_sidetone(0);
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
                update_battery(state);
            }
            Action::Quit => break,
            Action::None => {
                update_battery(state);
            }
        }
    }

    Ok(())
}

fn toggle_sidetone() -> Result<(), Box<dyn std::error::Error>> {
    let default_sidetone = config::load_default_sidetone();

    if default_sidetone == 0 {
        return Ok(());
    }

    // Use a simple state file to track if sidetone is currently on
    let state_file = dirs::home_dir()
        .map(|p| p.join(".cache/headset-tui-sidetone-state"))
        .unwrap_or_default();

    let is_on = std::fs::read_to_string(&state_file)
        .map(|s| s.trim() == "on")
        .unwrap_or(false);

    if is_on {
        // Turn off
        set_sidetone(0);
        let _ = std::fs::write(&state_file, "off");
    } else {
        // Turn on
        set_sidetone(default_sidetone);
        let _ = std::fs::write(&state_file, "on");
    }

    Ok(())
}

fn set_sidetone(value: u8) {
    let _ = Command::new("headsetcontrol")
        .arg("-s")
        .arg(value.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn update_battery(state: &mut UiState) {
    if let Ok(output) = Command::new("headsetcontrol")
        .arg("-o")
        .arg("json")
        .output()
    {
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

fn waybar_status() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(output) = Command::new("headsetcontrol")
        .arg("-o")
        .arg("json")
        .output()
    {
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
            } else {
                println!("{{\"text\": \"<span foreground=\\\"#ff0000\\\" font_weight=\\\"bold\\\">󰋎</span>\", \"class\": \"headset-error\", \"tooltip\": \"Headset not found\"}}");
            }
        } else {
            println!("{{\"text\": \"<span foreground=\\\"#ff0000\\\" font_weight=\\\"bold\\\">󰋎</span>\", \"class\": \"headset-error\", \"tooltip\": \"Headset not found\"}}");
        }
    } else {
        println!("{{\"text\": \"<span foreground=\\\"#ff0000\\\" font_weight=\\\"bold\\\">󰋎</span>\", \"class\": \"headset-error\", \"tooltip\": \"Headset not found\"}}");
    }
    Ok(())
}

fn install_waybar() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    let waybar_config = format!("{}/.config/waybar/config.jsonc", home);
    let module_name = "custom/headsetcontrol";

    // Detect terminal
    let terminal_cmd = if std::process::Command::new("alacritty").arg("--version").output().is_ok() {
        "alacritty"
    } else if std::process::Command::new("kitty").arg("--version").output().is_ok() {
        "kitty"
    } else {
        "alacritty"
    };

    println!("=== Headset-TUI Waybar Installer ===");

    if !std::path::Path::new(&waybar_config).exists() {
        println!("  ⚠ Waybar config not found at {}", waybar_config);
        return Ok(());
    }

    let content = std::fs::read_to_string(&waybar_config)?;
    if content.contains(&format!("\"{}\"", module_name)) {
        println!("  ✓ headset-tui already configured in Waybar");
        return Ok(());
    }

    // Determine binary path (use current executable path if possible, fallback to headset-tui)
    let bin_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "headset-tui".to_string());

    println!("Adding headset-tui to Waybar (using {})...", terminal_cmd);

    // Simple insertion into modules-right if found, or append block
    let mut new_content = content.clone();
    if let Some(pos) = new_content.find("modules-right") {
        if let Some(bracket) = new_content[pos..].find('[') {
            let insert_idx = pos + bracket + 1;
            new_content.insert_str(insert_idx, &format!("\"{}\", ", module_name));
        }
    }

    // Add module definition block before last brace
    if let Some(last_brace) = new_content.rfind('}') {
        let block = format!(
            r#"
  "{}": {{
    "exec": "{} --waybar-status",
    "return-type": "json",
    "interval": 10,
    "format": "{{text}}",
    "tooltip": true,
    "on-click": "{} {}",
    "on-right-click": "bash -c '{} --toggle-sidetone &'"
  }},
"#,
            module_name, bin_path, terminal_cmd, bin_path, bin_path
        );
        new_content.insert_str(last_brace, &block);
    }

    std::fs::write(&waybar_config, new_content)?;
    println!("  ✓ Waybar configuration updated successfully!");

    // Restart waybar if running
    if std::process::Command::new("pgrep").arg("-f").arg("waybar").output().is_ok() {
        println!("Restarting Waybar...");
        let _ = std::process::Command::new("pkill").arg("-f").arg("waybar").status();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let _ = std::process::Command::new("waybar").spawn();
    }

    Ok(())
}

fn quickshell_status() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(output) = Command::new("headsetcontrol")
        .arg("-o")
        .arg("json")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Ok(hc_output) = serde_json::from_str::<HeadsetControlOutput>(&stdout) {
            if hc_output.device_count > 0 {
                let device = &hc_output.devices[0];
                let battery_level = device.battery.level;
                let battery_status = &device.battery.status;
                let charging = battery_status == "BATTERY_CHARGING";
                let product = &device.product;

                let icon = if charging {
                    "󰂄"
                } else if battery_level > 50 {
                    "󱊣"
                } else if battery_level >= 15 {
                    "󱊢"
                } else {
                    "󱊡"
                };

                println!(
                    "{{\"connected\": true, \"product\": \"{}\", \"battery\": {}, \"charging\": {}, \"icon\": \"{}\"}}",
                    product, battery_level, charging, icon
                );
                return Ok(());
            }
        }
    }

    println!("{{\"connected\": false, \"product\": \"\", \"battery\": -1, \"charging\": false, \"icon\": \"󰋎\"}}");
    Ok(())
}
