use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
    Frame,
};

use super::bordered_block;
use crate::app::{App, FilterMode, Panel};
use crate::config::TopicColorRule;
use crate::state::TopicInfo;

pub fn render_tree(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focused_panel == Panel::TopicTree;

    // Build title with filter/star badges
    let title = match app.filter_mode {
        FilterMode::All => {
            if app.topic_filter.is_some() {
                "Topics [filtered]"
            } else {
                "Topics"
            }
        }
        FilterMode::Starred => "Topics [★]",
    };
    let block = bordered_block(title, focused);
    let inner = block.inner(area);

    frame.render_widget(block, area);

    let topics = app.get_visible_topics();

    if topics.is_empty() {
        let empty_msg = if app.topic_filter.is_some() {
            "No topics match filter"
        } else {
            "Waiting for messages..."
        };
        let text = Line::from(Span::styled(
            empty_msg,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));
        frame.render_widget(List::new(vec![ListItem::new(text)]), inner);
        return;
    }

    // Calculate visible window and ensure selection is visible
    let visible_height = inner.height as usize;
    let total = topics.len();
    let selected = app.selected_topic_index.min(total.saturating_sub(1));

    // Ensure scroll keeps selection in view
    if selected < app.tree_scroll {
        app.tree_scroll = selected;
    } else if selected >= app.tree_scroll + visible_height {
        app.tree_scroll = selected.saturating_sub(visible_height.saturating_sub(1));
    }

    let color_rules = &app.config.ui.topic_colors;
    let now_ms = chrono::Utc::now().timestamp_millis();

    let items: Vec<ListItem> = topics
        .iter()
        .enumerate()
        .map(|(i, topic)| {
            let is_selected = i == app.selected_topic_index;
            let is_starred = app.is_starred(&topic.full_path);
            create_topic_item(topic, is_selected, focused, is_starred, color_rules, now_ms)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_topic_index));
    *state.offset_mut() = app.tree_scroll;

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(if focused {
                Color::DarkGray
            } else {
                Color::Black
            })
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(list, inner, &mut state);
}

fn create_topic_item(
    topic: &TopicInfo,
    is_selected: bool,
    focused: bool,
    is_starred: bool,
    color_rules: &[TopicColorRule],
    now_ms: i64,
) -> ListItem<'static> {
    let indent = "  ".repeat(topic.depth);

    // Star indicator
    let star = if is_starred { "★ " } else { "" };

    // Determine icon based on topic type and state
    let icon = if topic.has_children {
        if topic.is_expanded {
            "▾ "
        } else {
            "▸ "
        }
    } else {
        "· "
    };

    // Activity indicator based on message recency
    let activity = topic.last_message_time.map(|t| {
        let age_ms = now_ms - t;
        if age_ms < 1_000 {
            ("●", Color::Green) // < 1 second ago: bright green
        } else if age_ms < 5_000 {
            ("●", Color::Yellow) // < 5 seconds: yellow
        } else if age_ms < 30_000 {
            ("○", Color::DarkGray) // < 30 seconds: fading
        } else {
            ("", Color::DarkGray) // older: no indicator
        }
    });

    // Color code by topic segment using config rules
    let segment_color = get_topic_color(&topic.segment, &topic.full_path, color_rules);

    // Format message count
    let count_str = if topic.message_count > 0 {
        format!(" {}", format_count(topic.message_count))
    } else {
        String::new()
    };

    let style = if is_selected && focused {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(segment_color)
    };

    let mut spans = vec![
        Span::raw(indent),
        Span::styled(star.to_string(), Style::default().fg(Color::Yellow)),
        Span::styled(icon.to_string(), Style::default().fg(Color::DarkGray)),
        Span::styled(topic.segment.clone(), style),
        Span::styled(count_str, Style::default().fg(Color::DarkGray)),
    ];

    // Add activity dot at the end
    if let Some((indicator, color)) = activity {
        if !indicator.is_empty() {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                indicator.to_string(),
                Style::default().fg(color),
            ));
        }
    }

    ListItem::new(Line::from(spans))
}

/// Get color based on topic segment using configurable rules
fn get_topic_color(segment: &str, full_path: &str, color_rules: &[TopicColorRule]) -> Color {
    // Check config-based color rules first
    for rule in color_rules {
        if rule.matches(segment, full_path) {
            return rule.to_color();
        }
    }

    // Fallback: UUIDs/IDs in gray, everything else white
    if is_uuid_like(segment) {
        Color::Gray
    } else {
        Color::White
    }
}

/// Check if a string looks like a UUID or ID
fn is_uuid_like(s: &str) -> bool {
    // Check for UUID format or long alphanumeric strings
    s.len() >= 8
        && s.chars().all(|c| c.is_alphanumeric() || c == '-')
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
