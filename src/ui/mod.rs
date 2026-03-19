pub mod containers;
pub mod images;
pub mod logs;
pub mod networks;
pub mod pods;
pub mod preview;
pub mod stats;
pub mod status;
pub mod volumes;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, FocusZone, Panel};
use crate::config::LayoutConfig;
use crate::runtime::PortMapping;

// ─── Public entry point ──────────────────────────────────────────────────────

/// Root render function — splits the screen into two columns (left panel + right
/// preview) in the lazygit style and delegates rendering to the appropriate
/// panel and preview modules.
pub fn draw(frame: &mut Frame, app: &App, layout: &LayoutConfig) {
    let outer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(layout.left_panel_percent),
            Constraint::Percentage(100 - layout.left_panel_percent),
        ])
        .split(frame.area());

    let left_area = outer[0];
    let right_area = outer[1];

    let left_focused = app.focus == FocusZone::LeftPanel;
    let right_focused = app.focus == FocusZone::RightPreview;

    // Route left panel by active_panel
    match app.active_panel {
        Panel::Status => status::draw_status_panel(frame, left_area, app, left_focused),
        Panel::Containers => containers::draw_container_list(frame, left_area, app, left_focused),
        Panel::Images => images::draw_image_list(frame, left_area, app, left_focused),
        Panel::Volumes => volumes::draw_volume_list(frame, left_area, app, left_focused),
        Panel::Networks => networks::draw_network_list(frame, left_area, app, left_focused),
        Panel::Pods => pods::draw_pod_list(frame, left_area, app, left_focused),
    }

    // Route right preview by active_panel
    match app.active_panel {
        Panel::Status => preview::draw_status_preview(frame, right_area, app, right_focused),
        Panel::Containers => {
            preview::draw_container_preview(frame, right_area, app, right_focused, layout)
        }
        Panel::Images => preview::draw_image_preview(frame, right_area, app, right_focused),
        Panel::Volumes => preview::draw_volume_preview(frame, right_area, app, right_focused),
        Panel::Networks => preview::draw_network_preview(frame, right_area, app, right_focused),
        Panel::Pods => preview::draw_pod_preview(frame, right_area, app, right_focused),
    }
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

/// Creates a bordered block with a styled title. The border is cyan when
/// the panel is focused, dark gray otherwise.
pub(crate) fn panel_block(title: &str, is_focused: bool) -> Block<'_> {
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    Block::default()
        .title(format!(" {} ", title))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
}

/// Simple titled block (unfocused variant of `panel_block`).
pub(crate) fn titled_block(title: &str) -> Block<'_> {
    panel_block(title, false)
}

/// A small section label — renders like " [System]" in bold cyan.
pub(crate) fn section_label(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" [{}]", label),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Compact key-value line for sidebar / status panels.
pub(crate) fn compact_kv(key: &str, value: &str) -> Line<'static> {
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

/// Formats a byte count into a human-readable string (e.g. "1.23 GB").
pub(crate) fn format_bytes(bytes: u64) -> String {
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
pub(crate) fn format_timestamp_age(ts: i64) -> String {
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
pub(crate) fn container_display_name(names: &[String]) -> String {
    names
        .first()
        .map(|n| n.strip_prefix('/').unwrap_or(n).to_string())
        .unwrap_or_else(|| "<unnamed>".to_string())
}

/// Formats a `PortMapping` into a human-readable string like "0.0.0.0:8080->80/tcp".
pub(crate) fn format_port(pm: &PortMapping) -> String {
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
pub(crate) fn format_ports(ports: &[PortMapping]) -> String {
    if ports.is_empty() {
        return String::new();
    }
    ports
        .iter()
        .map(format_port)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Renders a centered placeholder message for views with no content.
pub(crate) fn draw_placeholder(frame: &mut Frame, area: Rect, message: &str) {
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
        message.to_string(),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::ITALIC),
    )]))
    .centered();

    frame.render_widget(text, v_layout[1]);
}
