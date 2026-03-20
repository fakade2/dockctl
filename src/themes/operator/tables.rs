use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
};

use crate::app::{App, PanelId};
use crate::config::DockctlConfig;
use crate::display::{
    container_display_name, format_bytes, format_ports, format_scroll_hint, format_timestamp_age,
    percentage_color, selected_row_style, short_id, truncate_str, visible_column_count,
};

use super::helpers::{
    container_status_dot, create_sorted_header, image_status_dot, k9s_table_title,
    network_status_dot, state_color,
};
use super::status_view::draw_status_view;

pub(super) fn draw_resource_table(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    config: &DockctlConfig,
) {
    match app.operator_resource.as_str() {
        "containers" => {
            let cols = if app.wide_mode {
                &config.columns.containers_wide
            } else {
                &config.columns.containers
            };
            draw_containers_table(frame, area, app, cols);
        }
        "images" => {
            let cols = if app.wide_mode {
                &config.columns.images_wide
            } else {
                &config.columns.images
            };
            draw_images_table(frame, area, app, cols);
        }
        "volumes" => {
            let cols = if app.wide_mode {
                &config.columns.volumes_wide
            } else {
                &config.columns.volumes
            };
            draw_volumes_table(frame, area, app, cols);
        }
        "networks" => {
            let cols = if app.wide_mode {
                &config.columns.networks_wide
            } else {
                &config.columns.networks
            };
            draw_networks_table(frame, area, app, cols);
        }
        "compose" => draw_compose_table(frame, area, app),
        "status" => draw_status_view(frame, area, app),
        _ => {
            let cols = if app.wide_mode {
                &config.columns.containers_wide
            } else {
                &config.columns.containers
            };
            draw_containers_table(frame, area, app, cols);
        }
    }
}

