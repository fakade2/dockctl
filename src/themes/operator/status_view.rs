use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{AlertLevel, App};
use crate::display::{container_display_name, format_bytes, percentage_color, short_id};

pub(super) fn draw_status_view(frame: &mut Frame, area: Rect, app: &App) {
    let dim = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);
    let cyan = Style::default().fg(Color::Cyan);
    let green = Style::default().fg(Color::Green);
    let red = Style::default().fg(Color::Red);

    // Split area into top (system + docker) and bottom (top-cpu + top-mem + alerts)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // System stats + Docker stats
            Constraint::Min(1),    // Top containers + alerts
        ])
        .split(area);

    let top_halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[0]);

    let bottom_thirds = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(outer[1]);

    // --- System Stats ---
    let cpu_pct = app.host_cpu_percent;
    let mem_pct = if app.host_memory_total > 0 {
        (app.host_memory_used as f64 / app.host_memory_total as f64) * 100.0
    } else {
        0.0
    };
    let swap_pct = if app.host_swap_total > 0 {
        (app.host_swap_used as f64 / app.host_swap_total as f64) * 100.0
    } else {
        0.0
    };

    let cpu_bar = gauge_bar(cpu_pct, 20);
    let mem_bar = gauge_bar(mem_pct, 20);
    let swap_bar = gauge_bar(swap_pct, 20);

    let system_lines = vec![
        Line::from(vec![
            Span::styled(" CPU  ", dim),
            Span::styled(cpu_bar, Style::default().fg(percentage_color(cpu_pct))),
            Span::styled(format!(" {:.1}%", cpu_pct), white),
        ]),
        Line::from(vec![
            Span::styled(" RAM  ", dim),
            Span::styled(mem_bar, Style::default().fg(percentage_color(mem_pct))),
            Span::styled(
                format!(
                    " {:.1}% ({}/{})",
                    mem_pct,
                    format_bytes(app.host_memory_used),
                    format_bytes(app.host_memory_total)
                ),
                white,
            ),
        ]),
        Line::from(vec![
            Span::styled(" Swap ", dim),
            Span::styled(swap_bar, Style::default().fg(percentage_color(swap_pct))),
            Span::styled(
                format!(
                    " {:.1}% ({}/{})",
                    swap_pct,
                    format_bytes(app.host_swap_used),
                    format_bytes(app.host_swap_total)
                ),
                white,
            ),
        ]),
    ];

    let system_block = Block::default()
        .title(" System ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(dim);
    let system_widget = Paragraph::new(system_lines).block(system_block);
    frame.render_widget(system_widget, top_halves[0]);

    // --- Docker Stats ---
    let running = app
        .containers
        .iter()
        .filter(|c| c.state == "running")
        .count();
    let stopped = app.containers.len() - running;

    let docker_lines = vec![
        Line::from(vec![
            Span::styled(" Containers  ", dim),
            Span::styled(format!("{}", running), green),
            Span::styled(" running  ", dim),
            Span::styled(format!("{}", stopped), if stopped > 0 { red } else { dim }),
            Span::styled(" stopped", dim),
        ]),
        Line::from(vec![
            Span::styled(" Images      ", dim),
            Span::styled(format!("{}", app.images.len()), cyan),
        ]),
        Line::from(vec![
            Span::styled(" Volumes     ", dim),
            Span::styled(format!("{}", app.volumes.len()), cyan),
        ]),
        Line::from(vec![
            Span::styled(" Networks    ", dim),
            Span::styled(format!("{}", app.networks.len()), cyan),
        ]),
    ];

    let docker_block = Block::default()
        .title(" Docker ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(dim);
    let docker_widget = Paragraph::new(docker_lines).block(docker_block);
    frame.render_widget(docker_widget, top_halves[1]);

    // --- Top 5 by CPU ---
    let mut cpu_sorted: Vec<(&String, f64)> = app
        .container_stats
        .iter()
        .map(|(id, (cpu, _))| (id, *cpu))
        .collect();
    cpu_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    cpu_sorted.truncate(5);

    let cpu_lines: Vec<Line> = cpu_sorted
        .iter()
        .map(|(id, cpu)| {
            let name = app
                .containers
                .iter()
                .find(|c| &c.id == *id)
                .map(|c| container_display_name(&c.names))
                .unwrap_or_else(|| short_id(id));
            Line::from(vec![
                Span::styled(
                    format!(" {:.1}%", cpu),
                    Style::default().fg(percentage_color(*cpu)),
                ),
                Span::styled(format!("  {}", name), white),
            ])
        })
        .collect();

    let cpu_block = Block::default()
        .title(" Top CPU ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(dim);
    let cpu_widget = Paragraph::new(cpu_lines).block(cpu_block);
    frame.render_widget(cpu_widget, bottom_thirds[0]);

    // --- Top 5 by Memory ---
    let mut mem_sorted: Vec<(&String, u64)> = app
        .container_stats
        .iter()
        .map(|(id, (_, mem))| (id, *mem))
        .collect();
    mem_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    mem_sorted.truncate(5);

    let mem_lines: Vec<Line> = mem_sorted
        .iter()
        .map(|(id, mem)| {
            let name = app
                .containers
                .iter()
                .find(|c| &c.id == *id)
                .map(|c| container_display_name(&c.names))
                .unwrap_or_else(|| short_id(id));
            Line::from(vec![
                Span::styled(format!(" {}", format_bytes(*mem)), cyan),
                Span::styled(format!("  {}", name), white),
            ])
        })
        .collect();

    let mem_block = Block::default()
        .title(" Top Memory ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(dim);
    let mem_widget = Paragraph::new(mem_lines).block(mem_block);
    frame.render_widget(mem_widget, bottom_thirds[1]);

    // --- Alerts ---
    let alert_lines: Vec<Line> = if app.alerts.is_empty() {
        vec![Line::from(Span::styled(" No active alerts", dim))]
    } else {
        app.alerts
            .iter()
            .map(|alert| {
                let (icon, color) = match alert.level {
                    AlertLevel::Critical => ("\u{25cf}", Color::Red),
                    AlertLevel::Warning => ("\u{25cf}", Color::Yellow),
                };
                Line::from(vec![
                    Span::styled(format!(" {} ", icon), Style::default().fg(color)),
                    Span::styled(alert.message.clone(), Style::default().fg(color)),
                ])
            })
            .collect()
    };

    let alert_block = Block::default()
        .title(" Alerts ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(dim);
    let alert_widget = Paragraph::new(alert_lines).block(alert_block);
    frame.render_widget(alert_widget, bottom_thirds[2]);
}

/// Build a text-based gauge bar: [========------] for a given percentage.
fn gauge_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!(
        "[{}{}]",
        "\u{2588}".repeat(filled),
        "\u{2500}".repeat(empty),
    )
}
