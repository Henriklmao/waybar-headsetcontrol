mod app;
mod cli;
mod config;
mod headset;
mod input;
mod ui;
mod waybar;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::process::ExitCode;

use app::{Outcome, UiState};
use cli::{Cli, Command, Mode};
use config::KeyConfig;
use headset::{Headset, HeadsetControl};
use input::Action;

fn main() -> ExitCode {
    match Cli::parse(std::env::args().skip(1)) {
        Ok(cli) => match run(cli) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("headset-tui: {err}");
                ExitCode::FAILURE
            }
        },
        Err(err) => {
            eprintln!("headset-tui: {err}");
            eprintln!("Run 'headset-tui --help' for usage.");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Help => {
            print!("{}", cli::help_text());
            Ok(())
        }
        Command::Config => {
            let stdin = io::stdin();
            let mut input = stdin.lock();
            let mut output = io::stdout();
            config::interactive_config(cli.mode, &mut input, &mut output)?;
            Ok(())
        }
        Command::WaybarStatus => {
            print_waybar_status(cli.mode);
            Ok(())
        }
        Command::ToggleSidetone => toggle_sidetone(cli.mode),
        Command::Tui => run_tui(cli.mode),
    }
}

/// One `--waybar-status` run: always prints a JSON object Waybar can consume.
fn print_waybar_status(mode: Mode) {
    let headset = HeadsetControl::new(mode);

    let output = match headset.status() {
        Ok(status) => waybar::status_output(&status),
        Err(_) => waybar::error_output(mode),
    };

    println!("{}", output.to_json());
}

fn toggle_sidetone(mode: Mode) -> Result<(), Box<dyn std::error::Error>> {
    let config = config::load(mode);

    if config.default_sidetone == 0 {
        return Ok(());
    }

    let headset = HeadsetControl::new(mode);
    app::toggle_sidetone(&headset, config.default_sidetone, &config::state_path(mode))?;

    Ok(())
}

fn run_tui(mode: Mode) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = config::load(mode);
    let headset = HeadsetControl::new(mode);

    // Apply default sidetone to the headset if it's not 0
    if config.default_sidetone > 0 {
        let _ = headset.set_sidetone(config.default_sidetone);
    }

    let mut state = UiState::new(config.default_sidetone, mode);
    app::apply_status(&mut state, headset.status());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut config.keys, &mut state, &headset);

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
    headset: &dyn Headset,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| {
            ui::draw(frame, state, keys);
        })?;

        let action = input::handle_input(keys);

        // Idle ticks refresh the battery display.
        if action == Action::None {
            app::apply_status(state, headset.status());
        }

        match app::apply_action(action, state, headset) {
            Outcome::Quit => break,
            Outcome::Continue => {}
            Outcome::OpenConfig => {
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                terminal.show_cursor()?;

                let stdin = io::stdin();
                let mut input = stdin.lock();
                let mut output = io::stdout();
                config::interactive_config(state.mode, &mut input, &mut output)?;

                enable_raw_mode()?;
                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                terminal.hide_cursor()?;
                terminal.clear()?;

                *keys = config::load(state.mode).keys;
                app::apply_status(state, headset.status());
            }
        }
    }

    Ok(())
}
