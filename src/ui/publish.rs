use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::widgets::{
    centered_rect, dialog_key_hint, render_multiline_field, render_qos_field, render_retain_field,
    render_text_field,
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
    let mut hints = Vec::new();
    hints.extend(dialog_key_hint("Enter", "Publish"));
    hints.extend(dialog_key_hint("Tab", "Next"));
    hints.extend(dialog_key_hint("^S", "Bookmark"));
    hints.extend(dialog_key_hint("Esc", "Cancel"));
    frame.render_widget(
        Paragraph::new(Line::from(hints)),
        chunks[3],
    );
}
