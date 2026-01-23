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

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, InputMode, Panel};

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

    let mut header_parts = vec![
        Span::styled(
            " mqtop ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("│ "),
        Span::styled(format!("● {}", status), Style::default().fg(color)),
        Span::raw(" │ "),
        Span::raw(format!("Topics: {} ", app.topic_tree.topic_count())),
        Span::raw("│ "),
        Span::raw(format!(
            "Msgs: {} ({}/s)",
            app.stats.total_messages(),
            format_rate(app.stats.messages_per_second())
        )),
        Span::raw(" │ "),
        Span::raw(format!("Uptime: {}", app.stats.uptime_string())),
    ];

    if let Some(server) = app.active_server() {
        header_parts.push(Span::raw(" │ "));
        header_parts.push(Span::styled(
            format!("Server: {}", server.name),
            Style::default().fg(Color::Yellow),
        ));
    }

    let header = Line::from(header_parts);

    frame.render_widget(Paragraph::new(header), area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mode_hint = match app.input_mode {
        InputMode::Normal => {
            "q:Quit /:Search f:Filter S:Servers P:Publish B:Bookmarks s:Star y:Copy m:Track ?:Help"
        }
        InputMode::Search => "Enter:Select  Esc:Cancel  ↑↓:Navigate results",
        InputMode::MetricSelect => "Enter:Track  Esc:Cancel  ↑↓/jk:Navigate",
        InputMode::Filter => "Enter:Apply  Esc:Cancel  (empty to clear)",
        InputMode::ServerManager => "Enter:Activate  e:Edit  a:Add  d:Delete  Esc:Close",
        InputMode::Publish => "Enter:Publish  Tab:Next field  Ctrl+S:Save Bookmark  Esc:Cancel",
        InputMode::BookmarkManager => "Enter:Publish  e:Edit  a:Add  d:Delete  Esc:Close",
    };

    // Check for status message first
    if let Some(status) = app.get_status() {
        let footer = Line::from(vec![
            Span::styled(
                format!(" {} ", status),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("│ "),
            Span::styled(mode_hint, Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(footer), area);
        return;
    }

    let footer = if let Some(ref err) = app.last_error {
        Line::from(vec![
            Span::styled(
                " ERROR: ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(truncate_str(err, 50), Style::default().fg(Color::Red)),
            Span::raw(" │ "),
            Span::raw(mode_hint),
        ])
    } else {
        Line::from(Span::styled(
            format!(" {}", mode_hint),
            Style::default().fg(Color::DarkGray),
        ))
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
