use crate::app::UiState;
use crate::config::KeyConfig;
use crate::headset::{BatteryState, SIDETONE_MAX};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, state: &UiState, keys: &KeyConfig) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(3),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    // Title
    let title_text = if state.mode.is_test() {
        "Headset TUI — simulated device"
    } else {
        "Headset TUI"
    };
    let title = Paragraph::new(title_text)
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(title, chunks[0]);

    // Battery status
    let battery_color = if state.battery_state == BatteryState::Charging {
        Color::Blue
    } else if state.battery.unwrap_or_default() > 50 {
        Color::Green
    } else if state.battery.unwrap_or_default() >= 15 {
        Color::Yellow
    } else {
        Color::Red
    };

    let battery_text = match state.battery {
        Some(level) if level >= 0 => Line::from(vec![Span::styled(
            format!("󰋎 Battery: {level}% - {}", state.device_name),
            Style::default().fg(battery_color),
        )]),
        _ => Line::from(vec![Span::styled(
            "󰋎 Battery: Error",
            Style::default().fg(Color::Red),
        )]),
    };

    let battery =
        Paragraph::new(battery_text).block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(battery, chunks[1]);

    // Sidetone slider
    if state.has_sidetone {
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Sidetone Level"),
            )
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent((f64::from(state.sidetone) / f64::from(SIDETONE_MAX) * 100.0) as u16)
            .label(format!("{}/{}", state.sidetone, SIDETONE_MAX));
        f.render_widget(gauge, chunks[2]);
    } else {
        let text = Paragraph::new("Sidetone: not available")
            .block(Block::default().borders(Borders::ALL).title("Sidetone"));
        f.render_widget(text, chunks[2]);
    }

    // Help text
    let help_text = format!(
        "[{}] Dec-1  [{}] Dec-10  [{}] Inc-1  [{}] Inc-10  [{}] Full  [{}] None  [{}] Verbose  [{}] Config  [{}] Quit",
        keys.dec1.to_uppercase(),
        keys.dec10.to_uppercase(),
        keys.inc1.to_uppercase(),
        keys.inc10.to_uppercase(),
        keys.full.to_uppercase(),
        keys.none.to_uppercase(),
        keys.verbose.to_uppercase(),
        keys.config.to_uppercase(),
        keys.quit.to_uppercase(),
    );

    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[3]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Mode;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    const WIDTH: u16 = 120;
    const HEIGHT: u16 = 24;
    /// Wide enough that the whole help footer fits on one row.
    const WIDE_WIDTH: u16 = 200;

    fn state() -> UiState {
        let mut state = UiState::new(42, Mode::Normal);
        state.battery = Some(42);
        state.battery_state = BatteryState::Medium;
        state.device_name = "Test Device".to_string();
        state.has_sidetone = true;
        state
    }

    fn render_buffer(state: &UiState, keys: &KeyConfig, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| draw(frame, state, keys))
            .expect("draw should succeed");
        terminal.backend().buffer().clone()
    }

    /// Render `state` and flatten the buffer, one line per terminal row.
    fn render(state: &UiState, keys: &KeyConfig) -> String {
        buffer_to_string(&render_buffer(state, keys, WIDTH, HEIGHT))
    }

    fn buffer_to_string(buffer: &Buffer) -> String {
        let width = buffer.area().width as usize;
        buffer
            .content()
            .chunks(width)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Position of the first cell where `needle` starts, if it was rendered.
    fn find_text(buffer: &Buffer, needle: &str) -> Option<(u16, u16)> {
        let width = buffer.area().width as usize;

        for (y, row) in buffer.content().chunks(width).enumerate() {
            let text: String = row.iter().map(|cell| cell.symbol()).collect();

            if let Some(offset) = text.find(needle) {
                let x = text[..offset].chars().count() as u16;
                return Some((x, y as u16));
            }
        }

        None
    }

    fn cell_at(buffer: &Buffer, needle: &str) -> ratatui::buffer::Cell {
        let (x, y) = find_text(buffer, needle).unwrap_or_else(|| {
            panic!("{needle:?} was not rendered:\n{}", buffer_to_string(buffer))
        });
        buffer[(x, y)].clone()
    }

    fn color_at(buffer: &Buffer, needle: &str) -> Color {
        cell_at(buffer, needle).fg
    }

    fn custom_keys() -> KeyConfig {
        KeyConfig {
            dec1: 'j',
            dec10: 'k',
            inc1: 'l',
            inc10: 'i',
            full: 'o',
            none: 'u',
            quit: 'x',
            config: 'y',
            verbose: 'h',
        }
    }

    #[test]
    fn normal_mode_shows_plain_title() {
        let rendered = render(&state(), &KeyConfig::default());

        assert!(rendered.contains("Headset TUI"));
        assert!(!rendered.contains("simulated device"));
    }

    #[test]
    fn test_mode_shows_simulated_device_title() {
        let mut state = state();
        state.mode = Mode::Test;

        let rendered = render(&state, &KeyConfig::default());

        assert!(rendered.contains("Headset TUI — simulated device"));
    }

    #[test]
    fn battery_line_shows_level_and_device_name() {
        let rendered = render(&state(), &KeyConfig::default());

        assert!(rendered.contains("Battery: 42% - Test Device"));
        assert!(rendered.contains("Sidetone Level"));
        assert!(rendered.contains("42/128"));
    }

    #[test]
    fn battery_error_when_level_is_missing() {
        let mut state = state();
        state.battery = None;

        let rendered = render(&state, &KeyConfig::default());

        assert!(rendered.contains("Battery: Error"));
        assert!(!rendered.contains("42%"));
    }

    #[test]
    fn battery_error_when_level_is_negative() {
        // A negative reading cannot be a percentage, so it renders as an error.
        let mut state = state();
        state.battery = Some(-1);

        let rendered = render(&state, &KeyConfig::default());

        assert!(rendered.contains("Battery: Error"));
        assert!(!rendered.contains("-1%"));
    }

    #[test]
    fn battery_error_is_red() {
        let mut state = state();
        state.battery = None;

        let buffer = render_buffer(&state, &KeyConfig::default(), WIDTH, HEIGHT);

        assert_eq!(color_at(&buffer, "Battery: Error"), Color::Red);
    }

    #[test]
    fn battery_is_green_above_50_percent() {
        let mut state = state();
        state.battery = Some(51);
        state.battery_state = BatteryState::Good;

        let buffer = render_buffer(&state, &KeyConfig::default(), WIDTH, HEIGHT);

        assert_eq!(color_at(&buffer, "Battery: 51%"), Color::Green);
    }

    #[test]
    fn battery_is_yellow_from_15_to_50_percent() {
        for level in [15, 42, 50] {
            let mut state = state();
            state.battery = Some(level);
            state.battery_state = BatteryState::Medium;

            let buffer = render_buffer(&state, &KeyConfig::default(), WIDTH, HEIGHT);
            let needle = format!("Battery: {level}%");

            assert_eq!(color_at(&buffer, &needle), Color::Yellow, "level {level}");
        }
    }

    #[test]
    fn battery_is_red_below_15_percent() {
        for level in [0, 14] {
            let mut state = state();
            state.battery = Some(level);
            state.battery_state = BatteryState::Low;

            let buffer = render_buffer(&state, &KeyConfig::default(), WIDTH, HEIGHT);
            let needle = format!("Battery: {level}%");

            assert_eq!(color_at(&buffer, &needle), Color::Red, "level {level}");
        }
    }

    #[test]
    fn battery_is_blue_while_charging() {
        let mut state = state();
        state.battery = Some(5);
        state.battery_state = BatteryState::Charging;

        let buffer = render_buffer(&state, &KeyConfig::default(), WIDTH, HEIGHT);

        // Charging wins over the level-based color.
        assert_eq!(color_at(&buffer, "Battery: 5%"), Color::Blue);
    }

    #[test]
    fn sidetone_gauge_label_matches_level() {
        for level in [0, 1, 42, 64, 127, 128] {
            let mut state = state();
            state.sidetone = level;

            let rendered = render(&state, &KeyConfig::default());

            assert!(
                rendered.contains(&format!("{level}/128")),
                "missing label for {level} in:\n{rendered}"
            );
        }
    }

    #[test]
    fn sidetone_gauge_handles_boundaries() {
        let mut state = state();
        state.sidetone = 0;
        assert!(render(&state, &KeyConfig::default()).contains("0/128"));

        state.sidetone = SIDETONE_MAX;
        assert!(render(&state, &KeyConfig::default()).contains("128/128"));
    }

    #[test]
    fn sidetone_unavailable_when_unsupported() {
        let mut state = state();
        state.has_sidetone = false;

        let rendered = render(&state, &KeyConfig::default());

        assert!(rendered.contains("Sidetone: not available"));
        assert!(!rendered.contains("42/128"));
        assert!(!rendered.contains("Sidetone Level"));
    }

    #[test]
    fn help_footer_lists_default_keys_in_uppercase() {
        let rendered = buffer_to_string(&render_buffer(
            &state(),
            &KeyConfig::default(),
            WIDE_WIDTH,
            HEIGHT,
        ));

        for expected in [
            "[A] Dec-1",
            "[S] Dec-10",
            "[D] Inc-1",
            "[W] Inc-10",
            "[F] Full",
            "[E] None",
            "[V] Verbose",
            "[C] Config",
            "[Q] Quit",
        ] {
            assert!(
                rendered.contains(expected),
                "{expected:?} missing from help footer:\n{rendered}"
            );
        }
    }

    #[test]
    fn help_footer_follows_rebound_keys() {
        let rendered =
            buffer_to_string(&render_buffer(&state(), &custom_keys(), WIDE_WIDTH, HEIGHT));

        for expected in [
            "[J] Dec-1",
            "[K] Dec-10",
            "[L] Inc-1",
            "[I] Inc-10",
            "[O] Full",
            "[U] None",
            "[H] Verbose",
            "[Y] Config",
            "[X] Quit",
        ] {
            assert!(rendered.contains(expected), "{expected:?} missing");
        }

        // The default letters are no longer advertised.
        for stale in ["[A] Dec-1", "[S] Dec-10", "[Q] Quit"] {
            assert!(!rendered.contains(stale), "{stale:?} should be gone");
        }
    }

    #[test]
    fn renders_in_a_small_terminal() {
        for has_sidetone in [true, false] {
            let mut state = state();
            state.has_sidetone = has_sidetone;

            let buffer = render_buffer(&state, &KeyConfig::default(), 40, 16);
            let rendered = buffer_to_string(&buffer);

            assert!(rendered.contains("Headset TUI"));
            assert!(rendered.contains("Battery: 42%"));
        }
    }

    #[test]
    fn garbage_device_names_still_render() {
        let names = [
            String::new(),
            "  ".to_string(),
            "🎧 Ünïcödé 日本語 \"quoted\" — name".to_string(),
            "x".repeat(500),
        ];

        for name in names {
            let mut state = state();
            state.device_name = name;

            let buffer = render_buffer(&state, &KeyConfig::default(), WIDTH, HEIGHT);
            let rendered = buffer_to_string(&buffer);

            assert!(rendered.contains("Battery: 42%"));
            assert!(rendered.contains("Headset TUI"));
            assert_eq!(buffer.area().width, WIDTH);
        }
    }
}
