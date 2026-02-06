use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::widgets::centered_rect;
use crate::app::App;

pub fn render_search(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, frame.area());

    // Clear the area behind the popup
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Search Topics ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    // Layout: search input + results
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input
            Constraint::Min(3),    // Results
        ])
        .split(inner);

    // Search input
    let input_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let input_text = Line::from(vec![
        Span::styled("/ ", Style::default().fg(Color::Cyan)),
        Span::raw(&app.search_query),
        Span::styled(
            "▌",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::SLOW_BLINK),
        ),
    ]);

    let input = Paragraph::new(input_text).block(input_block);
    frame.render_widget(input, chunks[0]);

    // Results
    if app.search_results.is_empty() && !app.search_query.is_empty() {
        let no_results = Paragraph::new(Span::styled(
            "No matching topics",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ))
        .alignment(Alignment::Center);
        frame.render_widget(no_results, chunks[1]);
    } else if !app.search_results.is_empty() {
        let visible_height = chunks[1].height.saturating_sub(1) as usize;
        let total = app.search_results.len();
        let window = visible_height.max(1);
        let max_start = total.saturating_sub(window);
        let start = app.search_scroll.min(max_start);
        let end = (start + window).min(total);

        let items: Vec<ListItem> = app
            .search_results
            .iter()
            .enumerate()
            .skip(start)
            .take(end.saturating_sub(start))
            .map(|(i, topic)| {
                let is_selected = i == app.search_result_index;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let highlighted = highlight_match(topic, &app.search_query);

                let prefix = if is_selected { "▶ " } else { "  " };
                let mut spans = vec![Span::styled(prefix, style)];
                spans.extend(highlighted);

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, chunks[1]);

        let count_text = format!("{}/{}", app.search_result_index + 1, total);
        let more = Paragraph::new(Span::styled(
            count_text,
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Right);
        let count_area = Rect {
            y: chunks[1].y + chunks[1].height.saturating_sub(1),
            height: 1,
            ..chunks[1]
        };
        frame.render_widget(more, count_area);
    } else {
        // Empty search - show hint
        let hint = Paragraph::new(vec![
            Line::from(Span::styled(
                "Type to search topics...",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Tips: ", Style::default().fg(Color::Cyan)),
                Span::raw("Search by device ID, site, or topic path"),
            ]),
            Line::from(vec![
                Span::raw("  • "),
                Span::styled("zap-", Style::default().fg(Color::Green)),
                Span::raw(" - Find Zap devices"),
            ]),
            Line::from(vec![
                Span::raw("  • "),
                Span::styled("meter", Style::default().fg(Color::Green)),
                Span::raw(" - Find meter topics"),
            ]),
            Line::from(vec![
                Span::raw("  • "),
                Span::styled("sites", Style::default().fg(Color::Green)),
                Span::raw(" - Find site topics"),
            ]),
        ]);
        frame.render_widget(hint, chunks[1]);
    }
}

fn highlight_match(text: &str, query: &str) -> Vec<Span<'static>> {
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    if let Some(start) = text_lower.find(&query_lower) {
        let end = start + query.len();
        vec![
            Span::raw(text[..start].to_string()),
            Span::styled(
                text[start..end].to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(text[end..].to_string()),
        ]
    } else {
        vec![Span::raw(text.to_string())]
    }
}
