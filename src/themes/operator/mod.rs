use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};

use crate::app::{App, PanelId};
use crate::config::DockctlConfig;
use crate::themes::{Theme, ThemeAction};
use crate::ui::{
    container_display_name, create_table_header, format_bytes, format_ports, format_scroll_hint,
    format_timestamp_age, percentage_color, selected_row_style, short_id, truncate_str,
    visible_column_count,
};

pub struct OperatorTheme;

impl Default for OperatorTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl OperatorTheme {
    pub fn new() -> Self {
        Self
    }
}

impl Theme for OperatorTheme {
    fn name(&self) -> &str {
        "operator"
    }

    fn draw(&self, frame: &mut Frame, app: &App, config: &DockctlConfig) {
        draw_operator(frame, app, config);
    }

    fn handle_key(&self, key: KeyEvent, app: &mut App, config: &DockctlConfig) -> ThemeAction {
        operator_handle_key(key, app, config)
    }
}

// ---------------------------------------------------------------------------
// Key handling
// ---------------------------------------------------------------------------

fn operator_handle_key(key: KeyEvent, app: &mut App, config: &DockctlConfig) -> ThemeAction {
    // Command mode — typing a : command
    if app.operator_command_mode {
        return handle_command_mode(key, app);
    }

    // Filter mode — typing a / filter
    if app.operator_filter_active {
        return handle_filter_mode(key, app);
    }

    // Normal mode
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => ThemeAction::Quit,

        // Enter command mode
        (KeyCode::Char(':'), _) => {
            app.operator_command_mode = true;
            app.operator_command_input.clear();
            ThemeAction::Consumed
        }

        // Enter filter mode
        (KeyCode::Char('/'), _) => {
            app.operator_filter_active = true;
            app.operator_filter.clear();
            ThemeAction::Consumed
        }

        // Navigation
        (KeyCode::Down | KeyCode::Char('j'), _) => {
            app.next_item();
            ThemeAction::Consumed
        }
        (KeyCode::Up | KeyCode::Char('k'), _) => {
            app.previous_item();
            ThemeAction::Consumed
        }

        // Column scrolling
        (KeyCode::Right | KeyCode::Char('l'), _) => {
            let max_cols = operator_max_cols(app, config);
            app.scroll_columns_right(max_cols);
            ThemeAction::Consumed
        }
        (KeyCode::Left | KeyCode::Char('h'), _) => {
            app.scroll_columns_left();
            ThemeAction::Consumed
        }

        // Describe — open detail popup for selected item
        (KeyCode::Char('d'), _) => {
            let (title, content) = describe_selected(app);
            ThemeAction::OpenPopup { title, content }
        }

        // Explain / Enter — open detail popup
        (KeyCode::Char('e') | KeyCode::Enter, _) => ThemeAction::NotHandled,

        // Esc — clear filter if active, otherwise not handled
        (KeyCode::Esc, _) => {
            if !app.operator_filter.is_empty() {
                app.operator_filter.clear();
                ThemeAction::Consumed
            } else {
                ThemeAction::Consumed
            }
        }

        // Help
        (KeyCode::Char('?'), _) => ThemeAction::OpenPopup {
            title: "Operator Keybindings".to_string(),
            content: vec![
                "Navigation".to_string(),
                "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
                "j / Down     Move down".to_string(),
                "k / Up       Move up".to_string(),
                "h / Left     Scroll columns left".to_string(),
                "l / Right    Scroll columns right".to_string(),
                String::new(),
                "Commands".to_string(),
                "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
                ":            Command mode".to_string(),
                "             :containers  :images  :volumes  :networks".to_string(),
                "/            Filter mode".to_string(),
                "d            Describe selected item".to_string(),
                "e / Enter    Explain (detail popup)".to_string(),
                "?            This help".to_string(),
                "q            Quit".to_string(),
                String::new(),
                "Container Actions (in :containers view)".to_string(),
                "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
                "s            Stop container".to_string(),
                "r            Restart container".to_string(),
                "x            Remove container".to_string(),
                "l            View logs".to_string(),
                "E            Exec into container (tmux)".to_string(),
            ],
        },

        // Container actions — pass to main loop
        _ => ThemeAction::NotHandled,
    }
}

