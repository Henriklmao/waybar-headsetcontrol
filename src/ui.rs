use crate::config::KeyConfig;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

pub struct UiState {
    pub sidetone: u8,
    pub battery: i32,
    pub battery_status: String,
    pub verbose_mode: bool,
    pub device_name: String,
    pub has_sidetone: bool,
}

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
    let title = Paragraph::new("Waybar Headsetcontrol TUI")
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(title, chunks[0]);

    // Battery status
    let battery_color = if state.battery > 50 {
        Color::Green
    } else if state.battery >= 15 {
        Color::Yellow
    } else {
        Color::Red
    };

    let battery_text = if state.battery >= 0 {
        Line::from(vec![Span::styled(
            format!("󰋎 Battery: {}% - {}", state.battery, state.device_name),
            Style::default().fg(battery_color),
        )])
    } else {
        Line::from(vec![Span::styled(
            "󰋎 Battery: Error",
            Style::default().fg(Color::Red),
        )])
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
            .percent((state.sidetone as f64 / 128.0 * 100.0) as u16)
            .label(format!("{}/128", state.sidetone));
        f.render_widget(gauge, chunks[2]);
    } else {
        let text = Paragraph::new("Sidetone: not available")
            .block(Block::default().borders(Borders::ALL).title("Sidetone"));
        f.render_widget(text, chunks[2]);
    }

    // Help text
    let help_text = format!(
        "[{}] Dec-1  [{}] Dec-10  [{}] Inc-1  [{}] Inc-10  [{}] Full  [{}] None  [{}] Config  [{}] Quit",
        keys.dec1.to_uppercase(),
        keys.dec10.to_uppercase(),
        keys.inc1.to_uppercase(),
        keys.inc10.to_uppercase(),
        keys.full.to_uppercase(),
        keys.none.to_uppercase(),
        keys.config.to_uppercase(),
        keys.quit.to_uppercase(),
    );

    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[3]);
}
