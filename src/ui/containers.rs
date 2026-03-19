use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style, Stylize},
    widgets::{Cell, Row, Table},
    Frame,
    layout::Rect,
};

use crate::app::App;
use crate::ui::{container_display_name, format_ports, panel_block};

/// Renders the container list as a table in the left panel.
pub fn draw_container_list(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let header = Row::new(vec![
        Cell::from(" Name").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Image").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Ports").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .containers
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let sc = state_color(&c.state);
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
                Cell::from(c.status.clone()).style(Style::default().fg(sc)),
                Cell::from(format_ports(&c.ports)).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let title = format!("Containers ({})", app.containers.len());
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
    .block(panel_block(&title, is_focused))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_widget(table, area);
}

/// Returns a color appropriate for the container state.
fn state_color(state: &str) -> Color {
    match state {
        "running" => Color::Green,
        "paused" => Color::Yellow,
        "restarting" => Color::Blue,
        "created" => Color::Gray,
        _ => Color::Red,
    }
}
