use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Cell, Row, Table},
    Frame,
};

use crate::app::App;
use crate::config::LayoutConfig;
use crate::ui::{container_display_name, format_ports, titled_block};

/// Renders the Dashboard view: a full-area table of currently running containers.
/// System info and resource counts are now displayed in the sidebar.
pub fn draw_dashboard(frame: &mut Frame, area: Rect, app: &App, _layout: &LayoutConfig) {
    draw_running_table(frame, area, app);
}

// ─── Running containers table ────────────────────────────────────────────────

fn draw_running_table(frame: &mut Frame, area: Rect, app: &App) {
    let running: Vec<_> = app
        .containers
        .iter()
        .filter(|c| c.state == "running")
        .collect();

    let header = Row::new(vec![
        Cell::from(" Name").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Image").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Ports").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = running
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let state_color = match c.state.as_str() {
                "running" => Color::Green,
                "paused" => Color::Yellow,
                _ => Color::Red,
            };
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
                Cell::from(c.image.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(c.status.clone()).style(Style::default().fg(state_color)),
                Cell::from(format_ports(&c.ports)).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let title = format!("Running Containers ({})", running.len());
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(25),
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
