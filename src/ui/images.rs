use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style, Stylize},
    widgets::{Cell, Row, Table},
    Frame,
    layout::Rect,
};

use crate::app::App;
use crate::ui::{format_bytes, format_timestamp_age, panel_block};

/// Renders the image list as a table in the left panel.
pub fn draw_image_list(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let header = Row::new(vec![
        Cell::from(" Repository:Tag").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("ID").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Size").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Created").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .images
        .iter()
        .enumerate()
        .map(|(i, img)| {
            let repo_tag = if img.repo_tags.is_empty() {
                "<none>:<none>".to_string()
            } else {
                img.repo_tags.join(", ")
            };

            let short_id = short_image_id(&img.id);
            let size = format_bytes(img.size);
            let created = format_timestamp_age(img.created);

            let style = if i == app.selected_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!(" {}", repo_tag)).style(Style::default().fg(Color::White)),
                Cell::from(short_id).style(Style::default().fg(Color::DarkGray)),
                Cell::from(size).style(Style::default().fg(Color::Yellow)),
                Cell::from(created).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let title = format!("Images ({})", app.images.len());
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(18),
            Constraint::Percentage(18),
            Constraint::Percentage(24),
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

/// Returns the first 12 hex characters from a Docker/Podman image ID, stripping
/// any `sha256:` prefix.
fn short_image_id(id: &str) -> String {
    let stripped = id.strip_prefix("sha256:").unwrap_or(id);
    stripped.chars().take(12).collect()
}
