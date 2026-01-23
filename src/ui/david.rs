use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Render the David easter egg - Terry Pratchett themed MQTT musings from Death
pub fn render_david_easter_egg(frame: &mut Frame) {
    let area = centered_rect(70, 80, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" A BRIEF TREATISE ON MESSAGE QUEUING ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "I FIND MQTT QUITE FASCINATING.",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "MESSAGES QUEUE UP, WAITING THEIR TURN. MUCH LIKE ",
                Style::default().fg(Color::White),
            ),
            Span::styled(
                "SOULS",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::ITALIC),
            ),
            Span::styled(",", Style::default().fg(Color::White)),
        ]),
        Line::from(Span::styled(
            "REALLY. ALTHOUGH SOULS RARELY HAVE A QUALITY OF SERVICE",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "SETTING. THAT WOULD MAKE MY JOB CONSIDERABLY EASIER.*",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  * Death had once tried to implement QoS levels for the",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(Span::styled(
            "    afterlife. QoS 0 (\"at most once\") proved unpopular.",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "The Librarian has asked me to note that \"Ook\" is a perfectly",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "valid MQTT topic, and anyone who disagrees will be hit with",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "a very large dictionary.**",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  ** The OED. Hardcover. Repeatedly.",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "THERE IS NO FATE BUT WHAT WE SUBSCRIBE TO.",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "                                    -- GNU Terry Pratchett",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "                              [Press Esc to return to reality]",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

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
