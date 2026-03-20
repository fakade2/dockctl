use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Table},
};

use crate::app::{App, PanelId};
use crate::display::{
    collapsed_panel_line, create_table_header, format_scroll_hint, panel_block, selected_row_style,
    truncate_str, visible_column_count,
};

/// Renders the Volumes panel (panel 6, right column).
/// Columns are configurable and scrollable with Left/Right.
pub fn draw_volume_panel(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    is_expanded: bool,
    is_focused: bool,
    columns: &[String],
) {
    let count = app.volumes.len();
    let title = format!("Volumes ({})", count);

    if !is_expanded {
        frame.render_widget(ratatui::widgets::Clear, area);
        let line = collapsed_panel_line(&title, PanelId::Volumes.number(), is_focused);
        frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
        return;
    }

    let selected = app.panel_selection(PanelId::Volumes);
    let col_offset = app.panel_column_offset(PanelId::Volumes);
    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_table_header(visible_cols);

    let rows: Vec<Row> = app
        .volumes
        .iter()
        .enumerate()
        .map(|(i, vol)| {
            let style = selected_row_style(is_focused, i == selected);

            let cells: Vec<Cell> = visible_cols
                .iter()
                .map(|col| match col.as_str() {
                    "Name" => Cell::from(format!(" {}", vol.name))
                        .style(Style::default().fg(Color::White)),
                    "Driver" => {
                        Cell::from(vol.driver.clone()).style(Style::default().fg(Color::Yellow))
                    }
                    "Mountpoint" => Cell::from(truncate_str(&vol.mountpoint, 40))
                        .style(Style::default().fg(Color::DarkGray)),
                    "Created" => Cell::from(vol.created_at.clone())
                        .style(Style::default().fg(Color::DarkGray)),
                    _ => Cell::from(""),
                })
                .collect();

            Row::new(cells).style(style)
        })
        .collect();

    let col_widths: Vec<Constraint> = visible_cols
        .iter()
        .map(|col| match col.as_str() {
            "Name" => Constraint::Percentage(35),
            "Driver" => Constraint::Length(10),
            "Mountpoint" => Constraint::Percentage(40),
            "Created" => Constraint::Percentage(18),
            _ => Constraint::Percentage((100 / visible_cols.len() as u16).max(1)),
        })
        .collect();

    let scroll_hint = format_scroll_hint(start, max_visible, columns.len());

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(panel_block(
            &format!("Volumes ({}){}", count, scroll_hint),
            PanelId::Volumes.number(),
            is_focused,
        ))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(table, area);
}
