pub mod preview;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row},
};

use crate::runtime::PortMapping;

// ─── Shared helpers ──────────────────────────────────────────────────────────

/// Creates a bordered block with a numbered badge in the title.
/// Example: " [4] Containers (9) "
/// Number is in brackets, dimmed. Header text is high contrast (white bold when
/// focused, white when unfocused). Border is cyan when focused, dark gray otherwise.
pub fn panel_block(title: &str, number: u8, is_focused: bool) -> Block<'static> {
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let number_style = Style::default().fg(Color::DarkGray);
    let title_style = if is_focused {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let title_line = Line::from(vec![
        Span::styled(format!(" [{}]", number), number_style),
        Span::styled(format!(" {} ", title), title_style),
    ]);

    Block::default()
        .title(title_line)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
}

/// Renders a single-line collapsed panel header (no borders, just styled text).
/// Used for accordion panels that are not expanded.
pub fn collapsed_panel_line(title: &str, number: u8, is_focused: bool) -> Line<'static> {
    let number_style = Style::default().fg(Color::DarkGray);
    let title_style = if is_focused {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let sep_style = Style::default().fg(Color::DarkGray);

    Line::from(vec![
        Span::styled("─", sep_style),
        Span::styled(format!(" [{}]", number), number_style),
        Span::styled(format!(" {} ", title), title_style),
        Span::styled("─".repeat(40), sep_style),
    ])
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

/// Formats a `PortMapping` as "source:destination" (e.g. "5432:5432").
/// Strips protocol, `0.0.0.0`, and `::` — just shows the port numbers.
pub fn format_port(pm: &PortMapping) -> String {
    match pm.public_port {
        Some(pub_port) => format!("{}:{}", pub_port, pm.private_port),
        None => format!("{}", pm.private_port),
    }
}

/// Formats a list of port mappings into a deduplicated, sorted, comma-separated string.
/// Docker/bollard returns duplicate entries (IPv4 + IPv6, per protocol), so we deduplicate.
pub fn format_ports(ports: &[PortMapping]) -> String {
    if ports.is_empty() {
        return String::new();
    }
    let mut formatted: Vec<String> = ports
        .iter()
        .filter(|p| p.public_port.is_some()) // only show mapped ports
        .map(format_port)
        .collect();
    formatted.sort();
    formatted.dedup();
    formatted.join(", ")
}

/// Minimum column width for table rendering.
pub const MIN_TABLE_COLUMN_WIDTH: u16 = 12;

/// How many columns fit in the given width.
pub fn visible_column_count(width: u16, total: usize) -> usize {
    let usable = width.saturating_sub(2);
    let max_fit = (usable / MIN_TABLE_COLUMN_WIDTH) as usize;
    max_fit.min(total).max(1)
}

/// Style for a selected row in a table.
pub fn selected_row_style(is_focused: bool, is_selected: bool) -> Style {
    if is_focused && is_selected {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

/// Format a scroll indicator like " [1-4/8]".
pub fn format_scroll_hint(start: usize, visible: usize, total: usize) -> String {
    if total > visible {
        format!(
            " [{}-{}/{}]",
            start + 1,
            (start + visible).min(total),
            total
        )
    } else {
        String::new()
    }
}

/// Create a table header row from column names.
pub fn create_table_header(columns: &[String]) -> Row<'static> {
    let cells: Vec<Cell> = columns
        .iter()
        .map(|c| {
            Cell::from(format!(" {}", c)).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();
    Row::new(cells).height(1).bottom_margin(1)
}

/// Truncate a Docker/Podman ID, stripping sha256: prefix.
pub fn short_id(id: &str) -> String {
    id.strip_prefix("sha256:")
        .unwrap_or(id)
        .chars()
        .take(12)
        .collect()
}

/// Color based on percentage thresholds.
pub fn percentage_color(pct: f64) -> Color {
    if pct > 80.0 {
        Color::Red
    } else if pct > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

/// Truncate a string, adding ellipsis if too long.
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let keep = (max_len - 3) / 2;
    format!("{}...{}", &s[..keep], &s[s.len() - keep..])
}
