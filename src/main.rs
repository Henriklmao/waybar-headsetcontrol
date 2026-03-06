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
    
    if args.len() > 1 && args[1] == "--waybar-status" {
        return waybar_status();
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

fn get_sidetone() -> u8 {
    Command::new("headsetcontrol")
        .arg("-g")
        .output()
        .ok()
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .trim()
                .split_whitespace()
                .find_map(|s| s.parse::<u8>().ok())
        })
        .unwrap_or(0)
}

fn toggle_sidetone() -> Result<(), Box<dyn std::error::Error>> {
    let default_sidetone = config::load_default_sidetone();
    
    if default_sidetone == 0 {
        return Ok(());
    }
    
    // Use a simple state file to track if sidetone is currently on
    let state_file = dirs::home_dir()
        .map(|p| p.join(".cache/wb-headsetcontrol-sidetone-state"))
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
                
                // Check if sidetone capability exists
                let has_sidetone = device.capabilities_str.contains(&"sidetone".to_string());
                
                let battery_level = device.battery.level;
                let battery_status = &device.battery.status;
                
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
                
                let tooltip = if has_sidetone {
                    format!("Battery: {}%", battery_level)
                } else {
                    format!("Battery: {}% | Sidetone: not available", battery_level)
                };
                
                let text = format!("<span foreground=\\\"{}\\\" font_weight=\\\"bold\\\">󰋎</span>", color);
                println!("{{\"text\": \"{}\", \"class\": \"{}\", \"tooltip\": \"{}\", \"on-click\": \"wb-headset\", \"on-right-click\": \"wb-headset --toggle-sidetone\"}}", text, class, tooltip);
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
