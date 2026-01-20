use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, PublishField};

pub fn render_publish(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Publish Message ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    // Create layout for fields
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Topic
            Constraint::Min(5),    // Payload
            Constraint::Length(3), // QoS + Retain
            Constraint::Length(2), // Help text
        ])
        .split(inner);

    // Topic field
    render_text_field(
        frame,
        "Topic",
        &app.publish_edit.topic,
        app.publish_edit.cursor,
        app.publish_edit.field == PublishField::Topic,
        chunks[0],
    );

    // Payload field (multi-line)
    render_multiline_field(
        frame,
        "Payload",
        &app.publish_edit.payload,
        app.publish_edit.cursor,
        app.publish_edit.field == PublishField::Payload,
        chunks[1],
    );

    // QoS and Retain fields on same row
    let options_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    render_qos_field(
        frame,
        app.publish_edit.qos,
        app.publish_edit.field == PublishField::Qos,
        options_chunks[0],
    );

    render_retain_field(
        frame,
        app.publish_edit.retain,
        app.publish_edit.field == PublishField::Retain,
        options_chunks[1],
    );

    // Help text
    let help = Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(": Publish  "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(": Next  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(": Cancel"),
    ]);
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );
}

fn render_text_field(
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

    // Show cursor in value
    let display_value = if focused {
        let mut chars: Vec<char> = value.chars().collect();
        let cursor_pos = cursor.min(chars.len());
        chars.insert(cursor_pos, '|');
        chars.into_iter().collect()
    } else {
        value.to_string()
    };

    let text = Paragraph::new(display_value).style(if focused {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::Gray)
    });

    frame.render_widget(text, inner);
}

fn render_multiline_field(
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

    // Show cursor in value
    let display_value = if focused {
        let mut chars: Vec<char> = value.chars().collect();
        let cursor_pos = cursor.min(chars.len());
        chars.insert(cursor_pos, '|');
        chars.into_iter().collect()
    } else {
        value.to_string()
    };

    let text = Paragraph::new(display_value)
        .style(if focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        })
        .wrap(ratatui::widgets::Wrap { trim: false });

    frame.render_widget(text, inner);
}

fn render_qos_field(frame: &mut Frame, qos: u8, focused: bool, area: Rect) {
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
            " [0] ",
            if qos == 0 {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::styled(
            " [1] ",
            if qos == 1 {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::styled(
            " [2] ",
            if qos == 2 {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
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

fn render_retain_field(frame: &mut Frame, retain: bool, focused: bool, area: Rect) {
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
            " [ON] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::styled(
            " [OFF] ",
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
