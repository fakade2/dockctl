use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::config::LayoutConfig;
use crate::ui::{container_display_name, format_ports, logs, stats, titled_block};

/// Renders the Containers view.
///
/// Layout:
/// - Left (or full width):  container list table
/// - Right (if detail):     detail panel showing inspect data
/// - Bottom (if logs):      log viewer panel
pub fn draw_containers(frame: &mut Frame, area: Rect, app: &App, layout: &LayoutConfig) {
    let has_logs = !app.log_lines.is_empty();
    let has_stats = app.stats.is_some();

    // Vertical split: main content vs bottom panels (logs / stats).
    let vert_chunks = if has_logs || has_stats {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(layout.vertical_split_percent),
                Constraint::Percentage(100 - layout.vertical_split_percent),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    };

    // Horizontal split: table vs detail panel.
    let main_area = vert_chunks[0];
    let horiz_chunks = if app.show_detail {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(layout.main_split_percent),
                Constraint::Percentage(100 - layout.main_split_percent),
            ])
            .split(main_area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(main_area)
    };

    draw_container_table(frame, horiz_chunks[0], app);

    if app.show_detail {
        draw_detail_panel(frame, horiz_chunks[1], app);
    }

    // Bottom panels
    if has_logs || has_stats {
        let bottom = vert_chunks[1];
        if has_logs && has_stats {
            let bottom_split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(bottom);
            logs::draw_logs(bottom_split[0], frame, &app.log_lines);
            if let Some(ref s) = app.stats {
                stats::draw_stats(bottom_split[1], frame, s);
            }
        } else if has_logs {
            logs::draw_logs(bottom, frame, &app.log_lines);
        } else if let Some(ref s) = app.stats {
            stats::draw_stats(bottom, frame, s);
        }
    }
}

// ─── Container table ─────────────────────────────────────────────────────────

fn draw_container_table(frame: &mut Frame, area: Rect, app: &App) {
    let header = Row::new(vec![
        Cell::from(" Name").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Image").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Mem").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Ports").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .containers
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let state_color = state_color(&c.state);
            let style = if i == app.selected_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(format!(" {}", container_display_name(&c.names)))
                    .style(Style::default().fg(Color::White)),
                Cell::from(c.image.clone()).style(Style::default().fg(Color::Gray)),
                Cell::from(c.status.clone()).style(Style::default().fg(state_color)),
                Cell::from("--").style(Style::default().fg(Color::DarkGray)),
                Cell::from(format_ports(&c.ports)).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let title = format!("Containers ({})", app.containers.len());
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(22),
            Constraint::Percentage(28),
            Constraint::Percentage(18),
            Constraint::Percentage(10),
            Constraint::Percentage(22),
        ],
    )
    .header(header)
    .block(titled_block(&title))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_widget(table, area);
}

// ─── Detail panel ────────────────────────────────────────────────────────────

fn draw_detail_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = titled_block("Detail");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = match &app.detail_container {
        Some(detail) => {
            let mut l: Vec<Line> = Vec::new();

            // Basic info
            l.push(section_header("Info"));
            l.push(kv_line("Name", &detail.name));
            l.push(kv_line("ID", &short_id(&detail.id)));
            l.push(kv_line("Image", &detail.image));
            l.push(kv_line("State", &detail.state.status));
            l.push(kv_line("PID", &detail.state.pid.to_string()));
            l.push(kv_line("Created", &detail.created));
            l.push(Line::default());

            // Command
            let full_cmd = if detail.args.is_empty() {
                detail.path.clone()
            } else {
                format!("{} {}", detail.path, detail.args.join(" "))
            };
            l.push(section_header("Command"));
            l.push(Line::from(Span::styled(
                format!("  {}", full_cmd),
                Style::default().fg(Color::White),
            )));
            l.push(Line::default());

            // State details
            l.push(section_header("State"));
            l.push(kv_line(
                "Running",
                if detail.state.running { "yes" } else { "no" },
            ));
            l.push(kv_line(
                "Paused",
                if detail.state.paused { "yes" } else { "no" },
            ));
            l.push(kv_line("ExitCode", &detail.state.exit_code.to_string()));
            l.push(kv_line("StartedAt", &detail.state.started_at));
            if !detail.state.finished_at.is_empty()
                && detail.state.finished_at != "0001-01-01T00:00:00Z"
            {
                l.push(kv_line("FinishedAt", &detail.state.finished_at));
            }
            l.push(Line::default());

            // Environment variables
            if !detail.env.is_empty() {
                l.push(section_header("Environment"));
                for env in &detail.env {
                    l.push(Line::from(Span::styled(
                        format!("  {}", env),
                        Style::default().fg(Color::White),
                    )));
                }
                l.push(Line::default());
            }

            // Labels
            if !detail.labels.is_empty() {
                l.push(section_header("Labels"));
                for (k, v) in &detail.labels {
                    l.push(Line::from(vec![
                        Span::styled(format!("  {}", k), Style::default().fg(Color::Yellow)),
                        Span::styled("=", Style::default().fg(Color::DarkGray)),
                        Span::styled(v.to_string(), Style::default().fg(Color::White)),
                    ]));
                }
            }

            l
        }
        None => vec![Line::from(Span::styled(
            " No container selected",
            Style::default().fg(Color::DarkGray).italic(),
        ))],
    };

    let paragraph = Paragraph::new(lines).scroll((0, 0));
    frame.render_widget(paragraph, inner);
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" [{}]", title),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Produces a key-value line. Converts values to owned strings to avoid
/// lifetime issues with temporaries.
fn kv_line(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<12} ", key),
            Style::default().fg(Color::DarkGray).bold(),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

fn short_id(id: &str) -> String {
    id.chars().take(12).collect()
}

fn state_color(state: &str) -> Color {
    match state {
        "running" => Color::Green,
        "paused" => Color::Yellow,
        "restarting" => Color::Blue,
        "created" => Color::Gray,
        _ => Color::Red,
    }
}
