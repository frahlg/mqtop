use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::widgets::centered_rect;

pub fn render_help(frame: &mut Frame) {
    let area = centered_rect(70, 85, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" mqtop Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let help_text = vec![
        section("Navigation"),
        keybind("↑↓ j/k", "Move up/down"),
        keybind("←→ h/l", "Collapse/Expand or move to parent/child"),
        keybind("H / L", "Collapse/Expand full branch"),
        keybind("Enter", "Toggle expand/collapse"),
        keybind("Tab", "Switch panel (Topics → Messages → Stats)"),
        keybind("1 / 2 / 3", "Jump to panel directly"),
        keybind("PgUp/PgDn", "Page up/down"),
        keybind("g / G", "Go to top/bottom"),
        Line::from(""),
        section("Search & Filter"),
        keybind("/", "Open fuzzy search"),
        keybind("f", "Set topic filter (MQTT wildcards: + #)"),
        keybind("s", "Star/unstar current topic"),
        keybind("*", "Toggle starred topics filter"),
        Line::from(""),
        section("Servers & Publishing"),
        keybind("S", "Manage MQTT servers"),
        keybind("P", "Open publish dialog"),
        keybind("Ctrl+P", "Copy current message to publish"),
        keybind("B", "Open bookmark manager"),
        keybind("Ctrl+S", "Save publish as bookmark"),
        Line::from(""),
        section("Data & Display"),
        keybind("m", "Track metric from current message"),
        keybind("p", "Cycle payload mode (Auto → Raw → Hex → JSON)"),
        keybind("y", "Copy topic to clipboard"),
        keybind("Y", "Copy payload to clipboard"),
        keybind("c", "Clear statistics"),
        Line::from(""),
        section("General"),
        keybind("?", "Toggle this help"),
        keybind("q / Ctrl+C", "Quit"),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Tip: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Topic colors are configurable via ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("[[ui.topic_colors]]", Style::default().fg(Color::Yellow)),
            Span::styled(" in config.toml", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "                                    [Esc to close]",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(help_text).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn section(title: &str) -> Line<'static> {
    Line::from(vec![Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )])
}

fn keybind(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:14}", key), Style::default().fg(Color::Yellow)),
        Span::raw(desc.to_string()),
    ])
}
