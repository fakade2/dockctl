use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::config::LayoutConfig;
use crate::ui::titled_block;

/// Renders the Pods view.
///
/// If the active runtime does not support pods (Docker), a notice is shown
/// instead of the table.
pub fn draw_pods(frame: &mut Frame, area: Rect, app: &App, _layout: &LayoutConfig) {
    if !app.supports_pods {
        draw_unsupported(frame, area);
        return;
    }

    draw_pod_table(frame, area, app);
}

// ─── Unsupported runtime notice ──────────────────────────────────────────────

fn draw_unsupported(frame: &mut Frame, area: Rect) {
    let block = titled_block("Pods");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let v_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Length(3),
            Constraint::Percentage(40),
        ])
        .split(inner);

    let msg = Paragraph::new(Line::from(vec![Span::styled(
        "Pods are only available with Podman runtime",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::ITALIC),
    )]))
    .centered();

    frame.render_widget(msg, v_layout[1]);
}

// ─── Pod table ───────────────────────────────────────────────────────────────

fn draw_pod_table(frame: &mut Frame, area: Rect, app: &App) {
    let header = Row::new(vec![
        Cell::from(" Name").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Containers").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Created").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .pods
        .iter()
        .enumerate()
        .map(|(i, pod)| {
            let status_color = match pod.status.as_str() {
                "Running" => Color::Green,
                "Degraded" => Color::Yellow,
                "Created" => Color::Gray,
                "Exited" => Color::Red,
                _ => Color::White,
            };

            let style = if i == app.selected_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!(" {}", pod.name)).style(Style::default().fg(Color::White)),
                Cell::from(pod.status.clone()).style(Style::default().fg(status_color)),
                Cell::from(format!("{}", pod.containers.len()))
                    .style(Style::default().fg(Color::Yellow)),
                Cell::from(pod.created.clone()).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let title = format!("Pods ({})", app.pods.len());
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
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
