mod config;
mod input;
mod ui;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::process::Command;

use config::KeyConfig;
use input::{handle_input, Action};
use ui::{draw, UiState};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 && args[1] == "--config" {
        config::interactive_config()?;
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let keys = config::load_config();
    let mut state = UiState {
        sidetone: get_sidetone(),
        battery: 0,
        battery_status: String::new(),
    };
    update_battery(&mut state);

    let result = run_app(&mut terminal, &keys, &mut state);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    keys: &KeyConfig,
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

fn set_sidetone(value: u8) {
    let _ = Command::new("headsetcontrol")
        .arg("-s")
        .arg(value.to_string())
        .spawn();
}

fn update_battery(state: &mut UiState) {
    if let Ok(output) = Command::new("headsetcontrol").arg("-b").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Some(pos) = stdout.find("Level:") {
            let rest = &stdout[pos + 6..];
            let num_str: String = rest
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect();

            if let Ok(level) = num_str.parse::<i32>() {
                state.battery = level;
                state.battery_status = if level > 50 {
                    "🟢 Good".to_string()
                } else if level >= 15 {
                    "🟡 Medium".to_string()
                } else {
                    "🔴 Low".to_string()
                };
                return;
            }
        }
    }

    state.battery = -1;
    state.battery_status = "❌ Error".to_string();
}