fn handle_command_mode(key: KeyEvent, app: &mut App) -> ThemeAction {
    match key.code {
        KeyCode::Esc => {
            app.operator_command_mode = false;
            app.operator_command_input.clear();
            ThemeAction::Consumed
        }
        KeyCode::Enter => {
            let cmd = app.operator_command_input.clone();
            app.operator_command_mode = false;
            app.operator_command_input.clear();
            execute_command(app, &cmd);
            ThemeAction::Consumed
        }
        KeyCode::Backspace => {
            app.operator_command_input.pop();
            ThemeAction::Consumed
        }
        KeyCode::Char(c) => {
            app.operator_command_input.push(c);
            ThemeAction::Consumed
        }
        _ => ThemeAction::Consumed,
    }
}

fn handle_filter_mode(key: KeyEvent, app: &mut App) -> ThemeAction {
    match key.code {
        KeyCode::Esc => {
            app.operator_filter_active = false;
            app.operator_filter.clear();
            ThemeAction::Consumed
        }
        KeyCode::Enter => {
            app.operator_filter_active = false;
            // Filter stays applied
            ThemeAction::Consumed
        }
        KeyCode::Backspace => {
            app.operator_filter.pop();
            ThemeAction::Consumed
        }
        KeyCode::Char(c) => {
            app.operator_filter.push(c);
            ThemeAction::Consumed
        }
        _ => ThemeAction::Consumed,
    }
}

/// Execute a : command — switch resource view.
fn execute_command(app: &mut App, cmd: &str) {
    let cmd = cmd.trim().to_lowercase();
    let resource = fuzzy_match_resource(&cmd);
    if let Some(res) = resource {
        app.operator_resource = res.to_string();
        // Sync active_panel so container actions in main.rs work
        match res {
            "containers" => app.focus_panel(PanelId::Containers),
            "images" => app.focus_panel(PanelId::Images),
            "volumes" => app.focus_panel(PanelId::Volumes),
            "networks" => app.focus_panel(PanelId::Networks),
            _ => {}
        }
    }
}

/// Fuzzy match a command input to a resource name.
fn fuzzy_match_resource(input: &str) -> Option<&'static str> {
    let resources = ["containers", "images", "volumes", "networks"];
    // Exact match first
    if let Some(r) = resources.iter().find(|&&r| r == input) {
        return Some(r);
    }
    // Prefix match
    let matches: Vec<&&str> = resources
        .iter()
        .filter(|&&r| r.starts_with(input))
        .collect();
    if matches.len() == 1 {
        return Some(matches[0]);
    }
    // Common abbreviations
    match input {
        "con" | "cnt" | "c" => Some("containers"),
        "img" | "im" | "i" => Some("images"),
        "vol" | "v" => Some("volumes"),
        "net" | "n" => Some("networks"),
        _ => None,
    }
}

fn operator_max_cols(app: &App, config: &DockctlConfig) -> usize {
    match app.operator_resource.as_str() {
        "containers" => config.columns.containers.len(),
        "images" => config.columns.images.len(),
        "volumes" => config.columns.volumes.len(),
        "networks" => config.columns.networks.len(),
        _ => 0,
    }
}

