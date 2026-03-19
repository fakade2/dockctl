use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::runtime::ContainerStats;
use crate::ui::{format_bytes, panel_block};

/// Renders a compact stats panel showing CPU, memory, network, and block I/O
/// for a single container.
pub fn draw_stats(area: Rect, frame: &mut Frame, stats: &ContainerStats) {
    let title = format!("Stats: {}", stats.name);
    let block = panel_block(&title, 0, false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    // Left column: CPU & Memory
    let cpu_color = if stats.cpu_percent > 80.0 {
        Color::Red
    } else if stats.cpu_percent > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let mem_pct = if stats.memory_limit > 0 {
        (stats.memory_usage as f64 / stats.memory_limit as f64) * 100.0
    } else {
        0.0
    };
    let mem_color = if mem_pct > 80.0 {
        Color::Red
    } else if mem_pct > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let cpu_bar = gauge_bar(stats.cpu_percent, 20, cpu_color);
    let mem_bar = gauge_bar(mem_pct, 20, mem_color);

    let left_lines = vec![
        Line::default(),
        stat_label_line("CPU"),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{:.1}%", stats.cpu_percent),
                Style::default()
                    .fg(cpu_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            cpu_bar,
        ]),
        Line::default(),
        stat_label_line("Memory"),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(
                    "{} / {}",
                    format_bytes(stats.memory_usage),
                    format_bytes(stats.memory_limit),
                ),
                Style::default().fg(mem_color),
            ),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{:.1}%", mem_pct),
                Style::default()
                    .fg(mem_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            mem_bar,
        ]),
    ];

    // Right column: Network & Block I/O
    let right_lines = vec![
        Line::default(),
        stat_label_line("Network I/O"),
        Line::from(vec![
            Span::styled("  RX  ", Style::default().fg(Color::DarkGray).bold()),
            Span::styled(
                format_bytes(stats.network_rx),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("  TX  ", Style::default().fg(Color::DarkGray).bold()),
            Span::styled(
                format_bytes(stats.network_tx),
                Style::default().fg(Color::Blue),
            ),
        ]),
        Line::default(),
        stat_label_line("Block I/O"),
        Line::from(vec![
            Span::styled("  Read  ", Style::default().fg(Color::DarkGray).bold()),
            Span::styled(
                format_bytes(stats.block_read),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Write ", Style::default().fg(Color::DarkGray).bold()),
            Span::styled(
                format_bytes(stats.block_write),
                Style::default().fg(Color::Magenta),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(left_lines), cols[0]);
    frame.render_widget(Paragraph::new(right_lines), cols[1]);
}

fn gauge_bar(pct: f64, width: usize, color: Color) -> Span<'static> {
    let clamped = pct.clamp(0.0, 100.0);
    let filled = ((clamped / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    let bar = format!(
        "[{}{}]",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
    );

    Span::styled(bar, Style::default().fg(color))
}

fn stat_label_line(label: &str) -> Line<'_> {
    Line::from(Span::styled(
        format!("  {}", label),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}
