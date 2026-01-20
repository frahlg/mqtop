use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_help(frame: &mut Frame) {
    let area = centered_rect(70, 80, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" mqtop - Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let help_text = vec![
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑/↓, j/k    ", Style::default().fg(Color::Yellow)),
            Span::raw("Move up/down"),
        ]),
        Line::from(vec![
            Span::styled("  ←/→, h/l    ", Style::default().fg(Color::Yellow)),
            Span::raw("Collapse/Expand or move to parent"),
        ]),
        Line::from(vec![
            Span::styled("  Enter       ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle expand/collapse"),
        ]),
        Line::from(vec![
            Span::styled("  Tab         ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch panel (Topics → Messages → Stats)"),
        ]),
        Line::from(vec![
            Span::styled("  1/2/3       ", Style::default().fg(Color::Yellow)),
            Span::raw("Jump to panel directly"),
        ]),
        Line::from(vec![
            Span::styled("  PgUp/PgDn   ", Style::default().fg(Color::Yellow)),
            Span::raw("Page up/down"),
        ]),
        Line::from(vec![
            Span::styled("  g/G         ", Style::default().fg(Color::Yellow)),
            Span::raw("Go to top/bottom"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Search & Filter",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  /           ", Style::default().fg(Color::Yellow)),
            Span::raw("Open search"),
        ]),
        Line::from(vec![
            Span::styled("  S           ", Style::default().fg(Color::Yellow)),
            Span::raw("Manage MQTT servers"),
        ]),
        Line::from(vec![
            Span::styled("  s           ", Style::default().fg(Color::Yellow)),
            Span::raw("Star/unstar current topic"),
        ]),
        Line::from(vec![
            Span::styled("  *           ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle starred topics filter"),
        ]),
        Line::from(vec![
            Span::styled("  f           ", Style::default().fg(Color::Yellow)),
            Span::raw("Set topic filter (MQTT wildcards)"),
        ]),
        Line::from(vec![
            Span::styled("  ↑↓ / jk    ", Style::default().fg(Color::Yellow)),
            Span::raw("Navigate topics/messages"),
        ]),
        Line::from(vec![
            Span::styled("  ←→ / hl    ", Style::default().fg(Color::Yellow)),
            Span::raw("Collapse/expand or move into child"),
        ]),
        Line::from(vec![
            Span::styled("  H / L      ", Style::default().fg(Color::Yellow)),
            Span::raw("Collapse/expand full branch"),
        ]),
        Line::from(vec![
            Span::styled("  m           ", Style::default().fg(Color::Yellow)),
            Span::raw("Track metric from current message"),
        ]),
        Line::from(vec![
            Span::styled("  P           ", Style::default().fg(Color::Yellow)),
            Span::raw("Open publish dialog"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+P      ", Style::default().fg(Color::Yellow)),
            Span::raw("Copy message to publish dialog"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+S      ", Style::default().fg(Color::Yellow)),
            Span::raw("Save publish as bookmark (in publish dialog)"),
        ]),
        Line::from(vec![
            Span::styled("  B           ", Style::default().fg(Color::Yellow)),
            Span::raw("Open bookmark manager"),
        ]),
        Line::from(vec![
            Span::styled("  y           ", Style::default().fg(Color::Yellow)),
            Span::raw("Copy topic to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  Y           ", Style::default().fg(Color::Yellow)),
            Span::raw("Copy payload to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  Esc         ", Style::default().fg(Color::Yellow)),
            Span::raw("Cancel search / Close help"),
        ]),
        Line::from(vec![
            Span::styled("  Enter       ", Style::default().fg(Color::Yellow)),
            Span::raw("Select search result"),
        ]),
        Line::from(vec![
            Span::styled("  PgUp/PgDn   ", Style::default().fg(Color::Yellow)),
            Span::raw("Scroll search results"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Display",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  p           ", Style::default().fg(Color::Yellow)),
            Span::raw("Cycle payload mode (Auto → Raw → Hex → JSON)"),
        ]),
        Line::from(vec![
            Span::styled("  c           ", Style::default().fg(Color::Yellow)),
            Span::raw("Clear statistics"),
        ]),
        Line::from(vec![
            Span::styled("  ?           ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "General",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  q, Ctrl+C   ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Topic Colors",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Topic colors are configurable via "),
            Span::styled("[[ui.topic_colors]]", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::raw("  in config.toml. UUIDs/IDs are shown in "),
            Span::styled("gray", Style::default().fg(Color::Gray)),
            Span::raw("."),
        ]),
    ];

    let paragraph = Paragraph::new(help_text);
    frame.render_widget(paragraph, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
