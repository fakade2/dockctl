pub mod alerts;
pub mod containers;
pub mod images;
pub mod logs;
pub mod networks;
pub mod pods;
pub mod preview;
pub mod resources;
pub mod stats;
pub mod status;
pub mod volumes;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Wrap},
};

use crate::app::{App, PanelId};
use crate::config::DockctlConfig;
use crate::runtime::PortMapping;

// ─── Public entry point ──────────────────────────────────────────────────────

/// Root render function — 7-panel layout with left sidebar, right accordion,
/// bottom status bar, and optional popup overlay.
pub fn draw(frame: &mut Frame, app: &App, config: &DockctlConfig) {
    let layout = &config.layout;
    // 1. Split vertically: main area (fills) + bottom status bar (1 line)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let main_area = outer[0];
    let bar_area = outer[1];

    // 2. Split main area horizontally: left column + right column
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(layout.left_panel_percent),
            Constraint::Percentage(100 - layout.left_panel_percent),
        ])
        .split(main_area);

    let left_area = columns[0];
    let right_area = columns[1];

    // 3. Left column: 3 panels stacked equally
    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(left_area);

    alerts::draw_alerts_panel(
        frame,
        left_panels[0],
        app,
        app.active_panel == PanelId::Alerts,
    );
    resources::draw_resources_panel(
        frame,
        left_panels[1],
        app,
        app.active_panel == PanelId::Resources,
    );
    status::draw_status_panel(
        frame,
        left_panels[2],
        app,
        app.active_panel == PanelId::Status,
    );

    // 4. Right column: log view, detail view, or accordion
    if app.log_view_active {
        draw_log_view(frame, right_area, app);
    } else if app.detail_view_active {
        draw_detail_view(frame, right_area, app);
    } else {
        draw_right_accordion(frame, right_area, app, config);
    }

    // 5. Bottom status bar
    draw_status_bar(frame, bar_area);

    // 6. Popup overlay (if open)
    if app.show_popup {
        draw_popup(frame, app);
    }
}

/// Renders the right column with accordion behavior: the expanded panel gets
/// most of the vertical space, collapsed panels get just 3 lines (title bar).
fn draw_right_accordion(frame: &mut Frame, area: Rect, app: &App, config: &DockctlConfig) {
    let right_panels = PanelId::RIGHT;
    let expanded = app.expanded_right;

    // Build constraints: expanded gets Min(8), collapsed get Length(3)
    let constraints: Vec<Constraint> = right_panels
        .iter()
        .map(|&p| {
            if p == expanded {
                Constraint::Min(8)
            } else {
                Constraint::Length(3)
            }
        })
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (i, &panel) in right_panels.iter().enumerate() {
        let is_expanded = panel == expanded;
        let is_focused = panel == app.active_panel;

        match panel {
            PanelId::Containers => {
                containers::draw_container_panel(
                    frame,
                    chunks[i],
                    app,
                    is_expanded,
                    is_focused,
                    &config.columns.containers,
                );
            }
            PanelId::Images => {
                images::draw_image_panel(
                    frame,
                    chunks[i],
                    app,
                    is_expanded,
                    is_focused,
                    &config.columns.images,
                );
            }
            PanelId::Volumes => {
                volumes::draw_volume_panel(
                    frame,
                    chunks[i],
                    app,
                    is_expanded,
                    is_focused,
                    &config.columns.volumes,
                );
            }
            PanelId::Networks => {
                networks::draw_network_panel(
                    frame,
                    chunks[i],
                    app,
                    is_expanded,
                    is_focused,
                    &config.columns.networks,
                );
            }
            _ => {}
        }
    }
}

/// Renders the bottom status bar.
fn draw_status_bar(frame: &mut Frame, area: Rect) {
    let version = concat!("dockctl v", env!("CARGO_PKG_VERSION"));

    let key_style = Style::default().fg(Color::Cyan);
    let label_style = Style::default().fg(Color::DarkGray);
    let sep_style = Style::default().fg(Color::DarkGray);

    let left_spans = vec![
        Span::styled(" Explain: ", label_style),
        Span::styled("e", key_style),
        Span::styled(" | ", sep_style),
        Span::styled("Quit: ", label_style),
        Span::styled("q", key_style),
        Span::styled(" | ", sep_style),
        Span::styled("Keybindings: ", label_style),
        Span::styled("?", key_style),
    ];

    // Calculate padding to right-align the version string
    let left_len: usize = left_spans.iter().map(|s| s.width()).sum();
    let right_len = version.len() + 1;
    let total_width = area.width as usize;
    let padding = total_width.saturating_sub(left_len + right_len);

    let mut spans = left_spans;
    spans.push(Span::raw(" ".repeat(padding)));
    spans.push(Span::styled(
        format!("{} ", version),
        Style::default().fg(Color::DarkGray),
    ));

    let line = Line::from(spans);
    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
}

