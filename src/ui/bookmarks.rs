use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::widgets::{
    centered_rect, dialog_key_hint, render_multiline_field, render_qos_field, render_retain_field,
    render_text_field, truncate_safe,
};
use crate::app::{App, BookmarkField};

pub fn render_bookmark_manager(frame: &mut Frame, app: &App) {
    // If editing, show the edit dialog instead
    if app.bookmark_manager.editing.is_some() {
        render_bookmark_edit(frame, app);
        return;
    }

    let area = centered_rect(60, 70, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Bookmarks ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    // Layout: list area + help text
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(inner);

    // Group bookmarks by category
    let bookmarks = &app.user_data.bookmarks;

    if bookmarks.is_empty() {
        let empty_msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No bookmarks yet",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'a' to add a new bookmark",
                Style::default().fg(Color::Yellow),
            )),
        ])
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(empty_msg, chunks[0]);
    } else {
        // Build list items grouped by category
        let mut items: Vec<ListItem> = Vec::new();
        let mut current_category: Option<Option<&String>> = None;

        // Sort bookmarks by category for display
        let mut indexed_bookmarks: Vec<(usize, &crate::persistence::Bookmark)> =
            bookmarks.iter().enumerate().collect();
        indexed_bookmarks.sort_by(|a, b| {
            let cat_a = a.1.category.as_deref().unwrap_or("");
            let cat_b = b.1.category.as_deref().unwrap_or("");
            cat_a.cmp(cat_b)
        });

        // Map original indices to display indices
        let mut display_to_original: Vec<usize> = Vec::new();

        for (original_idx, bookmark) in &indexed_bookmarks {
            let cat = bookmark.category.as_ref();

            // Add category header if changed
            if current_category != Some(cat) {
                current_category = Some(cat);
                let cat_name = cat.map(|s| s.as_str()).unwrap_or("uncategorized");
                items.push(ListItem::new(Line::from(Span::styled(
                    format!("[{}]", cat_name),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))));
            }

            let is_selected = app.bookmark_manager.selected_index == *original_idx;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if is_selected { "▶ " } else { "  " };

            // Truncate topic if too long (safely handling UTF-8)
            let max_topic_len = 30;
            let topic_display = if bookmark.topic.len() > max_topic_len {
                format!("{}...", truncate_safe(&bookmark.topic, max_topic_len - 3))
            } else {
                bookmark.topic.clone()
            };

            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&bookmark.name, style),
                Span::styled("  ", Style::default()),
                Span::styled(topic_display, Style::default().fg(Color::DarkGray)),
            ]);

            items.push(ListItem::new(line));
            display_to_original.push(*original_idx);
        }

        let list = List::new(items);
        frame.render_widget(list, chunks[0]);
    }

    // Help text
    let mut hints = Vec::new();
    hints.extend(dialog_key_hint("Enter", "Publish"));
    hints.extend(dialog_key_hint("e", "Edit"));
    hints.extend(dialog_key_hint("a", "Add"));
    hints.extend(dialog_key_hint("d", "Delete"));
    hints.extend(dialog_key_hint("Esc", "Close"));
    frame.render_widget(Paragraph::new(Line::from(hints)), chunks[1]);
}

fn render_bookmark_edit(frame: &mut Frame, app: &App) {
    let editing = match &app.bookmark_manager.editing {
        Some(e) => e,
        None => return,
    };

    let area = centered_rect(60, 65, frame.area());

    frame.render_widget(Clear, area);

    let title = if editing.is_new {
        " New Bookmark "
    } else {
        " Edit Bookmark "
    };

    let block = Block::default()
        .title(title)
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
            Constraint::Length(3), // Name
            Constraint::Length(3), // Category
            Constraint::Length(3), // Topic
            Constraint::Min(5),    // Payload
            Constraint::Length(3), // QoS + Retain
            Constraint::Length(2), // Help text
        ])
        .split(inner);

    // Name field
    render_text_field(
        frame,
        "Name",
        &editing.name,
        editing.cursor,
        editing.field == BookmarkField::Name,
        chunks[0],
    );

    // Category field
    render_text_field(
        frame,
        "Category",
        &editing.category,
        editing.cursor,
        editing.field == BookmarkField::Category,
        chunks[1],
    );

    // Topic field
    render_text_field(
        frame,
        "Topic",
        &editing.topic,
        editing.cursor,
        editing.field == BookmarkField::Topic,
        chunks[2],
    );

    // Payload field (multi-line)
    render_multiline_field(
        frame,
        "Payload",
        &editing.payload,
        editing.cursor,
        editing.field == BookmarkField::Payload,
        chunks[3],
    );

    // QoS and Retain fields on same row
    let options_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[4]);

    render_qos_field(
        frame,
        editing.qos,
        editing.field == BookmarkField::Qos,
        options_chunks[0],
    );

    render_retain_field(
        frame,
        editing.retain,
        editing.field == BookmarkField::Retain,
        options_chunks[1],
    );

    // Help text
    let mut hints = Vec::new();
    hints.extend(dialog_key_hint("Enter", "Save"));
    hints.extend(dialog_key_hint("Tab", "Next"));
    hints.extend(dialog_key_hint("Esc", "Cancel"));
    frame.render_widget(Paragraph::new(Line::from(hints)), chunks[5]);
}
