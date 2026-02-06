mod bookmarks;
mod david;
mod filter;
mod help;
mod message_view;
mod metric_select;
mod publish;
mod search;
mod server_manager;
mod stats_view;
mod tree_view;
pub mod widgets;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, InputMode, Panel};
use widgets::key_hint;

pub use bookmarks::render_bookmark_manager;
pub use filter::render_filter;
pub use help::render_help;
pub use message_view::render_messages;
pub use metric_select::render_metric_select;
pub use publish::render_publish;
pub use search::render_search;
pub use server_manager::render_server_manager;
pub use stats_view::render_stats;
pub use tree_view::render_tree;

/// Main render function
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Create main layout: header, content, footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(size);

    render_header(frame, app, main_chunks[0]);

    let show_three_panels = size.width >= 110 && size.height >= 12;
    let show_two_panels = size.width >= 80 && size.height >= 10;

    if show_three_panels {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(45),
                Constraint::Percentage(25),
            ])
            .split(main_chunks[1]);

        render_tree(frame, app, content_chunks[0]);
        render_messages(frame, app, content_chunks[1]);
        render_stats(frame, app, content_chunks[2]);
    } else if show_two_panels {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(main_chunks[1]);

        match app.focused_panel {
            Panel::Stats => {
                render_messages(frame, app, content_chunks[0]);
                render_stats(frame, app, content_chunks[1]);
            }
            Panel::Messages => {
                render_tree(frame, app, content_chunks[0]);
                render_messages(frame, app, content_chunks[1]);
            }
            Panel::TopicTree => {
                render_tree(frame, app, content_chunks[0]);
                render_messages(frame, app, content_chunks[1]);
            }
        }
    } else {
        match app.focused_panel {
            Panel::TopicTree => render_tree(frame, app, main_chunks[1]),
            Panel::Messages => render_messages(frame, app, main_chunks[1]),
            Panel::Stats => render_stats(frame, app, main_chunks[1]),
        }
    }

    render_footer(frame, app, main_chunks[2]);

    if app.input_mode == InputMode::Search {
        render_search(frame, app);
    }

    if app.input_mode == InputMode::MetricSelect {
        render_metric_select(frame, app);
    }

    if app.input_mode == InputMode::Filter {
        render_filter(frame, app);
    }

    if app.input_mode == InputMode::ServerManager {
        render_server_manager(frame, app);
    }

    if app.input_mode == InputMode::Publish {
        render_publish(frame, app);
    }

    if app.input_mode == InputMode::BookmarkManager {
        render_bookmark_manager(frame, app);
    }

    if app.show_help {
        render_help(frame);
    }

    if app.show_david_easter_egg {
        david::render_david_easter_egg(frame);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let status = app.connection_status();
    let color = app.connection_color();

    // Connection status with animated indicator
    let conn_indicator = match app.connection_state {
        crate::mqtt::ConnectionState::Connected => "●",
        crate::mqtt::ConnectionState::Connecting | crate::mqtt::ConnectionState::Reconnecting => {
            "◌"
        }
        crate::mqtt::ConnectionState::Disconnected => "○",
    };

    let rate = app.stats.messages_per_second();
    let rate_color = if rate >= 100.0 {
        Color::Green
    } else if rate > 0.0 {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let mut header_parts = vec![
        Span::styled(
            " mqtop ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{} {}", conn_indicator, status),
            Style::default().fg(color),
        ),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", app.topic_tree.topic_count()),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" topics", Style::default().fg(Color::DarkGray)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format_rate(rate),
            Style::default().fg(rate_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" msg/s", Style::default().fg(Color::DarkGray)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", app.stats.total_messages()),
            Style::default().fg(Color::White),
        ),
        Span::styled(" total", Style::default().fg(Color::DarkGray)),
    ];

    if let Some(server) = app.active_server() {
        header_parts.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        header_parts.push(Span::styled(
            server.name.clone(),
            Style::default().fg(Color::Yellow),
        ));
    }

    // Active filter indicator
    if let Some(ref filter) = app.topic_filter {
        header_parts.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        header_parts.push(Span::styled(
            format!(" {} ", filter),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Starred filter indicator
    if app.filter_mode == crate::app::FilterMode::Starred {
        header_parts.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        header_parts.push(Span::styled(
            " ★ ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let header = Line::from(header_parts);
    frame.render_widget(Paragraph::new(header), area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mode_hints: Vec<Span<'static>> = match app.input_mode {
        InputMode::Normal => {
            let mut hints = Vec::new();
            hints.extend(key_hint("?", "Help"));
            hints.extend(key_hint("/", "Search"));
            hints.extend(key_hint("f", "Filter"));
            hints.extend(key_hint("S", "Servers"));
            hints.extend(key_hint("P", "Publish"));
            hints.extend(key_hint("B", "Bookmarks"));
            hints.extend(key_hint("s", "Star"));
            hints.extend(key_hint("y", "Copy"));
            hints.extend(key_hint("m", "Track"));
            hints.extend(key_hint("q", "Quit"));
            hints
        }
        InputMode::Search => {
            let mut hints = Vec::new();
            hints.extend(key_hint("Enter", "Select"));
            hints.extend(key_hint("↑↓", "Navigate"));
            hints.extend(key_hint("Esc", "Cancel"));
            hints
        }
        InputMode::MetricSelect => {
            let mut hints = Vec::new();
            hints.extend(key_hint("Enter", "Track"));
            hints.extend(key_hint("↑↓", "Navigate"));
            hints.extend(key_hint("Esc", "Cancel"));
            hints
        }
        InputMode::Filter => {
            let mut hints = Vec::new();
            hints.extend(key_hint("Enter", "Apply"));
            hints.extend(key_hint("Esc", "Cancel"));
            hints
        }
        InputMode::ServerManager => {
            let mut hints = Vec::new();
            hints.extend(key_hint("Enter", "Connect"));
            hints.extend(key_hint("e", "Edit"));
            hints.extend(key_hint("a", "Add"));
            hints.extend(key_hint("d", "Delete"));
            hints.extend(key_hint("Esc", "Close"));
            hints
        }
        InputMode::Publish => {
            let mut hints = Vec::new();
            hints.extend(key_hint("Enter", "Publish"));
            hints.extend(key_hint("Tab", "Next"));
            hints.extend(key_hint("^S", "Bookmark"));
            hints.extend(key_hint("Esc", "Cancel"));
            hints
        }
        InputMode::BookmarkManager => {
            let mut hints = Vec::new();
            hints.extend(key_hint("Enter", "Publish"));
            hints.extend(key_hint("e", "Edit"));
            hints.extend(key_hint("a", "Add"));
            hints.extend(key_hint("d", "Delete"));
            hints.extend(key_hint("Esc", "Close"));
            hints
        }
    };

    // Check for status message first
    if let Some(status) = app.get_status() {
        let mut parts = vec![
            Span::styled(
                format!(" {} ", status),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ];
        parts.extend(mode_hints);
        frame.render_widget(Paragraph::new(Line::from(parts)), area);
        return;
    }

    let footer = if let Some(ref err) = app.last_error {
        let mut parts = vec![
            Span::styled(
                " ERROR ",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", truncate_str(err, 40)),
                Style::default().fg(Color::Red),
            ),
        ];
        parts.extend(mode_hints);
        Line::from(parts)
    } else {
        let mut parts = vec![Span::raw(" ")];
        parts.extend(mode_hints);
        Line::from(parts)
    };

    frame.render_widget(Paragraph::new(footer), area);
}

/// Helper to create a bordered block with optional focus highlight
pub fn bordered_block(title: &str, focused: bool) -> Block<'_> {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(Span::styled(
            format!(" {} ", title),
            if focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        ))
}

fn format_rate(rate: f64) -> String {
    if rate >= 1000.0 {
        format!("{:.1}k", rate / 1000.0)
    } else if rate >= 1.0 {
        format!("{:.1}", rate)
    } else if rate > 0.0 {
        format!("{:.2}", rate)
    } else {
        "0".to_string()
    }
}

fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}
