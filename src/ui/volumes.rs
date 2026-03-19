use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style, Stylize},
    widgets::{Cell, Row, Table},
    Frame,
    layout::Rect,
};

use crate::app::App;
use crate::ui::panel_block;

/// Renders the volume list as a table in the left panel.
pub fn draw_volume_list(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let header = Row::new(vec![
        Cell::from(" Name").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Driver").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Mountpoint").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .volumes
        .iter()
        .enumerate()
        .map(|(i, vol)| {
            let style = if i == app.selected_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!(" {}", vol.name)).style(Style::default().fg(Color::White)),
                Cell::from(vol.driver.clone()).style(Style::default().fg(Color::Yellow)),
                Cell::from(truncate_path(&vol.mountpoint, 50))
                    .style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let title = format!("Volumes ({})", app.volumes.len());
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(15),
            Constraint::Percentage(50),
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

/// Truncates a filesystem path to `max_len` characters, replacing the middle
/// with an ellipsis when necessary.
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    let keep = (max_len - 3) / 2;
    format!("{}...{}", &path[..keep], &path[path.len() - keep..])
}
