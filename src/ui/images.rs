use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Table},
};

use crate::app::{App, PanelId};
use crate::ui::{
    create_table_header, format_bytes, format_scroll_hint, format_timestamp_age, panel_block,
    selected_row_style, short_id, visible_column_count,
};

/// Renders the Images panel (panel 5, right column).
/// Columns are configurable and scrollable with Left/Right.
pub fn draw_image_panel(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    is_expanded: bool,
    is_focused: bool,
    columns: &[String],
) {
    let count = app.images.len();
    let title = format!("Images ({})", count);

    if !is_expanded {
        let line = crate::ui::collapsed_panel_line(&title, PanelId::Images.number(), is_focused);
        frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
        return;
    }

    let selected = app.panel_selection(PanelId::Images);
    let col_offset = app.panel_column_offset(PanelId::Images);
    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_table_header(visible_cols);

    let rows: Vec<Row> =
        app.images
            .iter()
            .enumerate()
            .map(|(i, img)| {
                let style = selected_row_style(is_focused, i == selected);

                let cells: Vec<Cell> = visible_cols
                    .iter()
                    .map(|col| match col.as_str() {
                        "Repository" => {
                            let tag = img
                                .repo_tags
                                .first()
                                .map(|t| t.split(':').next().unwrap_or(t).to_string())
                                .unwrap_or_else(|| "<none>".into());
                            Cell::from(format!(" {}", tag)).style(Style::default().fg(Color::White))
                        }
                        "Tag" => {
                            let tag = img
                                .repo_tags
                                .first()
                                .and_then(|t| t.split(':').nth(1))
                                .unwrap_or("<none>");
                            Cell::from(tag.to_string()).style(Style::default().fg(Color::DarkGray))
                        }
                        "ID" => Cell::from(short_id(&img.id))
                            .style(Style::default().fg(Color::DarkGray)),
                        "Size" => Cell::from(format_bytes(img.size))
                            .style(Style::default().fg(Color::Yellow)),
                        "Created" => Cell::from(format_timestamp_age(img.created))
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
            "Repository" => Constraint::Percentage(35),
            "Tag" => Constraint::Percentage(15),
            "ID" => Constraint::Length(14),
            "Size" => Constraint::Length(10),
            "Created" => Constraint::Percentage(18),
            _ => Constraint::Percentage((100 / visible_cols.len() as u16).max(1)),
        })
        .collect();

    let scroll_hint = format_scroll_hint(start, max_visible, columns.len());

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(panel_block(
            &format!("Images ({}){}", count, scroll_hint),
            PanelId::Images.number(),
            is_focused,
        ))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(table, area);
}