pub(super) fn draw_containers_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
    let selected = app.panel_selection(PanelId::Containers);
    let col_offset = app.panel_column_offset(PanelId::Containers);
    let filter = &app.operator_filter;

    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_sorted_header(visible_cols, app.sort_column.as_deref(), app.sort_ascending);

    let filtered: Vec<(usize, &crate::runtime::ContainerSummary)> = app
        .containers
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            if filter.is_empty() {
                return true;
            }
            let f = filter.to_lowercase();
            container_display_name(&c.names).to_lowercase().contains(&f)
                || c.image.to_lowercase().contains(&f)
                || c.state.to_lowercase().contains(&f)
        })
        .collect();

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(display_idx, (_, c))| {
            let sc = state_color(&c.state);
            let style = selected_row_style(true, display_idx == selected);
            let (dot, dot_color) = container_status_dot(&c.state);

            let cells: Vec<Cell> = visible_cols
                .iter()
                .map(|col| match col.as_str() {
                    "Name" => {
                        let name = container_display_name(&c.names);
                        Cell::from(Line::from(vec![
                            Span::styled(format!(" {} ", dot), Style::default().fg(dot_color)),
                            Span::styled(name, Style::default().fg(Color::White)),
                        ]))
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

            Row::new(cells).style(style)
        })
        .collect();

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
    let title = k9s_table_title(
        "containers",
        app.containers.len(),
        filtered.len(),
        filter,
        &scroll_hint,
    );

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .title_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(table, area);
}

pub(super) fn draw_images_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
    let selected = app.panel_selection(PanelId::Images);
    let col_offset = app.panel_column_offset(PanelId::Images);
    let filter = &app.operator_filter;

    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_sorted_header(visible_cols, app.sort_column.as_deref(), app.sort_ascending);

    let filtered: Vec<(usize, &crate::runtime::ImageSummary)> = app
        .images
        .iter()
        .enumerate()
        .filter(|(_, img)| {
            if filter.is_empty() {
                return true;
            }
            let f = filter.to_lowercase();
            img.repo_tags.iter().any(|t| t.to_lowercase().contains(&f))
                || short_id(&img.id).contains(&f)
        })
        .collect();

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(display_idx, (_, img))| {
            let style = selected_row_style(true, display_idx == selected);
            let (dot, dot_color) = image_status_dot(&img.repo_tags);

            let cells: Vec<Cell> =
                visible_cols
                    .iter()
                    .map(|col| match col.as_str() {
                        "Repository" => {
                            let tag = img
                                .repo_tags
                                .first()
                                .map(|t| t.split(':').next().unwrap_or(t).to_string())
                                .unwrap_or_else(|| "<none>".into());
                            Cell::from(Line::from(vec![
                                Span::styled(format!(" {} ", dot), Style::default().fg(dot_color)),
                                Span::styled(tag, Style::default().fg(Color::White)),
                            ]))
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
    let title = k9s_table_title(
        "images",
        app.images.len(),
        filtered.len(),
        filter,
        &scroll_hint,
    );

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .title_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(table, area);
}

pub(super) fn draw_volumes_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
    let selected = app.panel_selection(PanelId::Volumes);
    let col_offset = app.panel_column_offset(PanelId::Volumes);
    let filter = &app.operator_filter;

    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_sorted_header(visible_cols, app.sort_column.as_deref(), app.sort_ascending);

    let filtered: Vec<(usize, &crate::runtime::VolumeSummary)> = app
        .volumes
        .iter()
        .enumerate()
        .filter(|(_, vol)| {
            if filter.is_empty() {
                return true;
            }
            let f = filter.to_lowercase();
            vol.name.to_lowercase().contains(&f) || vol.driver.to_lowercase().contains(&f)
        })
        .collect();

    // Volumes: all get a blue dot
    let vol_dot = "\u{25cf}";
    let vol_dot_color = Color::Blue;

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(display_idx, (_, vol))| {
            let style = selected_row_style(true, display_idx == selected);

            let cells: Vec<Cell> = visible_cols
                .iter()
                .map(|col| match col.as_str() {
                    "Name" => Cell::from(Line::from(vec![
                        Span::styled(format!(" {} ", vol_dot), Style::default().fg(vol_dot_color)),
                        Span::styled(vol.name.clone(), Style::default().fg(Color::White)),
                    ])),
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
    let title = k9s_table_title(
        "volumes",
        app.volumes.len(),
        filtered.len(),
        filter,
        &scroll_hint,
    );

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .title_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(table, area);
}

pub(super) fn draw_compose_table(frame: &mut Frame, area: Rect, app: &App) {
    let selected = app.panel_selection(PanelId::Containers);

    match &app.compose_selected_project {
        None => {
            // Project list view
            let columns = ["Project", "Services", "Running", "Status", "Working Dir"];
            let header = Row::new(
                columns
                    .iter()
                    .map(|c| {
                        Cell::from(format!(" {}", c)).style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                    })
                    .collect::<Vec<Cell>>(),
            )
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

            let rows: Vec<Row> = app
                .compose_projects
                .iter()
                .enumerate()
                .map(|(idx, project)| {
                    let style = selected_row_style(true, idx == selected);
                    let status_str = if project.running_count == project.total_count {
                        "running"
                    } else if project.running_count > 0 {
                        "partial"
                    } else {
                        "stopped"
                    };
                    let (dot, dot_color) = if project.running_count == project.total_count {
                        ("\u{25cf}", Color::Green)
                    } else if project.running_count > 0 {
                        ("\u{25cf}", Color::Yellow)
                    } else {
                        ("\u{25cf}", Color::Red)
                    };
                    let status_color = match status_str {
                        "running" => Color::Green,
                        "partial" => Color::Yellow,
                        _ => Color::Red,
                    };

                    Row::new(vec![
                        Cell::from(Line::from(vec![
                            Span::styled(format!(" {} ", dot), Style::default().fg(dot_color)),
                            Span::styled(project.name.clone(), Style::default().fg(Color::White)),
                        ])),
                        Cell::from(format!("{}", project.total_count))
                            .style(Style::default().fg(Color::Gray)),
                        Cell::from(format!("{}/{}", project.running_count, project.total_count))
                            .style(Style::default().fg(Color::Green)),
                        Cell::from(status_str.to_string()).style(Style::default().fg(status_color)),
                        Cell::from(truncate_str(&project.working_dir, 40))
                            .style(Style::default().fg(Color::DarkGray)),
                    ])
                    .style(style)
                })
                .collect();

            let col_widths = vec![
                Constraint::Percentage(25),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Percentage(35),
            ];

            let title = format!(" Compose({}) ", app.compose_projects.len());

            let table = Table::new(rows, col_widths)
                .header(header)
                .block(
                    Block::default()
                        .title(title)
                        .title_style(
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .row_highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );

            frame.render_widget(table, area);
        }
        Some(project_name) => {
            // Service list view for a specific project
            let project = app
                .compose_projects
                .iter()
                .find(|p| &p.name == project_name);
            let services: &[crate::app::ComposeService] = match project {
                Some(p) => &p.services,
                None => &[],
            };

            let columns_svc = ["Service", "Image", "State", "Status", "Ports"];
            let header = Row::new(
                columns_svc
                    .iter()
                    .map(|c| {
                        Cell::from(format!(" {}", c)).style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                    })
                    .collect::<Vec<Cell>>(),
            )
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

            let rows: Vec<Row> = services
                .iter()
                .enumerate()
                .map(|(idx, svc)| {
                    let style = selected_row_style(true, idx == selected);
                    let (dot, dot_color) = container_status_dot(&svc.state);
                    let sc = state_color(&svc.state);

                    Row::new(vec![
                        Cell::from(Line::from(vec![
                            Span::styled(format!(" {} ", dot), Style::default().fg(dot_color)),
                            Span::styled(svc.name.clone(), Style::default().fg(Color::White)),
                        ])),
                        Cell::from(svc.image.clone()).style(Style::default().fg(Color::Gray)),
                        Cell::from(svc.state.clone()).style(Style::default().fg(sc)),
                        Cell::from(svc.status.clone()).style(Style::default().fg(sc)),
                        Cell::from(format_ports(&svc.ports))
                            .style(Style::default().fg(Color::DarkGray)),
                    ])
                    .style(style)
                })
                .collect();

            let col_widths = vec![
                Constraint::Percentage(20),
                Constraint::Percentage(25),
                Constraint::Length(10),
                Constraint::Percentage(18),
                Constraint::Percentage(18),
            ];

            let title = format!(" Compose > {}({}) ", project_name, services.len());

            let table = Table::new(rows, col_widths)
                .header(header)
                .block(
                    Block::default()
                        .title(title)
                        .title_style(
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .row_highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );

            frame.render_widget(table, area);
        }
    }
}

pub(super) fn draw_networks_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
    let selected = app.panel_selection(PanelId::Networks);
    let col_offset = app.panel_column_offset(PanelId::Networks);
    let filter = &app.operator_filter;

    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_sorted_header(visible_cols, app.sort_column.as_deref(), app.sort_ascending);

    let filtered: Vec<(usize, &crate::runtime::NetworkSummary)> = app
        .networks
        .iter()
        .enumerate()
        .filter(|(_, net)| {
            if filter.is_empty() {
                return true;
            }
            let f = filter.to_lowercase();
            net.name.to_lowercase().contains(&f) || net.driver.to_lowercase().contains(&f)
        })
        .collect();

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(display_idx, (_, net))| {
            let style = selected_row_style(true, display_idx == selected);
            let (dot, dot_color) = network_status_dot(net.containers.len());

            let cells: Vec<Cell> = visible_cols
                .iter()
                .map(|col| match col.as_str() {
                    "Name" => Cell::from(Line::from(vec![
                        Span::styled(format!(" {} ", dot), Style::default().fg(dot_color)),
                        Span::styled(net.name.clone(), Style::default().fg(Color::White)),
                    ])),
                    "ID" => {
                        Cell::from(short_id(&net.id)).style(Style::default().fg(Color::DarkGray))
                    }
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
                    "Scope" => {
                        Cell::from(net.scope.clone()).style(Style::default().fg(Color::DarkGray))
                    }
                    "Containers" => {
                        let count = net.containers.len();
                        Cell::from(format!("{}", count)).style(Style::default().fg(if count > 0 {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }))
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
    let title = k9s_table_title(
        "networks",
        app.networks.len(),
        filtered.len(),
        filter,
        &scroll_hint,
    );

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .title_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(table, area);
}
