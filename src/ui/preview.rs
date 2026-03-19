use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::App;
use crate::config::LayoutConfig;
use crate::ui::{
    container_display_name, draw_placeholder, format_bytes, format_ports, format_timestamp_age,
    logs, panel_block, stats,
};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Section header line for preview panels — bold cyan, like " [Info]".
fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" [{}]", title),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Key-value line for preview detail — left-aligned key, then value.
fn kv_line(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<12} ", key),
            Style::default().fg(Color::DarkGray).bold(),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

/// Returns the first 12 characters of an ID string.
fn short_id(id: &str) -> String {
    let stripped = id.strip_prefix("sha256:").unwrap_or(id);
    stripped.chars().take(12).collect()
}

// ─── Status preview ──────────────────────────────────────────────────────────

/// Right-side preview for the Status panel: shows a table of running containers.
pub fn draw_status_preview(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let running: Vec<_> = app
        .containers
        .iter()
        .filter(|c| c.state == "running")
        .collect();

    if running.is_empty() {
        draw_placeholder(frame, area, "No running containers");
        return;
    }

    let header = Row::new(vec![
        Cell::from(" Name").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Image").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Ports").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = running
        .iter()
        .map(|c| {
            Row::new(vec![
                Cell::from(format!(" {}", container_display_name(&c.names)))
                    .style(Style::default().fg(Color::White)),
                Cell::from(c.image.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(c.status.clone()).style(Style::default().fg(Color::Green)),
                Cell::from(format_ports(&c.ports)).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let title = format!("Overview — Running ({})", running.len());
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
    .block(panel_block(&title, is_focused));

    frame.render_widget(table, area);
}

// ─── Container preview ───────────────────────────────────────────────────────

/// Right-side preview for the Containers panel.
///
/// If `detail_container` is loaded, shows a vertical split with detail on top
/// and logs + stats side-by-side on the bottom. Otherwise shows a summary of
/// the selected container from the list.
pub fn draw_container_preview(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    is_focused: bool,
    layout: &LayoutConfig,
) {
    if let Some(detail) = &app.detail_container {
        let has_bottom = !app.log_lines.is_empty() || app.stats.is_some();

        let vert_chunks = if has_bottom {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(layout.preview_split_percent),
                    Constraint::Percentage(100 - layout.preview_split_percent),
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)])
                .split(area)
        };

        // Top: container detail
        draw_container_detail(frame, vert_chunks[0], detail, app, is_focused);

        // Bottom: logs + stats side by side
        if has_bottom {
            let bottom = vert_chunks[1];
            let has_logs = !app.log_lines.is_empty();
            let has_stats = app.stats.is_some();

            if has_logs && has_stats {
                let bottom_split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                    .split(bottom);
                logs::draw_logs(bottom_split[0], frame, &app.log_lines);
                if let Some(ref s) = app.stats {
                    stats::draw_stats(bottom_split[1], frame, s);
                }
            } else if has_logs {
                logs::draw_logs(bottom, frame, &app.log_lines);
            } else if let Some(ref s) = app.stats {
                stats::draw_stats(bottom, frame, s);
            }
        }
    } else {
        // No detail loaded — show summary from the list
        draw_container_summary(frame, area, app, is_focused);
    }
}

/// Renders container detail info (name, ID, image, state, env, labels).
fn draw_container_detail(
    frame: &mut Frame,
    area: Rect,
    detail: &crate::runtime::ContainerDetail,
    app: &App,
    is_focused: bool,
) {
    let block = panel_block("Container Detail", is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Basic info
    lines.push(section_header("Info"));
    lines.push(kv_line("Name", &detail.name));
    lines.push(kv_line("ID", &short_id(&detail.id)));
    lines.push(kv_line("Image", &detail.image));
    lines.push(kv_line("State", &detail.state.status));
    lines.push(kv_line("PID", &detail.state.pid.to_string()));
    lines.push(kv_line("Created", &detail.created));
    lines.push(Line::default());

    // Command
    let full_cmd = if detail.args.is_empty() {
        detail.path.clone()
    } else {
        format!("{} {}", detail.path, detail.args.join(" "))
    };
    lines.push(section_header("Command"));
    lines.push(Line::from(Span::styled(
        format!("  {}", full_cmd),
        Style::default().fg(Color::White),
    )));
    lines.push(Line::default());

    // State details
    lines.push(section_header("State"));
    lines.push(kv_line(
        "Running",
        if detail.state.running { "yes" } else { "no" },
    ));
    lines.push(kv_line(
        "Paused",
        if detail.state.paused { "yes" } else { "no" },
    ));
    lines.push(kv_line("ExitCode", &detail.state.exit_code.to_string()));
    lines.push(kv_line("StartedAt", &detail.state.started_at));
    if !detail.state.finished_at.is_empty()
        && detail.state.finished_at != "0001-01-01T00:00:00Z"
    {
        lines.push(kv_line("FinishedAt", &detail.state.finished_at));
    }
    lines.push(Line::default());

    // Environment variables
    if !detail.env.is_empty() {
        lines.push(section_header("Environment"));
        for env in &detail.env {
            lines.push(Line::from(Span::styled(
                format!("  {}", env),
                Style::default().fg(Color::White),
            )));
        }
        lines.push(Line::default());
    }

    // Labels
    if !detail.labels.is_empty() {
        lines.push(section_header("Labels"));
        for (k, v) in &detail.labels {
            lines.push(Line::from(vec![
                Span::styled(format!("  {}", k), Style::default().fg(Color::Yellow)),
                Span::styled("=", Style::default().fg(Color::DarkGray)),
                Span::styled(v.to_string(), Style::default().fg(Color::White)),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines)
        .scroll((app.preview_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Renders a brief summary for the selected container (when no detail is loaded).
fn draw_container_summary(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let block = panel_block("Container Preview", is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = if let Some(c) = app.containers.get(app.selected_index) {
        vec![
            Line::default(),
            kv_line("Name", &container_display_name(&c.names)),
            kv_line("Image", &c.image),
            kv_line("State", &c.state),
            kv_line("Status", &c.status),
            kv_line("Ports", &format_ports(&c.ports)),
        ]
    } else {
        vec![Line::from(Span::styled(
            " No items",
            Style::default().fg(Color::DarkGray).italic(),
        ))]
    };

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

// ─── Image preview ───────────────────────────────────────────────────────────

/// Right-side preview for the Images panel: shows details for the selected image.
pub fn draw_image_preview(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let block = panel_block("Image Preview", is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = if let Some(img) = app.images.get(app.selected_index) {
        let repo_tags = if img.repo_tags.is_empty() {
            "<none>:<none>".to_string()
        } else {
            img.repo_tags.join(", ")
        };

        let mut l = vec![
            Line::default(),
            section_header("Image"),
            kv_line("Tags", &repo_tags),
            kv_line("ID", &short_id(&img.id)),
            kv_line("Size", &format_bytes(img.size)),
            kv_line("Created", &format_timestamp_age(img.created)),
            Line::default(),
        ];

        if !img.repo_digests.is_empty() {
            l.push(section_header("Digests"));
            for digest in &img.repo_digests {
                l.push(Line::from(Span::styled(
                    format!("  {}", digest),
                    Style::default().fg(Color::White),
                )));
            }
            l.push(Line::default());
        }

        if !img.labels.is_empty() {
            l.push(section_header("Labels"));
            for (k, v) in &img.labels {
                l.push(Line::from(vec![
                    Span::styled(format!("  {}", k), Style::default().fg(Color::Yellow)),
                    Span::styled("=", Style::default().fg(Color::DarkGray)),
                    Span::styled(v.to_string(), Style::default().fg(Color::White)),
                ]));
            }
        }

        l
    } else {
        vec![Line::from(Span::styled(
            " No items",
            Style::default().fg(Color::DarkGray).italic(),
        ))]
    };

    let paragraph = Paragraph::new(lines)
        .scroll((app.preview_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ─── Volume preview ──────────────────────────────────────────────────────────

/// Right-side preview for the Volumes panel: shows details for the selected volume.
pub fn draw_volume_preview(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let block = panel_block("Volume Preview", is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = if let Some(vol) = app.volumes.get(app.selected_index) {
        vec![
            Line::default(),
            section_header("Volume"),
            kv_line("Name", &vol.name),
            kv_line("Driver", &vol.driver),
            kv_line("Mountpoint", &vol.mountpoint),
            kv_line("Created", &vol.created_at),
        ]
    } else {
        vec![Line::from(Span::styled(
            " No items",
            Style::default().fg(Color::DarkGray).italic(),
        ))]
    };

    let paragraph = Paragraph::new(lines)
        .scroll((app.preview_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ─── Network preview ─────────────────────────────────────────────────────────

/// Right-side preview for the Networks panel: shows details for the selected network.
pub fn draw_network_preview(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let block = panel_block("Network Preview", is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = if let Some(net) = app.networks.get(app.selected_index) {
        let mut l = vec![
            Line::default(),
            section_header("Network"),
            kv_line("Name", &net.name),
            kv_line("ID", &short_id(&net.id)),
            kv_line("Driver", &net.driver),
            kv_line("Scope", &net.scope),
            Line::default(),
        ];

        if !net.containers.is_empty() {
            l.push(section_header("Connected Containers"));
            for (cid, cname) in &net.containers {
                l.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", short_id(cid)),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(cname.to_string(), Style::default().fg(Color::White)),
                ]));
            }
        }

        l
    } else {
        vec![Line::from(Span::styled(
            " No items",
            Style::default().fg(Color::DarkGray).italic(),
        ))]
    };

    let paragraph = Paragraph::new(lines)
        .scroll((app.preview_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ─── Pod preview ─────────────────────────────────────────────────────────────

/// Right-side preview for the Pods panel: shows details for the selected pod.
pub fn draw_pod_preview(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    if !app.supports_pods {
        draw_placeholder(frame, area, "Pods only available with Podman");
        return;
    }

    let block = panel_block("Pod Preview", is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = if let Some(pod) = app.pods.get(app.selected_index) {
        let mut l = vec![
            Line::default(),
            section_header("Pod"),
            kv_line("Name", &pod.name),
            kv_line("ID", &short_id(&pod.id)),
            kv_line("Status", &pod.status),
            kv_line("Created", &pod.created),
            Line::default(),
        ];

        if !pod.containers.is_empty() {
            l.push(section_header("Containers"));
            for cr in &pod.containers {
                let status_color = match cr.status.as_str() {
                    "running" | "Running" => Color::Green,
                    "exited" | "Exited" => Color::Red,
                    _ => Color::Yellow,
                };
                l.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", short_id(&cr.id)),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(cr.name.clone(), Style::default().fg(Color::White)),
                    Span::raw(" "),
                    Span::styled(cr.status.clone(), Style::default().fg(status_color)),
                ]));
            }
        }

        l
    } else {
        vec![Line::from(Span::styled(
            " No items",
            Style::default().fg(Color::DarkGray).italic(),
        ))]
    };

    let paragraph = Paragraph::new(lines)
        .scroll((app.preview_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}
