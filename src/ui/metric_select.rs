use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::widgets::centered_rect;
use crate::app::App;

pub fn render_metric_select(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 60, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Select Metric to Track ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    if app.available_fields.is_empty() {
        let msg = Paragraph::new("No numeric fields available");
        frame.render_widget(msg, inner);
        return;
    }

    // Header
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(inner);

    let header = Paragraph::new(Line::from(vec![
        Span::raw("Select a field to track ("),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" to confirm, "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" to cancel)"),
    ]));
    frame.render_widget(header, chunks[0]);

    // Field list
    let items: Vec<ListItem> = app
        .available_fields
        .iter()
        .enumerate()
        .map(|(i, (field, value))| {
            let is_selected = i == app.metric_select_index;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if is_selected { "▶ " } else { "  " };

            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(field.clone(), style),
                Span::raw(" = "),
                Span::styled(format_value(*value), Style::default().fg(Color::Cyan)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    // Footer hint
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑↓/jk", Style::default().fg(Color::DarkGray)),
        Span::raw(" navigate  "),
        Span::styled("Enter", Style::default().fg(Color::DarkGray)),
        Span::raw(" select"),
    ]));
    frame.render_widget(footer, chunks[2]);
}

fn format_value(v: f64) -> String {
    if v.abs() >= 1000000.0 {
        format!("{:.2}M", v / 1000000.0)
    } else if v.abs() >= 1000.0 {
        format!("{:.2}k", v / 1000.0)
    } else if v.fract() == 0.0 {
        format!("{:.0}", v)
    } else {
        format!("{:.2}", v)
    }
}

