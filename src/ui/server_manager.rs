use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::widgets::{centered_rect, dialog_key_hint};
use crate::app::{App, ServerField};
use crate::broker::BrokerKind;

pub fn render_server_manager(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 70, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Servers (Tab: MQTT/NATS) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    if app.preset_select.active {
        render_preset_select(frame, app, inner);
        return;
    }
    if app.server_edit.active {
        render_server_edit(frame, app, inner);
        return;
    }
    if app.nats_server_edit.active {
        render_nats_server_edit(frame, app, inner);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(inner);

    let (active_server, protocol) = match app.server_manager_kind {
        BrokerKind::Mqtt => (app.config.mqtt.active_server.clone(), "MQTT"),
        BrokerKind::Nats => (app.config.nats.active_server.clone(), "NATS"),
    };
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Protocol: "),
            Span::styled(protocol, Style::default().fg(Color::Yellow)),
            Span::raw("  "),
            Span::styled("Tab", Style::default().fg(Color::Cyan)),
            Span::raw(" switch"),
        ]),
        Line::from(vec![
            Span::raw("Active: "),
            Span::styled(active_server, Style::default().fg(Color::Yellow)),
        ]),
    ]);
    frame.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = match app.server_manager_kind {
        BrokerKind::Mqtt => app
            .config
            .mqtt
            .servers
            .iter()
            .enumerate()
            .map(|(index, server)| {
                let is_active = server.name == app.config.mqtt.active_server;
                let is_selected = index == app.server_manager_index;
                let mut spans = Vec::new();
                let prefix = if is_selected { "▶ " } else { "  " };
                spans.push(Span::styled(
                    prefix,
                    Style::default().fg(if is_selected {
                        Color::Cyan
                    } else {
                        Color::DarkGray
                    }),
                ));
                if is_active {
                    spans.push(Span::styled("★ ", Style::default().fg(Color::Yellow)));
                } else {
                    spans.push(Span::raw("  "));
                }
                spans.push(Span::styled(
                    server.name.clone(),
                    Style::default().fg(Color::White),
                ));
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    format!("{}:{}", server.host, server.port),
                    Style::default().fg(Color::DarkGray),
                ));
                if server.use_tls {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled("TLS", Style::default().fg(Color::Green)));
                }
                ListItem::new(Line::from(spans))
            })
            .collect(),
        BrokerKind::Nats => app
            .config
            .nats
            .servers
            .iter()
            .enumerate()
            .map(|(index, server)| {
                let is_active = server.name == app.config.nats.active_server;
                let is_selected = index == app.server_manager_index;
                let mut spans = Vec::new();
                let prefix = if is_selected { "▶ " } else { "  " };
                spans.push(Span::styled(
                    prefix,
                    Style::default().fg(if is_selected {
                        Color::Cyan
                    } else {
                        Color::DarkGray
                    }),
                ));
                if is_active {
                    spans.push(Span::styled("★ ", Style::default().fg(Color::Yellow)));
                } else {
                    spans.push(Span::raw("  "));
                }
                spans.push(Span::styled(
                    server.name.clone(),
                    Style::default().fg(Color::White),
                ));
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    format!("{}:{}", server.host, server.port),
                    Style::default().fg(Color::DarkGray),
                ));
                if server.use_tls {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled("TLS", Style::default().fg(Color::Green)));
                }
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    server.subscribe_subject.clone(),
                    Style::default().fg(Color::DarkGray),
                ));
                ListItem::new(Line::from(spans))
            })
            .collect(),
    };

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    let mut hints = Vec::new();
    hints.extend(dialog_key_hint("Enter", "Connect"));
    hints.extend(dialog_key_hint("e", "Edit"));
    hints.extend(dialog_key_hint("a", "Add"));
    hints.extend(dialog_key_hint("d", "Delete"));
    hints.extend(dialog_key_hint("Tab", "Switch"));
    hints.extend(dialog_key_hint("Esc", "Close"));
    frame.render_widget(Paragraph::new(Line::from(hints)), chunks[2]);
}

