use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Table},
    Frame,
};

use crate::app::{App, PanelId};
use crate::ui::{
    create_table_header, format_scroll_hint, panel_block, selected_row_style, short_id,
    visible_column_count,
};

/// Renders the Networks panel (panel 7, right column).
/// Columns are configurable and scrollable with Left/Right.
pub fn draw_network_panel(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    is_expanded: bool,
    is_focused: bool,
    columns: &[String],
) {
    let count = app.networks.len();
    let title = format!("Networks ({})", count);

    if !is_expanded {
        let block = panel_block(&title, PanelId::Networks.number(), is_focused);
        frame.render_widget(block, area);
        return;
    }

    let selected = app.panel_selection(PanelId::Networks);
    let col_offset = app.panel_column_offset(PanelId::Networks);
    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_table_header(visible_cols);

    let rows: Vec<Row> = app
        .networks
        .iter()
        .enumerate()
        .map(|(i, net)| {
            let style = selected_row_style(is_focused, i == selected);

            let cells: Vec<Cell> = visible_cols
                .iter()
                .map(|col| match col.as_str() {
                    "Name" => Cell::from(format!(" {}", net.name))
                        .style(Style::default().fg(Color::White)),
                    "ID" => Cell::from(short_id(&net.id))
                        .style(Style::default().fg(Color::DarkGray)),
                    "Driver" => {
                        let color = match net.driver.as_str() {
                            "bridge" => Color::Green,
                            "host" => Color::Yellow,
                            "overlay" => Color::Blue,
                            "macvlan" => Color::Magenta,
                            "none" => Color::DarkGray,
                            _ => Color::White,
                        };
                        Cell::from(net.driver.clone()).style(Style::default().fg(color))
                    }
                    "Scope" => Cell::from(net.scope.clone())
                        .style(Style::default().fg(Color::DarkGray)),
                    "Containers" => {
                        let count = net.containers.len();
                        Cell::from(format!("{}", count))
                            .style(Style::default().fg(if count > 0 { Color::Green } else { Color::DarkGray }))
                    }
                    _ => Cell::from(""),
                })
                .collect();

            Row::new(cells).style(style)
        })
        .collect();

    let col_widths: Vec<Constraint> = visible_cols
        .iter()
        .map(|col| match col.as_str() {
            "Name" => Constraint::Percentage(30),
            "ID" => Constraint::Length(14),
            "Driver" => Constraint::Length(10),
            "Scope" => Constraint::Length(8),
            "Containers" => Constraint::Length(12),
            _ => Constraint::Percentage((100 / visible_cols.len() as u16).max(1)),
        })
        .collect();

    let scroll_hint = format_scroll_hint(start, max_visible, columns.len());

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(panel_block(
            &format!("Networks ({}){}", count, scroll_hint),
            PanelId::Networks.number(),
            is_focused,
        ))
        .row_highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    frame.render_widget(table, area);
}

