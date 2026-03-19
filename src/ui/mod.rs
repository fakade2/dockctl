pub mod builds;
pub mod compose;
pub mod containers;
pub mod dashboard;
pub mod images;
pub mod logs;
pub mod networks;
pub mod pods;
pub mod stats;
pub mod volumes;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{AlertLevel, App, View};
use crate::config::LayoutConfig;
use crate::runtime::PortMapping;

// ─── Public entry point ──────────────────────────────────────────────────────

/// Root render function — draws the sidebar and delegates the main area to the
/// active view.
pub fn draw(frame: &mut Frame, app: &App, layout: &LayoutConfig) {
    let (sidebar_constraint, main_constraint) = (
        Constraint::Length(layout.sidebar_width),
        Constraint::Min(0),
    );

    let constraints = if layout.sidebar_position == "right" {
        vec![main_constraint, sidebar_constraint]
    } else {
        vec![sidebar_constraint, main_constraint]
    };

    let outer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(frame.area());

    let (sidebar_area, main_area) = if layout.sidebar_position == "right" {
        (outer[1], outer[0])
    } else {
        (outer[0], outer[1])
    };

    draw_sidebar(frame, sidebar_area, app);

    match app.current_view {
        View::Dashboard => dashboard::draw_dashboard(frame, main_area, app, layout),
        View::Containers => containers::draw_containers(frame, main_area, app, layout),
        View::Images => images::draw_images(frame, main_area, app, layout),
        View::Volumes => volumes::draw_volumes(frame, main_area, app, layout),
        View::Networks => networks::draw_networks(frame, main_area, app, layout),
        View::Compose => compose::draw_compose(frame, main_area, app, layout),
        View::Builds => builds::draw_builds(frame, main_area, app, layout),
        View::Pods => pods::draw_pods(frame, main_area, app, layout),
    }
}

// ─── Sidebar ─────────────────────────────────────────────────────────────────

/// Renders the left-hand navigation sidebar with system info, resource counts,
/// alerts, and view navigation.
fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(
            Line::from(vec![
                Span::styled(
                    " dockctl ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    concat!("v", env!("CARGO_PKG_VERSION"), " "),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

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

    // ── Navigation section ───────────────────────────────────────────────
    lines.push(section_label("Views"));
    for &view in View::ALL {
        // Skip Pods in the sidebar if the runtime doesn't support them
        if view == View::Pods && !app.supports_pods {
            continue;
        }

        let is_active = view == app.current_view;
        let shortcut = view.shortcut();

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
            Span::styled(view.label().to_string(), label_style),
            Span::raw(" "),
            indicator,
        ]));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// A small section label rendered in the sidebar.
fn section_label(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" [{}]", label),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Compact key-value line for the sidebar.
fn compact_kv(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

/// Creates a standard bordered block with a styled title — used across views.
pub fn titled_block(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {} ", title))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
}

/// Renders a centered placeholder message for views not yet implemented.
pub fn draw_placeholder(frame: &mut Frame, area: Rect, message: &str) {
    let block = titled_block(message);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Vertically center
    let v_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(3),
            Constraint::Percentage(45),
        ])
        .split(inner);

    let text = Paragraph::new(Line::from(vec![Span::styled(
        message,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::ITALIC),
    )]))
    .centered();

    frame.render_widget(text, v_layout[1]);
}

/// Formats a byte count into a human-readable string (e.g. "1.23 GB").
pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.2} GB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MB", b / MIB)
    } else if b >= KIB {
        format!("{:.0} KB", b / KIB)
    } else {
        format!("{} B", bytes)
    }
}

/// Formats a Unix timestamp (seconds since epoch) into a relative age string.
pub fn format_timestamp_age(ts: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = now - ts;
    if diff < 0 {
        return "in the future".to_string();
    }
    let days = diff / 86400;
    let hours = diff / 3600;
    let minutes = diff / 60;
    if days > 30 {
        // Format as absolute date
        chrono::DateTime::from_timestamp(ts, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".to_string())
    } else if days > 0 {
        format!("{} days ago", days)
    } else if hours > 0 {
        format!("{} hours ago", hours)
    } else if minutes > 0 {
        format!("{} min ago", minutes)
    } else {
        "just now".to_string()
    }
}

/// Returns the first (primary) display name from a container's names list.
/// Docker prepends `/` to container names, which is stripped here.
pub fn container_display_name(names: &[String]) -> String {
    names
        .first()
        .map(|n| n.strip_prefix('/').unwrap_or(n).to_string())
        .unwrap_or_else(|| "<unnamed>".to_string())
}

/// Formats a `PortMapping` into a human-readable string like "0.0.0.0:8080->80/tcp".
pub fn format_port(pm: &PortMapping) -> String {
    match (&pm.ip, pm.public_port) {
        (Some(ip), Some(pub_port)) => {
            format!("{}:{}->{}/{}", ip, pub_port, pm.private_port, pm.port_type)
        }
        (None, Some(pub_port)) => {
            format!("{}->{}/{}", pub_port, pm.private_port, pm.port_type)
        }
        _ => format!("{}/{}", pm.private_port, pm.port_type),
    }
}

/// Formats a list of port mappings into a single comma-separated string.
pub fn format_ports(ports: &[PortMapping]) -> String {
    if ports.is_empty() {
        return String::new();
    }
    ports
        .iter()
        .map(format_port)
        .collect::<Vec<_>>()
        .join(", ")
}
