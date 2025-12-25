use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Panel};
use crate::state::{render_sparkline, HealthStatus, LatencyTracker, Stats};
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

    // Tracked Metrics section - placed high so it's always visible
    let metrics = app.metric_tracker.get_metrics();
    if !metrics.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Tracked Metrics", Style::default().add_modifier(Modifier::BOLD).fg(Color::Magenta)),
        ]));

        for metric in metrics {
            // Metric label and current value
            let current = metric.latest().map(|v| format_metric_value(v)).unwrap_or_else(|| "---".to_string());
            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", metric.label), Style::default().fg(Color::White)),
                Span::styled(current, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
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
                    Span::styled(format_metric_value(metric.min), Style::default().fg(Color::Blue)),
                    Span::styled(" max:", Style::default().fg(Color::DarkGray)),
                    Span::styled(format_metric_value(metric.max), Style::default().fg(Color::Red)),
                    Span::styled(" avg:", Style::default().fg(Color::DarkGray)),
                    Span::styled(format_metric_value(metric.avg()), Style::default().fg(Color::Yellow)),
                ]));
            }
        }
        lines.push(Line::from(""));
    }

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

    // Latency info
    if app.latency_tracker.inter_arrival_count > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Latency", Style::default().add_modifier(Modifier::BOLD)),
        ]));

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
                        Style::default().fg(if max.as_secs() > 5 { Color::Red } else { Color::White }),
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
                    Style::default().fg(if jitter.as_millis() > 500 { Color::Yellow } else { Color::White }),
                ),
            ]));
        }
    }

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

    // Device Health section
    let device_count = app.device_tracker.device_count();
    if device_count > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Device Health", Style::default().add_modifier(Modifier::BOLD).fg(Color::Green)),
        ]));

        let (healthy, warning, stale, unknown) = app.device_tracker.count_by_status();
        lines.push(Line::from(vec![
            Span::styled("  ● ", Style::default().fg(Color::Green)),
            Span::styled(format!("{} healthy", healthy), Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled("● ", Style::default().fg(Color::Yellow)),
            Span::styled(format!("{} warn", warning), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ● ", Style::default().fg(Color::Red)),
            Span::styled(format!("{} stale", stale), Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled("● ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} new", unknown), Style::default().fg(Color::White)),
        ]));

        // Show top 3 most recent devices
        let devices = app.device_tracker.get_devices();
        if !devices.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Recent:", Style::default().fg(Color::DarkGray)),
            ]));

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
                    Span::styled(format!("  {} ", status_char), Style::default().fg(status_color)),
                    Span::styled(display_id, Style::default().fg(Color::White)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {} | {}", device.last_seen_string(), device.message_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }

            if devices.len() > 3 {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  ... +{} more", devices.len() - 3),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
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
