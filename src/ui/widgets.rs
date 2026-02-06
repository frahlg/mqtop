use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Create a centered popup rectangle within a given area
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

/// Safely truncate a string at a valid UTF-8 character boundary
pub fn truncate_safe(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Render a single-line text input field with a blinking block cursor
pub fn render_text_field(
    frame: &mut Frame,
    label: &str,
    value: &str,
    cursor: usize,
    focused: bool,
    area: Rect,
) {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(format!(" {} ", label))
        .borders(Borders::ALL)
        .border_style(style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if focused {
        let cursor_pos = cursor.min(value.len());
        let (before, after) = value.split_at(cursor_pos);
        let line = Line::from(vec![
            Span::styled(before.to_string(), Style::default().fg(Color::White)),
            Span::styled(
                "\u{258c}",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
            Span::styled(after.to_string(), Style::default().fg(Color::White)),
        ]);
        frame.render_widget(Paragraph::new(line), inner);
    } else {
        let text = Paragraph::new(value.to_string()).style(Style::default().fg(Color::Gray));
        frame.render_widget(text, inner);
    }
}

/// Render a multi-line text input field with a blinking block cursor
pub fn render_multiline_field(
    frame: &mut Frame,
    label: &str,
    value: &str,
    cursor: usize,
    focused: bool,
    area: Rect,
) {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(format!(" {} ", label))
        .borders(Borders::ALL)
        .border_style(style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if focused {
        let cursor_pos = cursor.min(value.len());
        let (before, after) = value.split_at(cursor_pos);
        let line = Line::from(vec![
            Span::styled(before.to_string(), Style::default().fg(Color::White)),
            Span::styled(
                "\u{258c}",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
            Span::styled(after.to_string(), Style::default().fg(Color::White)),
        ]);
        let paragraph = Paragraph::new(line).wrap(ratatui::widgets::Wrap { trim: false });
        frame.render_widget(paragraph, inner);
    } else {
        let text = Paragraph::new(value.to_string())
            .style(Style::default().fg(Color::Gray))
            .wrap(ratatui::widgets::Wrap { trim: false });
        frame.render_widget(text, inner);
    }
}

/// Render a QoS selector field (0, 1, 2)
pub fn render_qos_field(frame: &mut Frame, qos: u8, focused: bool, area: Rect) {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" QoS ")
        .borders(Borders::ALL)
        .border_style(style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let qos_text = Line::from(vec![
        Span::styled(
            " 0 ",
            if qos == 0 {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::raw(" "),
        Span::styled(
            " 1 ",
            if qos == 1 {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::raw(" "),
        Span::styled(
            " 2 ",
            if qos == 2 {
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
    ]);

    let hint = if focused {
        Line::from(Span::styled(
            "Space/0/1/2",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        Line::from("")
    };

    let text = Paragraph::new(vec![qos_text, hint]);
    frame.render_widget(text, inner);
}

/// Render a retain toggle field
pub fn render_retain_field(frame: &mut Frame, retain: bool, focused: bool, area: Rect) {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Retain ")
        .borders(Borders::ALL)
        .border_style(style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let retain_text = if retain {
        Line::from(Span::styled(
            " ON ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        ))
    } else {
        Line::from(Span::styled(
            " OFF ",
            Style::default().fg(Color::DarkGray),
        ))
    };

    let hint = if focused {
        Line::from(Span::styled(
            "Space to toggle",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        Line::from("")
    };

    let text = Paragraph::new(vec![retain_text, hint]);
    frame.render_widget(text, inner);
}

/// Format a key hint span pair (e.g., "q" : "Quit")
pub fn key_hint(key: &str, action: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            key.to_string(),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {} ", action), Style::default().fg(Color::DarkGray)),
    ]
}

/// Format a key hint for dialog footers (brighter keys)
pub fn dialog_key_hint(key: &str, action: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(key.to_string(), Style::default().fg(Color::Yellow)),
        Span::styled(
            format!(" {}  ", action),
            Style::default().fg(Color::DarkGray),
        ),
    ]
}