fn render_server_edit(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled("Editing MQTT server", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" next field  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" save"),
    ]));
    frame.render_widget(header, chunks[0]);

    let fields = app.visible_server_fields();
    let items: Vec<ListItem> = fields
        .iter()
        .map(|field| {
            let is_active = *field == app.server_edit.field;
            let label = field.label();
            let value = app.server_edit_field_value(*field);
            let style = if is_active {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let mut spans = vec![Span::styled(
                format!("{:>12}: ", label),
                Style::default().fg(Color::DarkGray),
            )];
            if is_active && !field.is_checkbox() {
                let cursor = app.server_edit.cursor.min(value.len());
                let (head, tail) = value.split_at(cursor);
                spans.push(Span::styled(head.to_string(), style));
                spans.push(Span::styled(
                    "▌",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::SLOW_BLINK),
                ));
                spans.push(Span::styled(tail.to_string(), style));
                // Show placeholder hint for empty Client ID
                if *field == ServerField::ClientId && value.is_empty() {
                    spans.push(Span::styled(
                        "(auto: mqtop-timestamp)",
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            } else {
                // Show placeholder for empty Client ID when not active
                if *field == ServerField::ClientId && value.is_empty() {
                    spans.push(Span::styled("(auto)", Style::default().fg(Color::DarkGray)));
                } else {
                    spans.push(Span::styled(value, style));
                }
                if is_active && field.is_checkbox() {
                    spans.push(Span::styled(
                        "▌",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::SLOW_BLINK),
                    ));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    let mut hints = Vec::new();
    hints.extend(dialog_key_hint("Enter", "Save"));
    hints.extend(dialog_key_hint("Tab", "Next"));
    hints.extend(dialog_key_hint("Space", "Toggle"));
    hints.extend(dialog_key_hint("Esc", "Cancel"));
    frame.render_widget(Paragraph::new(Line::from(hints)), chunks[2]);
}

fn render_nats_server_edit(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled("Editing NATS server", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" next field  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" save"),
    ]));
    frame.render_widget(header, chunks[0]);

    let fields = app.visible_nats_fields();
    let items: Vec<ListItem> = fields
        .iter()
        .map(|field| {
            let is_active = *field == app.nats_server_edit.field;
            let label = field.label();
            let value = app.nats_server_edit_field_value(*field);
            let style = if is_active {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let mut spans = vec![Span::styled(
                format!("{:>12}: ", label),
                Style::default().fg(Color::DarkGray),
            )];
            if is_active && !field.is_checkbox() {
                let cursor = app.nats_server_edit.cursor.min(value.len());
                let (head, tail) = value.split_at(cursor);
                spans.push(Span::styled(head.to_string(), style));
                spans.push(Span::styled(
                    "▌",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::SLOW_BLINK),
                ));
                spans.push(Span::styled(tail.to_string(), style));
            } else {
                spans.push(Span::styled(value, style));
                if is_active && field.is_checkbox() {
                    spans.push(Span::styled(
                        "▌",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::SLOW_BLINK),
                    ));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    let mut hints = Vec::new();
    hints.extend(dialog_key_hint("Enter", "Save"));
    hints.extend(dialog_key_hint("Tab", "Next"));
    hints.extend(dialog_key_hint("Space", "Toggle"));
    hints.extend(dialog_key_hint("Esc", "Cancel"));
    frame.render_widget(Paragraph::new(Line::from(hints)), chunks[2]);
}

fn render_preset_select(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);

    let kind_label = match app.preset_select.broker_kind {
        BrokerKind::Mqtt => "MQTT",
        BrokerKind::Nats => "NATS",
    };
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!("Select {} server template", kind_label),
        Style::default().fg(Color::Cyan),
    )]));
    frame.render_widget(header, chunks[0]);

    let labels = app.preset_labels();
    let items: Vec<ListItem> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let is_selected = i == app.preset_select.selected;
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}", prefix, label),
                style,
            )))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    let mut hints = Vec::new();
    hints.extend(dialog_key_hint("Enter", "Select"));
    hints.extend(dialog_key_hint("↑↓", "Navigate"));
    hints.extend(dialog_key_hint("Esc", "Cancel"));
    frame.render_widget(Paragraph::new(Line::from(hints)), chunks[2]);
}
