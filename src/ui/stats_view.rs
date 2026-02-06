use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::bordered_block;
use crate::app::{App, Panel};
use crate::state::{render_sparkline, HealthStatus, LatencyTracker, Stats};

pub fn render_stats(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == Panel::Stats;
    let block = bordered_block("Stats", focused);
    let inner = block.inner(area);

    frame.render_widget(block, area);

    let mut lines = Vec::new();

    // Connection info
    lines.push(stats_section("Connection"));
    lines.push(Line::from(vec![
        Span::styled("  Status  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            app.connection_status(),
            Style::default().fg(app.connection_color()),
        ),
    ]));
    if let Some(server) = app.active_server() {
        lines.push(Line::from(vec![
            Span::styled("  Host    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}:{}", server.host, server.port),
                Style::default().fg(Color::Cyan),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Server  ", Style::default().fg(Color::DarkGray)),
            Span::styled(server.name.clone(), Style::default().fg(Color::Yellow)),
        ]));
    }
    lines.push(Line::from(""));

    // Message stats
    lines.push(stats_section("Messages"));
    lines.push(Line::from(vec![
        Span::styled("  Total   ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format_number(app.stats.total_messages()),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Rate    ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            Stats::format_rate(app.stats.messages_per_second()),
            Style::default().fg(Color::Green),
        ),
    ]));
    lines.push(Line::from(""));

    // Tracked Metrics section - placed high so it's always visible
    let metrics = app.metric_tracker.get_metrics();
    if !metrics.is_empty() {
        lines.push(stats_section_colored("Tracked Metrics", Color::Magenta));

        for metric in metrics {
            // Metric label and current value
            let current = metric
                .latest()
                .map(format_metric_value)
                .unwrap_or_else(|| "---".to_string());
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}: ", metric.label),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    current,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            // Sparkline
            let sparkline_width = 20;
            let sparkline_data = metric.sparkline_data(sparkline_width);
            let sparkline_str = render_sparkline(&sparkline_data, sparkline_width);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(sparkline_str, Style::default().fg(Color::Magenta)),
            ]));

            // Min/Max/Avg stats on one line
            if metric.count > 0 {
                lines.push(Line::from(vec![
                    Span::styled("  min:", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format_metric_value(metric.min),
                        Style::default().fg(Color::Blue),
                    ),
                    Span::styled(" max:", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format_metric_value(metric.max),
                        Style::default().fg(Color::Red),
                    ),
                    Span::styled(" avg:", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format_metric_value(metric.avg()),
                        Style::default().fg(Color::Yellow),
                    ),
                ]));
            }
        }
        lines.push(Line::from(""));
    }

    // Data stats
    lines.push(stats_section("Data"));
    lines.push(Line::from(vec![
        Span::styled("  Total   ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            Stats::format_bytes(app.stats.total_bytes()),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Rate    ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(
                "{}/s",
                Stats::format_bytes(app.stats.bytes_per_second() as u64)
            ),
            Style::default().fg(Color::Green),
        ),
    ]));
    lines.push(Line::from(""));

    // Topic stats
    lines.push(stats_section("Topics"));
    lines.push(Line::from(vec![
        Span::styled("  Unique  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format_number(app.topic_tree.topic_count() as u64),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Buffered", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(
                " {}",
                format_number(app.message_buffer.total_stored() as u64)
            ),
            Style::default().fg(Color::Yellow),
        ),
    ]));
    lines.push(Line::from(""));

    // Session info
    lines.push(stats_section("Session"));
    lines.push(Line::from(vec![
        Span::styled("  Uptime  ", Style::default().fg(Color::DarkGray)),
        Span::styled(app.stats.uptime_string(), Style::default().fg(Color::White)),
    ]));
    if let Some(server) = app.active_server() {
        lines.push(Line::from(vec![
            Span::styled("  Client  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                server.client_id.clone(),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    // Latency info
    if app.latency_tracker.inter_arrival_count > 0 {
        lines.push(Line::from(""));
        lines.push(stats_section("Latency"));

        // Inter-arrival time (time between messages)
        if let Some(avg) = app.latency_tracker.avg_inter_arrival() {
            lines.push(Line::from(vec![
                Span::raw("  Interval: "),
                Span::styled(
                    LatencyTracker::format_duration(avg),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(" avg", Style::default().fg(Color::DarkGray)),
            ]));
        }

        // Payload latency (if timestamps available)
        if let Some(avg) = app.latency_tracker.avg_payload_latency() {
            let color = if app.latency_tracker.has_high_latency() {
                Color::Red
            } else if avg.as_millis() > 1000 {
                Color::Yellow
            } else {
                Color::Green
            };

            lines.push(Line::from(vec![
                Span::raw("  Msg Delay: "),
                Span::styled(
                    LatencyTracker::format_duration(avg),
                    Style::default().fg(color),
                ),
                Span::styled(" avg", Style::default().fg(Color::DarkGray)),
            ]));

            if let Some(max) = app.latency_tracker.max_payload_latency {
                lines.push(Line::from(vec![
                    Span::styled("  max: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        LatencyTracker::format_duration(max),
                        Style::default().fg(if max.as_secs() > 5 {
                            Color::Red
                        } else {
                            Color::White
                        }),
                    ),
                ]));
            }
        }

        // Jitter
        if let Some(jitter) = app.latency_tracker.jitter() {
            lines.push(Line::from(vec![
                Span::raw("  Jitter: "),
                Span::styled(
                    LatencyTracker::format_duration(jitter),
                    Style::default().fg(if jitter.as_millis() > 500 {
                        Color::Yellow
                    } else {
                        Color::White
                    }),
                ),
            ]));
        }
    }

    // Topic Categories (configurable)
    let categories = &app.config.ui.topic_categories;
    if !categories.is_empty() {
        lines.push(Line::from(""));
        lines.push(stats_section_colored("Categories", Color::Cyan));

        let visible = app.get_visible_topics();
        for category in categories {
            let count = visible
                .iter()
                .filter(|t| category.matches(&t.full_path))
                .count();
            if count > 0 {
                lines.push(Line::from(vec![
                    Span::raw(format!("  {}: ", category.label)),
                    Span::styled(count.to_string(), Style::default().fg(category.to_color())),
                ]));
            }
        }
    }

    // Device Health section
    let device_count = app.device_tracker.device_count();
    if device_count > 0 {
        lines.push(Line::from(""));
        lines.push(stats_section_colored("Device Health", Color::Green));

        let (healthy, warning, stale, unknown) = app.device_tracker.count_by_status();
        lines.push(Line::from(vec![
            Span::styled("  ● ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("{} healthy", healthy),
                Style::default().fg(Color::White),
            ),
            Span::raw("  "),
            Span::styled("● ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{} warn", warning),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ● ", Style::default().fg(Color::Red)),
            Span::styled(
                format!("{} stale", stale),
                Style::default().fg(Color::White),
            ),
            Span::raw("  "),
            Span::styled("● ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} new", unknown),
                Style::default().fg(Color::White),
            ),
        ]));

        // Show top 3 most recent devices
        let devices = app.device_tracker.get_devices();
        if !devices.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  Recent:",
                Style::default().fg(Color::DarkGray),
            )]));

            for device in devices.iter().take(3) {
                let status_color = match device.status {
                    HealthStatus::Healthy => Color::Green,
                    HealthStatus::Warning => Color::Yellow,
                    HealthStatus::Stale => Color::Red,
                    HealthStatus::Unknown => Color::DarkGray,
                };
                let status_char = match device.status {
                    HealthStatus::Healthy => "●",
                    HealthStatus::Warning => "●",
                    HealthStatus::Stale => "○",
                    HealthStatus::Unknown => "◌",
                };

                // Truncate device ID for display
                let display_id = if device.device_id.len() > 12 {
                    format!("{}...", &device.device_id[..12])
                } else {
                    device.device_id.clone()
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", status_char),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(display_id, Style::default().fg(Color::White)),
                ]));
                lines.push(Line::from(vec![Span::styled(
                    format!(
                        "    {} | {}",
                        device.last_seen_string(),
                        device.message_count
                    ),
                    Style::default().fg(Color::DarkGray),
                )]));
            }

            if devices.len() > 3 {
                lines.push(Line::from(vec![Span::styled(
                    format!("  ... +{} more", devices.len() - 3),
                    Style::default().fg(Color::DarkGray),
                )]));
            }
        }
    }

    // Add scroll indicator if content exceeds panel height
    let total_lines = lines.len();
    let visible_height = inner.height as usize;

    let paragraph = if total_lines > visible_height {
        // Apply scroll offset from app state
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((app.stats_scroll as u16, 0))
    } else {
        Paragraph::new(lines).wrap(Wrap { trim: false })
    };

    frame.render_widget(paragraph, inner);
}

fn stats_section(title: &str) -> Line<'static> {
    Line::from(vec![Span::styled(
        format!("▸ {}", title),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )])
}

fn stats_section_colored(title: &str, color: Color) -> Line<'static> {
    Line::from(vec![Span::styled(
        format!("▸ {}", title),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )])
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

fn format_metric_value(v: f64) -> String {
    if v.abs() >= 1_000_000.0 {
        format!("{:.2}M", v / 1_000_000.0)
    } else if v.abs() >= 1_000.0 {
        format!("{:.1}k", v / 1_000.0)
    } else if v.fract() == 0.0 {
        format!("{:.0}", v)
    } else {
        format!("{:.2}", v)
    }
}
