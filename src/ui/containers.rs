use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Table},
};

use crate::app::{App, PanelId};
use crate::ui::{
    container_display_name, create_table_header, format_ports, format_scroll_hint, panel_block,
    percentage_color, selected_row_style, short_id, visible_column_count,
};

/// Renders the Containers panel (panel 4, right column).
///
/// Columns are configurable via `config.toml` and scrollable with Left/Right.
pub fn draw_container_panel(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    is_expanded: bool,
    is_focused: bool,
    columns: &[String],
) {
    let count = app.containers.len();
    let title = format!("Containers ({})", count);

    if !is_expanded {
        let block = panel_block(&title, PanelId::Containers.number(), is_focused);
        frame.render_widget(block, area);
        return;
    }

    let selected = app.panel_selection(PanelId::Containers);
    let col_offset = app.panel_column_offset(PanelId::Containers);

    // Determine visible columns based on area width
    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    // Header
    let header = create_table_header(visible_cols);

    // Rows
    let rows: Vec<Row> = app
        .containers
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let sc = state_color(&c.state);
            let style = selected_row_style(is_focused, i == selected);

            let cells: Vec<Cell> = visible_cols
                .iter()
                .map(|col| match col.as_str() {
                    "Name" => Cell::from(format!(" {}", container_display_name(&c.names)))
                        .style(Style::default().fg(Color::White)),
                    "Image" => Cell::from(c.image.clone()).style(Style::default().fg(Color::Gray)),
                    "Status" => Cell::from(c.status.clone()).style(Style::default().fg(sc)),
                    "State" => Cell::from(c.state.clone()).style(Style::default().fg(sc)),
                    "Ports" => Cell::from(format_ports(&c.ports))
                        .style(Style::default().fg(Color::DarkGray)),
                    "CPU" => {
                        let pct = app.container_stats.get(&c.id).map(|(cpu, _)| *cpu);
                        match pct {
                            Some(p) => Cell::from(format!("{:.1}%", p))
                                .style(Style::default().fg(percentage_color(p))),
                            None => Cell::from("--").style(Style::default().fg(Color::DarkGray)),
                        }
                    }
                    "Memory" => {
                        let mem = app.container_stats.get(&c.id).map(|(_, m)| *m);
                        match mem {
                            Some(m) => Cell::from(crate::ui::format_bytes(m))
                                .style(Style::default().fg(Color::Cyan)),
                            None => Cell::from("--").style(Style::default().fg(Color::DarkGray)),
                        }
                    }
                    "ID" => Cell::from(short_id(&c.id)).style(Style::default().fg(Color::DarkGray)),
                    "Command" => {
                        Cell::from(c.command.clone()).style(Style::default().fg(Color::DarkGray))
                    }
                    _ => Cell::from(""),
                })
                .collect();

            Row::new(cells).style(style)
        })
        .collect();

    // Smart column widths based on content type
    let col_widths: Vec<Constraint> = visible_cols
        .iter()
        .map(|col| match col.as_str() {
            "Name" => Constraint::Percentage(22),
            "Image" => Constraint::Percentage(20),
            "Status" => Constraint::Percentage(18),
            "State" => Constraint::Length(10),
            "Ports" => Constraint::Percentage(16),
            "CPU" => Constraint::Length(8),
            "Memory" => Constraint::Length(10),
            "ID" => Constraint::Length(14),
            "Command" => Constraint::Percentage(20),
            _ => Constraint::Percentage((100 / visible_cols.len() as u16).max(1)),
        })
        .collect();

    // Scroll indicator in title
    let scroll_hint = format_scroll_hint(start, max_visible, columns.len());

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(panel_block(
            &format!("Containers ({}){}", count, scroll_hint),
            PanelId::Containers.number(),
            is_focused,
        ))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(table, area);
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
