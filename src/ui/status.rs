use ratatui::{
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
    layout::Rect,
};

use crate::app::{AlertLevel, App, Panel};
use crate::ui::{compact_kv, panel_block, section_label};

/// Renders the Status left panel — full-height panel showing version header,
/// system info, resource counts, alerts, and the navigation legend.
pub fn draw_status_panel(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let block = panel_block("Status", is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // ── Version header ───────────────────────────────────────────────────
    lines.push(Line::from(vec![
        Span::styled(
            " dockctl ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            concat!("v", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    lines.push(Line::default());

    // ── System info section ──────────────────────────────────────────────
    if let Some(info) = &app.system_info {
        lines.push(section_label("System"));
        lines.push(compact_kv(
            "RT",
            &format!("{} {}", info.runtime_type, info.server_version),
        ));
        lines.push(compact_kv("OS", &format!("{}/{}", info.os, info.arch)));
        lines.push(Line::default());
    }

    // ── Resource counts section ──────────────────────────────────────────
    let (running, stopped) = match &app.system_info {
        Some(info) => (info.containers_running, info.containers_stopped),
        None => {
            let running = app
                .containers
                .iter()
                .filter(|c| c.state == "running")
                .count() as i64;
            let stopped = app.containers.len() as i64 - running;
            (running, stopped)
        }
    };

    let image_count = match &app.system_info {
        Some(info) => info.images,
        None => app.images.len() as i64,
    };

    lines.push(section_label("Resources"));
    lines.push(Line::from(vec![
        Span::styled(" C ", Style::default().fg(Color::DarkGray).bold()),
        Span::styled(
            format!("{}", running),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" up ", Style::default().fg(Color::Green)),
        Span::styled(
            format!("{}", stopped),
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" down", Style::default().fg(Color::Red)),
    ]));
    lines.push(compact_kv("I", &format!("{}", image_count)));
    lines.push(compact_kv("V", &format!("{}", app.volumes.len())));
    lines.push(compact_kv("N", &format!("{}", app.networks.len())));
    lines.push(Line::default());

    // ── Alerts section ───────────────────────────────────────────────────
    if !app.alerts.is_empty() {
        lines.push(section_label("Alerts"));
        for alert in &app.alerts {
            let (indicator, color) = match alert.level {
                AlertLevel::Critical => ("!!", Color::Red),
                AlertLevel::Warning => ("! ", Color::Yellow),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", indicator),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(alert.message.clone(), Style::default().fg(color)),
            ]));
        }
        lines.push(Line::default());
    }

    // ── Navigation legend ────────────────────────────────────────────────
    lines.push(section_label("Panels"));
    let max_shortcut = if app.supports_pods { "6" } else { "5" };
    for &panel in Panel::ALL {
        // Skip Pods if the runtime doesn't support them
        if panel == Panel::Pods && !app.supports_pods {
            continue;
        }

        let is_active = panel == app.active_panel;
        let shortcut = panel.shortcut();

        let badge_style = if is_active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        };

        let label_style = if is_active {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let indicator = if is_active {
            Span::styled(">>", Style::default().fg(Color::Cyan).bold())
        } else {
            Span::raw("  ")
        };

        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", shortcut), badge_style),
            Span::raw(" "),
            Span::styled(panel.label().to_string(), label_style),
            Span::raw(" "),
            indicator,
        ]));
    }

    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        format!(" 1-{} switch panels", max_shortcut),
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
