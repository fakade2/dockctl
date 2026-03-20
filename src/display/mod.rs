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
        Span::styled("───", sep_style),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::PortMapping;

    // format_bytes
    #[test]
    fn format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }
    #[test]
    fn format_bytes_bytes() {
        assert_eq!(format_bytes(500), "500 B");
    }
    #[test]
    fn format_bytes_kb() {
        assert_eq!(format_bytes(2048), "2 KB");
    }
    #[test]
    fn format_bytes_mb() {
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    }
    #[test]
    fn format_bytes_gb() {
        assert_eq!(format_bytes(2 * 1024 * 1024 * 1024), "2.00 GB");
    }

    // short_id
    #[test]
    fn short_id_strips_sha256() {
        assert_eq!(short_id("sha256:abcdef123456789"), "abcdef123456");
    }
    #[test]
    fn short_id_no_prefix() {
        assert_eq!(short_id("abcdef123456789"), "abcdef123456");
    }
    #[test]
    fn short_id_short_input() {
        assert_eq!(short_id("abc"), "abc");
    }
    #[test]
    fn short_id_empty() {
        assert_eq!(short_id(""), "");
    }

    // container_display_name
    #[test]
    fn display_name_strips_slash() {
        assert_eq!(container_display_name(&["/nginx".into()]), "nginx");
    }
    #[test]
    fn display_name_no_slash() {
        assert_eq!(container_display_name(&["nginx".into()]), "nginx");
    }
    #[test]
    fn display_name_empty() {
        assert_eq!(container_display_name(&[]), "<unnamed>");
    }
    #[test]
    fn display_name_multiple() {
        assert_eq!(container_display_name(&["/a".into(), "/b".into()]), "a");
    }

    // format_port
    #[test]
    fn format_port_mapped() {
        let pm = PortMapping {
            ip: Some("0.0.0.0".into()),
            private_port: 80,
            public_port: Some(8080),
            port_type: "tcp".into(),
        };
        assert_eq!(format_port(&pm), "8080:80");
    }
    #[test]
    fn format_port_no_public() {
        let pm = PortMapping {
            ip: None,
            private_port: 80,
            public_port: None,
            port_type: "tcp".into(),
        };
        assert_eq!(format_port(&pm), "80");
    }

    // format_ports
    #[test]
    fn format_ports_empty() {
        assert_eq!(format_ports(&[]), "");
    }
    #[test]
    fn format_ports_deduplicates() {
        let ports = vec![
            PortMapping {
                ip: Some("0.0.0.0".into()),
                private_port: 80,
                public_port: Some(8080),
                port_type: "tcp".into(),
            },
            PortMapping {
                ip: Some("::".into()),
                private_port: 80,
                public_port: Some(8080),
                port_type: "tcp".into(),
            },
        ];
        assert_eq!(format_ports(&ports), "8080:80"); // deduplicated
    }

    // truncate_str
    #[test]
    fn truncate_short() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }
    #[test]
    fn truncate_long() {
        let result = truncate_str("a_very_long_path_name", 10);
        assert!(result.contains("..."));
        assert!(result.len() <= 10);
    }

    // percentage_color
    #[test]
    fn pct_color_green() {
        assert_eq!(percentage_color(30.0), Color::Green);
    }
    #[test]
    fn pct_color_yellow() {
        assert_eq!(percentage_color(60.0), Color::Yellow);
    }
    #[test]
    fn pct_color_red() {
        assert_eq!(percentage_color(90.0), Color::Red);
    }

    // visible_column_count
    #[test]
    fn visible_cols_narrow() {
        assert_eq!(visible_column_count(24, 10), 1);
    }
    #[test]
    fn visible_cols_wide() {
        assert_eq!(visible_column_count(100, 10), 8);
    }
    #[test]
    fn visible_cols_capped() {
        assert_eq!(visible_column_count(200, 3), 3);
    }

    // format_scroll_hint
    #[test]
    fn scroll_hint_no_scroll() {
        assert_eq!(format_scroll_hint(0, 5, 5), "");
    }
    #[test]
    fn scroll_hint_scrollable() {
        assert_eq!(format_scroll_hint(0, 3, 8), " [1-3/8]");
    }
    #[test]
    fn scroll_hint_offset() {
        assert_eq!(format_scroll_hint(2, 3, 8), " [3-5/8]");
    }

    // selected_row_style
    #[test]
    fn row_style_selected() {
        let s = selected_row_style(true, true);
        assert_ne!(s, Style::default());
    }
    #[test]
    fn row_style_not_selected() {
        assert_eq!(selected_row_style(true, false), Style::default());
    }
    #[test]
    fn row_style_not_focused() {
        assert_eq!(selected_row_style(false, true), Style::default());
    }

    // create_table_header
    #[test]
    fn table_header_columns() {
        let cols = vec!["Name".into(), "Status".into()];
        let _row = create_table_header(&cols);
    }

    // collapsed_panel_line
    #[test]
    fn collapsed_line_renders() {
        let line = collapsed_panel_line("Test", 1, false);
        assert!(!line.spans.is_empty());
    }
    #[test]
    fn collapsed_line_focused() {
        let line = collapsed_panel_line("Test", 1, true);
        assert!(!line.spans.is_empty());
    }

    // panel_block
    #[test]
    fn panel_block_creates() {
        let _block = panel_block("Test", 1, false);
    }
    #[test]
    fn panel_block_focused() {
        let _block = panel_block("Test", 1, true);
    }
}
