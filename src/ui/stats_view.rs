use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Panel};
use crate::state::Stats;
use super::bordered_block;

pub fn render_stats(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == Panel::Stats;
    let block = bordered_block("Stats", focused);
    let inner = block.inner(area);

    frame.render_widget(block, area);

    let mut lines = Vec::new();

    // Connection info
    lines.push(Line::from(vec![
        Span::styled("Connection", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Status: "),
        Span::styled(
            app.connection_status(),
            Style::default().fg(app.connection_color()),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Host: "),
        Span::styled(
            format!("{}:{}", app.config.mqtt.host, app.config.mqtt.port),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    lines.push(Line::from(""));

    // Message stats
    lines.push(Line::from(vec![
        Span::styled("Messages", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Total: "),
        Span::styled(
            format_number(app.stats.total_messages()),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Rate: "),
        Span::styled(
            Stats::format_rate(app.stats.messages_per_second()),
            Style::default().fg(Color::Green),
        ),
    ]));
    lines.push(Line::from(""));

    // Data stats
    lines.push(Line::from(vec![
        Span::styled("Data", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Total: "),
        Span::styled(
            Stats::format_bytes(app.stats.total_bytes()),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Rate: "),
        Span::styled(
            format!("{}/s", Stats::format_bytes(app.stats.bytes_per_second() as u64)),
            Style::default().fg(Color::Green),
        ),
    ]));
    lines.push(Line::from(""));

    // Topic stats
    lines.push(Line::from(vec![
        Span::styled("Topics", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Unique: "),
        Span::styled(
            format_number(app.topic_tree.topic_count() as u64),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Buffered: "),
        Span::styled(
            format_number(app.message_buffer.total_stored() as u64),
            Style::default().fg(Color::Yellow),
        ),
    ]));
    lines.push(Line::from(""));

    // Session info
    lines.push(Line::from(vec![
        Span::styled("Session", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Uptime: "),
        Span::styled(
            app.stats.uptime_string(),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Client: "),
        Span::styled(
            &app.config.mqtt.client_id,
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Sourceful entity counts (if we tracked them)
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Sourceful", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
    ]));

    // Count topics by prefix
    let visible = app.get_visible_topics();
    let wallets = visible.iter().filter(|t| t.full_path.starts_with("wallets")).count();
    let sites = visible.iter().filter(|t| t.full_path.contains("sites") || t.full_path.starts_with("sites")).count();
    let devices = visible.iter().filter(|t| t.full_path.contains("devices")).count();
    let telemetry = visible.iter().filter(|t| t.full_path.starts_with("telemetry")).count();
    let ems = visible.iter().filter(|t| t.full_path.starts_with("ems")).count();

    if wallets > 0 {
        lines.push(Line::from(vec![
            Span::raw("  Wallets: "),
            Span::styled(wallets.to_string(), Style::default().fg(Color::LightRed)),
        ]));
    }
    if sites > 0 {
        lines.push(Line::from(vec![
            Span::raw("  Sites: "),
            Span::styled(sites.to_string(), Style::default().fg(Color::Cyan)),
        ]));
    }
    if devices > 0 {
        lines.push(Line::from(vec![
            Span::raw("  Devices: "),
            Span::styled(devices.to_string(), Style::default().fg(Color::Green)),
        ]));
    }
    if telemetry > 0 {
        lines.push(Line::from(vec![
            Span::raw("  Telemetry: "),
            Span::styled(telemetry.to_string(), Style::default().fg(Color::Magenta)),
        ]));
    }
    if ems > 0 {
        lines.push(Line::from(vec![
            Span::raw("  EMS: "),
            Span::styled(ems.to_string(), Style::default().fg(Color::Blue)),
        ]));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
