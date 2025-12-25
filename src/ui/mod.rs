mod tree_view;
mod message_view;
mod stats_view;
mod search;
mod help;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, InputMode};

pub use tree_view::render_tree;
pub use message_view::render_messages;
pub use stats_view::render_stats;
pub use search::render_search;
pub use help::render_help;

/// Main render function
pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Create main layout: header, content, footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Header
            Constraint::Min(10),    // Content
            Constraint::Length(1),  // Footer
        ])
        .split(size);

    // Render header
    render_header(frame, app, main_chunks[0]);

    // Create content layout: tree | messages | stats
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),  // Topic tree
            Constraint::Percentage(45),  // Messages
            Constraint::Percentage(25),  // Stats
        ])
        .split(main_chunks[1]);

    // Render panels
    render_tree(frame, app, content_chunks[0]);
    render_messages(frame, app, content_chunks[1]);
    render_stats(frame, app, content_chunks[2]);

    // Render footer
    render_footer(frame, app, main_chunks[2]);

    // Render overlays
    if app.input_mode == InputMode::Search {
        render_search(frame, app);
    }

    if app.show_help {
        render_help(frame);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let status = app.connection_status();
    let color = app.connection_color();

    let header = Line::from(vec![
        Span::styled(" Sourceful DataFeeder ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
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
    ]);

    frame.render_widget(Paragraph::new(header), area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mode_hint = match app.input_mode {
        InputMode::Normal => {
            "q:Quit  /:Search  Tab:Panel  hjkl/↑↓←→:Navigate  Enter:Select  ?:Help  p:Payload mode"
        }
        InputMode::Search => {
            "Enter:Select  Esc:Cancel  ↑↓:Navigate results"
        }
    };

    let footer = if let Some(ref err) = app.last_error {
        Line::from(vec![
            Span::styled(" ERROR: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(truncate_str(err, 60), Style::default().fg(Color::Red)),
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
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
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
