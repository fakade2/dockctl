use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Cell, Row, Table},
    Frame,
};

use crate::app::App;
use crate::config::LayoutConfig;
use crate::ui::titled_block;

/// Renders the Networks view — a table listing all networks with name, short
/// ID, driver, and scope.
pub fn draw_networks(frame: &mut Frame, area: Rect, app: &App, _layout: &LayoutConfig) {
    let header = Row::new(vec![
        Cell::from(" Name").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("ID").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Driver").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Scope").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .networks
        .iter()
        .enumerate()
        .map(|(i, net)| {
            let short_id: String = net.id.chars().take(12).collect();

            let driver_color = match net.driver.as_str() {
                "bridge" => Color::Green,
                "host" => Color::Yellow,
                "overlay" => Color::Blue,
                "macvlan" => Color::Magenta,
                "none" => Color::DarkGray,
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
                Cell::from(format!(" {}", net.name)).style(Style::default().fg(Color::White)),
                Cell::from(short_id).style(Style::default().fg(Color::DarkGray)),
                Cell::from(net.driver.clone()).style(Style::default().fg(driver_color)),
                Cell::from(net.scope.clone()).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let title = format!("Networks ({})", app.networks.len());
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(20),
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
