use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
    Frame,
};

use crate::app::{App, FilterMode, Panel};
use crate::state::TopicInfo;
use super::bordered_block;

pub fn render_tree(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == Panel::TopicTree;
    let title = match app.filter_mode {
        FilterMode::All => "Topics",
        FilterMode::Starred => "★ Starred",
    };
    let block = bordered_block(title, focused);
    let inner = block.inner(area);

    frame.render_widget(block, area);

    let topics = app.get_visible_topics();

    if topics.is_empty() {
        let empty_msg = Line::from(Span::styled(
            "No topics yet...",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        ));
        frame.render_widget(
            List::new(vec![ListItem::new(empty_msg)]),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = topics
        .iter()
        .enumerate()
        .map(|(i, topic)| {
            let is_selected = i == app.selected_topic_index;
            let is_starred = app.is_starred(&topic.full_path);
            create_topic_item(topic, is_selected, focused, is_starred)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_topic_index));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(if focused { Color::DarkGray } else { Color::Black })
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, inner, &mut state);
}

fn create_topic_item(topic: &TopicInfo, is_selected: bool, focused: bool, is_starred: bool) -> ListItem<'static> {
    let indent = "  ".repeat(topic.depth);

    // Star indicator
    let star = if is_starred { "★ " } else { "" };

    // Determine icon based on topic type and state
    let icon = if topic.has_children {
        if topic.is_expanded { "▼ " } else { "▶ " }
    } else {
        "  "
    };

    // Color code by topic segment for Sourceful entities
    let segment_color = get_topic_color(&topic.segment, &topic.full_path);

    // Format message count
    let count_str = if topic.message_count > 0 {
        format!(" ({})", format_count(topic.message_count))
    } else {
        String::new()
    };

    let style = if is_selected && focused {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(segment_color)
    };

    let line = Line::from(vec![
        Span::raw(indent),
        Span::styled(star, Style::default().fg(Color::Yellow)),
        Span::styled(icon, Style::default().fg(Color::DarkGray)),
        Span::styled(topic.segment.clone(), style),
        Span::styled(count_str, Style::default().fg(Color::DarkGray)),
    ]);

    ListItem::new(line)
}

/// Get color based on topic segment for Sourceful-specific highlighting
fn get_topic_color(segment: &str, full_path: &str) -> Color {
    // Sourceful entity colors (Wallet → Site → Device → DER hierarchy)
    let segment_lower = segment.to_lowercase();
    let path_lower = full_path.to_lowercase();

    if segment_lower == "wallets" || path_lower.starts_with("wallets") || path_lower.contains("/wallets/") {
        Color::LightRed  // Wallets at top of hierarchy
    } else if segment_lower == "sites" || path_lower.starts_with("sites") || path_lower.contains("/sites/") {
        Color::Cyan
    } else if segment_lower == "devices" || path_lower.contains("/devices/") {
        Color::Green
    } else if segment_lower == "ders" || path_lower.contains("/ders/") {
        Color::Yellow
    } else if segment_lower == "telemetry" || path_lower.starts_with("telemetry") {
        Color::Magenta
    } else if segment_lower == "ems" || path_lower.starts_with("ems") {
        Color::Blue
    } else if segment_lower == "optimizer" || path_lower.contains("optimizer") {
        Color::LightBlue
    } else if segment_lower == "meta" || segment_lower == "metadata" {
        Color::LightCyan
    } else if is_uuid_like(segment) {
        Color::Gray  // UUIDs/IDs in gray
    } else {
        Color::White
    }
}

/// Check if a string looks like a UUID or ID
fn is_uuid_like(s: &str) -> bool {
    // Check for UUID format or long alphanumeric strings
    s.len() >= 8 && s.chars().all(|c| c.is_alphanumeric() || c == '-')
        && s.chars().filter(|c| c.is_numeric()).count() > 2
}

fn format_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}
