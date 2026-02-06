use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::widgets::centered_rect;
use crate::app::App;

pub fn render_filter(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Topic Filter (MQTT wildcards: + # ) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(inner);

    // Instructions
    let instructions = Paragraph::new(Line::from(vec![
        Span::raw("Enter pattern: "),
        Span::styled("+", Style::default().fg(Color::Cyan)),
        Span::raw(" = single level, "),
        Span::styled("#", Style::default().fg(Color::Cyan)),
        Span::raw(" = multi-level"),
    ]));
    frame.render_widget(instructions, chunks[0]);

    // Input field with cursor
    let input_display = format!("{}_", app.filter_input);
    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Yellow)),
        Span::styled(
            input_display,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    frame.render_widget(input, chunks[1]);

    // Examples
    let examples = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Examples: ",
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(vec![
            Span::styled("  telemetry/#       ", Style::default().fg(Color::Cyan)),
            Span::styled("All telemetry", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  telemetry/+/meter ", Style::default().fg(Color::Cyan)),
            Span::styled("Any device's meter", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  sites/+/devices/# ", Style::default().fg(Color::Cyan)),
            Span::styled("All site devices", Style::default().fg(Color::DarkGray)),
        ]),
    ]);
    frame.render_widget(examples, chunks[3]);

    // Footer hint
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" apply  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel  "),
        Span::styled("(empty)", Style::default().fg(Color::DarkGray)),
        Span::raw(" clears filter"),
    ]));
    frame.render_widget(footer, chunks[2]);
}