/// Build describe lines for the currently selected item based on resource view.
fn describe_selected(app: &App) -> (String, Vec<String>) {
    match app.operator_resource.as_str() {
        "containers" => {
            let idx = app.panel_selection(PanelId::Containers);
            if let Some(c) = app.containers.get(idx) {
                let title = format!("Container: {}", container_display_name(&c.names));
                let lines = vec![
                    format!("Name:    {}", container_display_name(&c.names)),
                    format!("Image:   {}", c.image),
                    format!("State:   {}", c.state),
                    format!("Status:  {}", c.status),
                    format!("ID:      {}", short_id(&c.id)),
                    format!("Ports:   {}", format_ports(&c.ports)),
                    format!("Command: {}", c.command),
                ];
                (title, lines)
            } else {
                (
                    "No Selection".to_string(),
                    vec!["No container selected".to_string()],
                )
            }
        }
        "images" => {
            let idx = app.panel_selection(PanelId::Images);
            if let Some(img) = app.images.get(idx) {
                let tag = img
                    .repo_tags
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "<none>:<none>".to_string());
                let title = format!("Image: {}", tag);
                let lines = vec![
                    format!("Tags:    {}", img.repo_tags.join(", ")),
                    format!("ID:      {}", short_id(&img.id)),
                    format!("Size:    {}", format_bytes(img.size)),
                    format!("Created: {}", format_timestamp_age(img.created)),
                ];
                (title, lines)
            } else {
                (
                    "No Selection".to_string(),
                    vec!["No image selected".to_string()],
                )
            }
        }
        "volumes" => {
            let idx = app.panel_selection(PanelId::Volumes);
            if let Some(vol) = app.volumes.get(idx) {
                let title = format!("Volume: {}", vol.name);
                let lines = vec![
                    format!("Name:       {}", vol.name),
                    format!("Driver:     {}", vol.driver),
                    format!("Mountpoint: {}", vol.mountpoint),
                    format!("Scope:      {}", vol.scope),
                    format!("Created:    {}", vol.created_at),
                ];
                (title, lines)
            } else {
                (
                    "No Selection".to_string(),
                    vec!["No volume selected".to_string()],
                )
            }
        }
        "networks" => {
            let idx = app.panel_selection(PanelId::Networks);
            if let Some(net) = app.networks.get(idx) {
                let title = format!("Network: {}", net.name);
                let mut lines = vec![
                    format!("Name:   {}", net.name),
                    format!("ID:     {}", short_id(&net.id)),
                    format!("Driver: {}", net.driver),
                    format!("Scope:  {}", net.scope),
                ];
                if !net.containers.is_empty() {
                    lines.push(String::new());
                    lines.push("Connected Containers:".to_string());
                    for (cid, cname) in &net.containers {
                        lines.push(format!("  {} {}", short_id(cid), cname));
                    }
                }
                (title, lines)
            } else {
                (
                    "No Selection".to_string(),
                    vec!["No network selected".to_string()],
                )
            }
        }
        _ => ("Unknown".to_string(), vec!["Unknown resource".to_string()]),
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn draw_operator(frame: &mut Frame, app: &App, config: &DockctlConfig) {
    // Layout: header (3) | table (fills) | footer (1)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let header_area = outer[0];
    let table_area = outer[1];
    let footer_area = outer[2];

    draw_header(frame, header_area, app);
    draw_resource_table(frame, table_area, app, config);
    draw_footer(frame, footer_area, app);

    // Popup overlay (if open)
    if app.show_popup {
        draw_popup(frame, app);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let resource = &app.operator_resource;
    let resource_cap = capitalize(resource);

    // Build info strings
    let runtime_info = if let Some(info) = &app.system_info {
        format!(
            "{} v{}  {} cores",
            info.runtime_type, info.server_version, info.cpus
        )
    } else {
        String::new()
    };

    let logo_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let breadcrumb_style = Style::default().fg(Color::White);
    let info_style = Style::default().fg(Color::DarkGray);

    let left_spans = vec![
        Span::styled(" dockctl ", logo_style),
        Span::styled("   ", Style::default()),
        Span::styled(format!("Docker > {}", resource_cap), breadcrumb_style),
    ];

    let left_len: usize = left_spans.iter().map(|s| s.width()).sum();
    let right_len = runtime_info.len() + 1;
    let total = area.width as usize;
    let padding = total.saturating_sub(left_len + right_len);

    let mut spans = left_spans;
    spans.push(Span::raw(" ".repeat(padding)));
    spans.push(Span::styled(format!("{} ", runtime_info), info_style));

    let line = Line::from(spans);
    let header = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, area);
}

fn draw_resource_table(frame: &mut Frame, area: Rect, app: &App, config: &DockctlConfig) {
    match app.operator_resource.as_str() {
        "containers" => draw_containers_table(frame, area, app, &config.columns.containers),
        "images" => draw_images_table(frame, area, app, &config.columns.images),
        "volumes" => draw_volumes_table(frame, area, app, &config.columns.volumes),
        "networks" => draw_networks_table(frame, area, app, &config.columns.networks),
        _ => draw_containers_table(frame, area, app, &config.columns.containers),
    }
}

fn draw_containers_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
    let selected = app.panel_selection(PanelId::Containers);
    let col_offset = app.panel_column_offset(PanelId::Containers);
    let filter = &app.operator_filter;

    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_table_header(visible_cols);

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

    let count = filtered.len();
    let scroll_hint = format_scroll_hint(start, max_visible, columns.len());
    let title = format!(" Containers ({}) {} ", count, scroll_hint);

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

fn draw_images_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
    let selected = app.panel_selection(PanelId::Images);
    let col_offset = app.panel_column_offset(PanelId::Images);
    let filter = &app.operator_filter;

    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_table_header(visible_cols);

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

    let count = filtered.len();
    let scroll_hint = format_scroll_hint(start, max_visible, columns.len());
    let title = format!(" Images ({}) {} ", count, scroll_hint);

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

fn draw_volumes_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
    let selected = app.panel_selection(PanelId::Volumes);
    let col_offset = app.panel_column_offset(PanelId::Volumes);
    let filter = &app.operator_filter;

    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_table_header(visible_cols);

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

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(display_idx, (_, vol))| {
            let style = selected_row_style(true, display_idx == selected);

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

    let count = filtered.len();
    let scroll_hint = format_scroll_hint(start, max_visible, columns.len());
    let title = format!(" Volumes ({}) {} ", count, scroll_hint);

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

fn draw_networks_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
    let selected = app.panel_selection(PanelId::Networks);
    let col_offset = app.panel_column_offset(PanelId::Networks);
    let filter = &app.operator_filter;

    let max_visible = visible_column_count(area.width, columns.len());
    let start = col_offset.min(columns.len().saturating_sub(max_visible));
    let visible_cols = &columns[start..(start + max_visible).min(columns.len())];

    let header = create_table_header(visible_cols);

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

            let cells: Vec<Cell> = visible_cols
                .iter()
                .map(|col| match col.as_str() {
                    "Name" => Cell::from(format!(" {}", net.name))
                        .style(Style::default().fg(Color::White)),
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

    let count = filtered.len();
    let scroll_hint = format_scroll_hint(start, max_visible, columns.len());
    let title = format!(" Networks ({}) {} ", count, scroll_hint);

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

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let key_style = Style::default().fg(Color::Cyan);
    let label_style = Style::default().fg(Color::DarkGray);
    let sep_style = Style::default().fg(Color::DarkGray);
    let input_style = Style::default().fg(Color::Yellow);

    let spans = if app.operator_command_mode {
        // Show command input
        vec![
            Span::styled(" :", key_style),
            Span::styled(&app.operator_command_input, input_style),
            Span::styled("_", Style::default().fg(Color::White)),
        ]
    } else if app.operator_filter_active {
        // Show filter input
        vec![
            Span::styled(" /", key_style),
            Span::styled(&app.operator_filter, input_style),
            Span::styled("_", Style::default().fg(Color::White)),
        ]
    } else {
        // Normal footer
        let mut spans = vec![
            Span::styled(format!(" :{} ", app.operator_resource), key_style),
            Span::styled("| ", sep_style),
        ];

        if !app.operator_filter.is_empty() {
            spans.push(Span::styled(
                format!("/{} ", app.operator_filter),
                input_style,
            ));
            spans.push(Span::styled("| ", sep_style));
        }

        spans.extend(vec![
            Span::styled("d:", label_style),
            Span::styled("describe ", key_style),
            Span::styled("l:", label_style),
            Span::styled("logs ", key_style),
            Span::styled("/:", label_style),
            Span::styled("filter ", key_style),
            Span::styled("?:", label_style),
            Span::styled("help", key_style),
        ]);
        spans
    };

    let line = Line::from(spans);
    let footer = Paragraph::new(line);
    frame.render_widget(footer, area);
}

/// Popup overlay — same style as dashboard theme.
fn draw_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let popup_width = (area.width as u32 * 60 / 100) as u16;
    let popup_height = (area.height as u32 * 70 / 100) as u16;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", app.popup_title))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let content_lines: Vec<Line> = app
        .popup_content
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(Color::White))))
        .collect();

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.popup_scroll, 0));

    frame.render_widget(paragraph, popup_area);
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

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}
