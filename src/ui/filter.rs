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

    let broker = app.connected_broker_kind;
    let single_wc = broker.wildcard_single();
    let multi_wc = broker.wildcard_multi();
    let sep = broker.topic_separator();
    let hint = broker.filter_title_hint();

    let block = Block::default()
        .title(" Topic Filter ")
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
        Span::styled(single_wc.to_string(), Style::default().fg(Color::Cyan)),
        Span::raw(" = single level, "),
        Span::styled(multi_wc.to_string(), Style::default().fg(Color::Cyan)),
        Span::raw(" = multi-level"),
        Span::raw("  "),
        Span::styled(format!("({})", hint), Style::default().fg(Color::DarkGray)),
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
            Span::styled(
                format!("  telemetry{}{}       ", sep, multi_wc),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("All telemetry", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  telemetry{}{}{}meter ", sep, single_wc, sep),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("Any device's meter", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  sites{}{}{}devices{}{} ", sep, single_wc, sep, sep, multi_wc),
                Style::default().fg(Color::Cyan),
            ),
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
