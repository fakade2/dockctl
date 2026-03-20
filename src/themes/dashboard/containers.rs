use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Paragraph, Row, Table},
};

use crate::app::{App, PanelId};
use crate::display::{
    collapsed_panel_line, container_display_name, create_table_header, format_bytes, format_ports,
    format_scroll_hint, panel_block, percentage_color, selected_row_style, short_id,
    visible_column_count,
};

/// Renders the Containers panel (panel 4, right column).
///
/// Groups containers by compose project. Compose projects show as group headers,
/// standalone containers appear under "standalone".
pub fn draw_container_panel(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    is_expanded: bool,
    is_focused: bool,
    columns: &[String],
) {
    let count = app.containers.len();
    let compose_count = app.compose_projects.len();
    let title = if compose_count > 0 {
        format!("Containers ({}) [{} projects]", count, compose_count)
    } else {
        format!("Containers ({})", count)
    };

    if !is_expanded {
        frame.render_widget(ratatui::widgets::Clear, area);
        let line = collapsed_panel_line(&title, PanelId::Containers.number(), is_focused);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    // Build grouped rows: compose projects first, then standalone
    let selected = app.panel_selection(PanelId::Containers);
    let col_offset = app.panel_column_offset(PanelId::Containers);
    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_table_header(visible_cols);

    // Collect containers grouped by project
    let mut rows: Vec<Row> = Vec::new();
    let mut flat_index: usize = 0;

    // First: compose project groups
    for project in &app.compose_projects {
        let status_color = if project.running_count == project.total_count {
            Color::Green
        } else if project.running_count > 0 {
            Color::Yellow
        } else {
            Color::Red
        };

        // Group header row — spans all columns
        let group_label = format!(
            " ▼ {} ({}/{} running)",
            project.name, project.running_count, project.total_count
        );
        let header_cells: Vec<Cell> = std::iter::once(
            Cell::from(group_label).style(
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .chain(visible_cols.iter().skip(1).map(|_| Cell::from("")))
        .collect();
        rows.push(Row::new(header_cells));

        // Service rows (indented)
        for service in &project.services {
            let container = app.containers.iter().find(|c| c.id == service.container_id);

            let style = selected_row_style(is_focused, flat_index == selected);
            let sc = state_color(&service.state);

            let cells: Vec<Cell> = visible_cols
                .iter()
                .map(|col| match col.as_str() {
                    "Name" => {
                        let dot = if service.state == "running" {
                            "●"
                        } else {
                            "○"
                        };
                        Cell::from(format!("   {} {}", dot, service.name))
                            .style(Style::default().fg(Color::White))
                    }
                    "Image" => {
                        Cell::from(service.image.clone()).style(Style::default().fg(Color::Gray))
                    }
                    "Status" => Cell::from(service.status.clone()).style(Style::default().fg(sc)),
                    "State" => Cell::from(service.state.clone()).style(Style::default().fg(sc)),
                    "Ports" => Cell::from(format_ports(&service.ports))
                        .style(Style::default().fg(Color::DarkGray)),
                    "CPU" => {
                        let pct = app
                            .container_stats
                            .get(&service.container_id)
                            .map(|(cpu, _)| *cpu);
                        match pct {
                            Some(p) => Cell::from(format!("{:.1}%", p))
                                .style(Style::default().fg(percentage_color(p))),
                            None => Cell::from("--").style(Style::default().fg(Color::DarkGray)),
                        }
                    }
                    "Memory" => {
                        let mem = app
                            .container_stats
                            .get(&service.container_id)
                            .map(|(_, m)| *m);
                        match mem {
                            Some(m) => {
                                Cell::from(format_bytes(m)).style(Style::default().fg(Color::Cyan))
                            }
                            None => Cell::from("--").style(Style::default().fg(Color::DarkGray)),
                        }
                    }
                    "ID" => Cell::from(short_id(&service.container_id))
                        .style(Style::default().fg(Color::DarkGray)),
                    "Command" => {
                        let cmd = container.map(|c| c.command.as_str()).unwrap_or("--");
                        Cell::from(cmd.to_string()).style(Style::default().fg(Color::DarkGray))
                    }
                    _ => Cell::from(""),
                })
                .collect();

            rows.push(Row::new(cells).style(style));
            flat_index += 1;
        }
    }

    // Then: standalone containers (not in any compose project)
    let compose_ids: std::collections::HashSet<&str> = app
        .compose_projects
        .iter()
        .flat_map(|p| p.services.iter().map(|s| s.container_id.as_str()))
        .collect();

    let standalone: Vec<&crate::runtime::ContainerSummary> = app
        .containers
        .iter()
        .filter(|c| !compose_ids.contains(c.id.as_str()))
        .collect();

    if !standalone.is_empty() {
        // Standalone group header
        let header_cells: Vec<Cell> = std::iter::once(
            Cell::from(format!(" ▼ standalone ({})", standalone.len())).style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .chain(visible_cols.iter().skip(1).map(|_| Cell::from("")))
        .collect();
        rows.push(Row::new(header_cells));

        for c in &standalone {
            let sc = state_color(&c.state);
            let style = selected_row_style(is_focused, flat_index == selected);
            let dot = if c.state == "running" { "●" } else { "○" };

            let cells: Vec<Cell> = visible_cols
                .iter()
                .map(|col| match col.as_str() {
                    "Name" => {
                        Cell::from(format!("   {} {}", dot, container_display_name(&c.names)))
                            .style(Style::default().fg(Color::White))
                    }
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
                            Some(m) => {
                                Cell::from(format_bytes(m)).style(Style::default().fg(Color::Cyan))
                            }
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

            rows.push(Row::new(cells).style(style));
            flat_index += 1;
        }
    }

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

    let scroll_hint = format_scroll_hint(start, max_visible, columns.len());

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(panel_block(
            &format!("Containers ({}){}", count, scroll_hint),
            PanelId::Containers.number(),
            is_focused,
        ))
        .column_spacing(1)
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