/// Renders the log viewer (shared across themes).
fn draw_log_view(frame: &mut Frame, area: Rect, app: &App) {
    let follow_indicator = if app.log_view_follow { " [FOLLOW]" } else { "" };
    let title = format!(
        " Logs: {} {} ",
        app.log_view_container_name, follow_indicator
    );

    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    let inner_height = inner.height as usize;

    let styled_lines: Vec<Line> = app
        .log_view_lines
        .iter()
        .map(|line| {
            let style =
                if line.contains("ERROR") || line.contains("FATAL") || line.contains("PANIC") {
                    Style::default().fg(Color::Red)
                } else if line.contains("WARN") {
                    Style::default().fg(Color::Yellow)
                } else if line.contains("INFO") {
                    Style::default().fg(Color::Green)
                } else if line.contains("DEBUG") || line.contains("TRACE") {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };
            Line::from(Span::styled(line.as_str(), style))
        })
        .collect();

    let scroll = if app.log_view_follow {
        styled_lines.len().saturating_sub(inner_height) as u16
    } else {
        app.log_view_scroll
    };

    let paragraph = Paragraph::new(styled_lines)
        .block(block)
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}

/// Renders the inline detail view (shared across themes).
fn draw_detail_view(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(" {} ", app.detail_view_title))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let content_lines: Vec<Line> = app
        .detail_view_content
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(Color::White))))
        .collect();

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_view_scroll, 0));

    frame.render_widget(paragraph, area);
}

/// Renders the popup overlay centered on the screen.
fn draw_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // 60% width, 70% height, centered
    let popup_width = (area.width as u32 * 60 / 100) as u16;
    let popup_height = (area.height as u32 * 70 / 100) as u16;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", app.popup_title))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let content_lines: Vec<Line> = app
        .popup_content
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(Color::White))))
        .collect();

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.popup_scroll, 0));

    frame.render_widget(paragraph, popup_area);
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

/// Creates a bordered block with a numbered badge in the title.
/// Example: " [4] Containers (9) "
/// Number is in brackets, dimmed. Header text is high contrast (white bold when
/// focused, white when unfocused). Border is cyan when focused, dark gray otherwise.
pub(crate) fn panel_block(title: &str, number: u8, is_focused: bool) -> Block<'static> {
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

/// Formats a `PortMapping` as "source:destination" (e.g. "5432:5432").
/// Strips protocol, `0.0.0.0`, and `::` — just shows the port numbers.
pub(crate) fn format_port(pm: &PortMapping) -> String {
    match pm.public_port {
        Some(pub_port) => format!("{}:{}", pub_port, pm.private_port),
        None => format!("{}", pm.private_port),
    }
}

/// Formats a list of port mappings into a deduplicated, sorted, comma-separated string.
/// Docker/bollard returns duplicate entries (IPv4 + IPv6, per protocol), so we deduplicate.
pub(crate) fn format_ports(ports: &[PortMapping]) -> String {
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
pub(crate) const MIN_TABLE_COLUMN_WIDTH: u16 = 12;

/// How many columns fit in the given width.
pub(crate) fn visible_column_count(width: u16, total: usize) -> usize {
    let usable = width.saturating_sub(2);
    let max_fit = (usable / MIN_TABLE_COLUMN_WIDTH) as usize;
    max_fit.min(total).max(1)
}

/// Style for a selected row in a table.
pub(crate) fn selected_row_style(is_focused: bool, is_selected: bool) -> Style {
    if is_focused && is_selected {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

/// Format a scroll indicator like " [1-4/8]".
pub(crate) fn format_scroll_hint(start: usize, visible: usize, total: usize) -> String {
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
pub(crate) fn create_table_header(columns: &[String]) -> Row<'static> {
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
pub(crate) fn short_id(id: &str) -> String {
    id.strip_prefix("sha256:")
        .unwrap_or(id)
        .chars()
        .take(12)
        .collect()
}

/// Color based on percentage thresholds.
pub(crate) fn percentage_color(pct: f64) -> Color {
    if pct > 80.0 {
        Color::Red
    } else if pct > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

/// Truncate a string, adding ellipsis if too long.
pub(crate) fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let keep = (max_len - 3) / 2;
    format!("{}...{}", &s[..keep], &s[s.len() - keep..])
}
