use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
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
    let help = Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(":Publish  "),
        Span::styled("e", Style::default().fg(Color::Yellow)),
        Span::raw(":Edit  "),
        Span::styled("a", Style::default().fg(Color::Yellow)),
        Span::raw(":Add  "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(":Delete  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(":Close"),
    ]);
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
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
    let help = Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(": Save  "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(": Next  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(": Cancel"),
    ]);
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        chunks[5],
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

/// Safely truncate a string at a valid UTF-8 character boundary
fn truncate_safe(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
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
