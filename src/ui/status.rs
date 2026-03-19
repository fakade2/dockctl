use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, PanelId};
use crate::ui::{format_bytes, panel_block, percentage_color};

/// Renders the Status panel (panel 3, left column).
///
/// Two sections showing used/total with percentage:
/// - System: CPU, RAM, Swap
/// - Docker: CPU, RAM, Disk (used/allocated)
pub fn draw_status_panel(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let block = panel_block("Status", PanelId::Status.number(), is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let selected = app.panel_selection(PanelId::Status);
    let mut rows: Vec<Line> = Vec::new();

    // ── System ────────────────────────────────────────────────────
    rows.push(section_header("System"));

    // Row 0: CPU
    let cpu_pct = app.host_cpu_percent;
    rows.push(make_stat_row("CPU", cpu_pct, None, is_focused && selected == 0));

    // Row 1: RAM
    let mem_pct = if app.host_memory_total > 0 {
        (app.host_memory_used as f64 / app.host_memory_total as f64) * 100.0
    } else {
        0.0
    };
    rows.push(make_stat_row(
        "RAM",
        mem_pct,
        Some(&format!("{} / {}", format_bytes(app.host_memory_used), format_bytes(app.host_memory_total))),
        is_focused && selected == 1,
    ));

    // Row 2: Swap
    if app.host_swap_total > 0 {
        let swap_pct = (app.host_swap_used as f64 / app.host_swap_total as f64) * 100.0;
        rows.push(make_stat_row(
            "Swap",
            swap_pct,
            Some(&format!("{} / {}", format_bytes(app.host_swap_used), format_bytes(app.host_swap_total))),
            is_focused && selected == 2,
        ));
    } else {
        rows.push(make_label_row("Swap", "N/A", is_focused && selected == 2));
    }

    // ── Docker ────────────────────────────────────────────────────
    rows.push(section_header("Docker"));

    match &app.system_info {
        Some(info) => {
            // Row 3: Docker CPU — aggregate from container_stats cache
            let docker_cpu: f64 = app.container_stats.values().map(|(cpu, _)| cpu).sum();
            rows.push(make_stat_row(
                "CPU",
                docker_cpu.min(100.0 * info.cpus as f64),
                Some(&format!("{} containers / {} cores", info.containers_running, info.cpus)),
                is_focused && selected == 3,
            ));

            // Row 4: Docker RAM — aggregate from container_stats cache
            let docker_mem: u64 = app.container_stats.values().map(|(_, mem)| mem).sum();
            let docker_mem_pct = if info.memory_total > 0 {
                (docker_mem as f64 / info.memory_total as f64) * 100.0
            } else {
                0.0
            };
            rows.push(make_stat_row(
                "RAM",
                docker_mem_pct,
                Some(&format!("{} / {}", format_bytes(docker_mem), format_bytes(info.memory_total))),
                is_focused && selected == 4,
            ));

            // Row 5: Docker Disk — images used vs total allocated
            let images_size: u64 = app.images.iter().map(|i| i.size).sum();
            let volumes_count = app.volumes.len();
            rows.push(make_label_row(
                "Disk",
                &format!("{} imgs, {} vols", format_bytes(images_size), volumes_count),
                is_focused && selected == 5,
            ));
        }
        None => {
            rows.push(make_label_row("CPU", "connecting...", is_focused && selected == 3));
            rows.push(make_label_row("RAM", "connecting...", is_focused && selected == 4));
            rows.push(make_label_row("Disk", "connecting...", is_focused && selected == 5));
        }
    }

    let paragraph = Paragraph::new(rows);
    frame.render_widget(paragraph, inner);
}

fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" ── {} ", title),
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
    ))
}

/// Stat row: " CPU  48.2%  4.2G / 16G "
/// Shows label, colored percentage, and optional detail text.
fn make_stat_row(label: &str, pct: f64, detail: Option<&str>, is_selected: bool) -> Line<'static> {
    let bg = if is_selected { Color::DarkGray } else { Color::Reset };
    let bold = if is_selected { Modifier::BOLD } else { Modifier::empty() };
    let color = percentage_color(pct);

    let mut spans = vec![
        Span::styled(
            format!(" {:<4} ", label),
            Style::default().fg(Color::Cyan).bg(bg).add_modifier(Modifier::BOLD | bold),
        ),
        Span::styled(
            format!("{:>5.1}%", pct),
            Style::default().fg(color).bg(bg).add_modifier(Modifier::BOLD | bold),
        ),
    ];

    if let Some(d) = detail {
        spans.push(Span::styled(
            format!("  {}", d),
            Style::default().fg(Color::DarkGray).bg(bg).add_modifier(bold),
        ));
    }

    Line::from(spans)
}

/// Simple label + value row without percentage.
fn make_label_row(label: &str, value: &str, is_selected: bool) -> Line<'static> {
    let bg = if is_selected { Color::DarkGray } else { Color::Reset };
    let bold = if is_selected { Modifier::BOLD } else { Modifier::empty() };

    Line::from(vec![
        Span::styled(
            format!(" {:<4} ", label),
            Style::default().fg(Color::Cyan).bg(bg).add_modifier(Modifier::BOLD | bold),
        ),
        Span::styled(
            value.to_string(),
            Style::default().fg(Color::White).bg(bg).add_modifier(bold),
        ),
    ])
}

