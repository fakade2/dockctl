use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use crate::app::{AlertLevel, App, PanelId};
use crate::config::DockctlConfig;
use crate::themes::{Theme, ThemeAction};
use crate::ui::{
    container_display_name, format_bytes, format_ports, format_scroll_hint, format_timestamp_age,
    percentage_color, selected_row_style, short_id, truncate_str, visible_column_count,
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

fn current_columns<'a>(app: &App, config: &'a DockctlConfig) -> &'a [String] {
    let cols_config = &config.columns;
    if app.wide_mode {
        match app.operator_resource.as_str() {
            "containers" => &cols_config.containers_wide,
            "images" => &cols_config.images_wide,
            "volumes" => &cols_config.volumes_wide,
            "networks" => &cols_config.networks_wide,
            _ => &cols_config.containers_wide,
        }
    } else {
        match app.operator_resource.as_str() {
            "containers" => &cols_config.containers,
            "images" => &cols_config.images,
            "volumes" => &cols_config.volumes,
            "networks" => &cols_config.networks,
            _ => &cols_config.containers,
        }
    }
}

fn operator_handle_key(key: KeyEvent, app: &mut App, config: &DockctlConfig) -> ThemeAction {
    // Detail view mode — full-screen scrollable text, handle keys first
    if app.detail_view_active {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                app.close_detail_view();
                return ThemeAction::Consumed;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.detail_view_scroll = app.detail_view_scroll.saturating_add(1);
                return ThemeAction::Consumed;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.detail_view_scroll = app.detail_view_scroll.saturating_sub(1);
                return ThemeAction::Consumed;
            }
            KeyCode::Char('g') => {
                app.detail_view_scroll = 0;
                return ThemeAction::Consumed;
            }
            KeyCode::Char('G') => {
                app.detail_view_scroll = app.detail_view_content.len().saturating_sub(1) as u16;
                return ThemeAction::Consumed;
            }
            _ => return ThemeAction::Consumed,
        }
    }

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

        // Sort by name
        (KeyCode::Char('N'), KeyModifiers::SHIFT) => {
            app.toggle_sort("Name");
            ThemeAction::Consumed
        }
        // Sort by status/state
        (KeyCode::Char('S'), KeyModifiers::SHIFT) => {
            app.toggle_sort("State");
            ThemeAction::Consumed
        }
        // Sort by age/created
        (KeyCode::Char('A'), KeyModifiers::SHIFT) => {
            app.toggle_sort("Created");
            ThemeAction::Consumed
        }
        // Generic sort (cycle through columns)
        (KeyCode::Char('O'), KeyModifiers::SHIFT) => {
            // Cycle to next column as sort target
            let columns = current_columns(app, config);
            let current = app.sort_column.as_deref();
            let next_idx = match current {
                Some(col) => {
                    columns.iter().position(|c| c == col)
                        .map(|i| (i + 1) % columns.len())
                        .unwrap_or(0)
                }
                None => 0,
            };
            if let Some(col) = columns.get(next_idx) {
                app.toggle_sort(col);
            }
            ThemeAction::Consumed
        }

        // Column reorder (Shift+Arrow) — consumed but no-op; use config.toml
        (KeyCode::Left, KeyModifiers::SHIFT) => {
            ThemeAction::Consumed
        }
        (KeyCode::Right, KeyModifiers::SHIFT) => {
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

        // Describe — open inline detail view for selected item
        (KeyCode::Char('d'), _) => {
            let (title, content) = describe_selected(app);
            app.open_detail_view(title, content);
            ThemeAction::Consumed
        }

        // Explain / Enter — open detail popup
        (KeyCode::Char('e') | KeyCode::Enter, _) => ThemeAction::NotHandled,

        // Wide mode toggle
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
            app.toggle_wide_mode();
            ThemeAction::Consumed
        }

        // Esc — back navigation
        (KeyCode::Esc, _) => {
            if app.operator_filter_active {
                app.operator_filter_active = false;
                app.operator_filter.clear();
                ThemeAction::Consumed
            } else if !app.operator_filter.is_empty() {
                app.operator_filter.clear();
                ThemeAction::Consumed
            } else if app.pop_view() {
                // Navigated back to previous view
                ThemeAction::Consumed
            } else {
                ThemeAction::Consumed
            }
        }

        // Show all resource aliases
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
            ThemeAction::OpenPopup {
                title: "Resource Aliases".to_string(),
                content: vec![
                    "Resource Commands".to_string(),
                    "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
                    ":containers  :con  :cnt  :c    Container list".to_string(),
                    ":images      :img  :im   :i    Image list".to_string(),
                    ":volumes     :vol  :v           Volume list".to_string(),
                    ":networks    :net  :n           Network list".to_string(),
                    ":status      :pu               System pulses/dashboard".to_string(),
                    String::new(),
                    "Navigation".to_string(),
                    "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
                    "Esc          Go back to previous view".to_string(),
                    "Ctrl+A       This alias list".to_string(),
                ],
            }
        }

        // Help
        (KeyCode::Char('?'), _) => ThemeAction::OpenPopup {
            title: "Keybindings".to_string(),
            content: help_lines(),
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

/// Execute a : command — switch resource view (with history stack).
fn execute_command(app: &mut App, cmd: &str) {
    let cmd = cmd.trim().to_lowercase();
    let resource = fuzzy_match_resource(&cmd);
    if let Some(res) = resource {
        app.push_view(res); // Uses history stack
        // Sync active_panel so container actions in main.rs work
        match res {
            "containers" => app.focus_panel(PanelId::Containers),
            "images" => app.focus_panel(PanelId::Images),
            "volumes" => app.focus_panel(PanelId::Volumes),
            "networks" => app.focus_panel(PanelId::Networks),
            _ => {} // "status" doesn't need panel sync
        }
    }
}

/// Fuzzy match a command input to a resource name.
fn fuzzy_match_resource(input: &str) -> Option<&'static str> {
    let resources = ["containers", "images", "volumes", "networks", "status"];
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
        "pu" | "pulse" | "pulses" | "st" | "stat" => Some("status"),
        _ => None,
    }
}

fn operator_max_cols(app: &App, config: &DockctlConfig) -> usize {
    current_columns(app, config).len()
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
    // Layout: header (4) | table/detail (fills) | footer (1)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let header_area = outer[0];
    let table_area = outer[1];
    let footer_area = outer[2];

    draw_header(frame, header_area, app);

    if app.detail_view_active {
        draw_detail_view(frame, table_area, app);
    } else {
        draw_resource_table(frame, table_area, app, config);
    }

    draw_footer(frame, footer_area, app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let resource = &app.operator_resource;
    let resource_cap = capitalize(resource);

    let logo_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);
    let green = Style::default().fg(Color::Green);
    let red = Style::default().fg(Color::Red);
    let cyan = Style::default().fg(Color::Cyan);
    let yellow = Style::default().fg(Color::Yellow);

    // --- Row 1 ---
    // Left: " dockctl > Containers(9)[all]" or " dockctl > Containers > nginx-proxy"
    // Right: "Docker v29.2.1 | C:9/2 I:15 V:8 N:4"
    let running = app
        .containers
        .iter()
        .filter(|c| c.state == "running")
        .count();
    let stopped = app.containers.len() - running;

    let (_total, filtered) = row_counts(app);
    let filter_tag = if app.operator_filter.is_empty() {
        "all".to_string()
    } else {
        app.operator_filter.clone()
    };

    let row1_left: Vec<Span> = if app.detail_view_active {
        // Breadcrumb with detail item: dockctl > Containers > nginx-proxy
        let detail_name = app
            .detail_view_title
            .split(": ")
            .nth(1)
            .unwrap_or(&app.detail_view_title);
        vec![
            Span::styled(" dockctl", logo_style),
            Span::styled(" > ", dim),
            Span::styled(resource_cap.clone(), white),
            Span::styled(" > ", dim),
            Span::styled(detail_name.to_string(), white),
        ]
    } else {
        // Normal breadcrumb: dockctl > Containers(9)[all]
        vec![
            Span::styled(" dockctl", logo_style),
            Span::styled(" > ", dim),
            Span::styled(format!("{}({})", resource_cap, filtered), white),
            Span::styled(format!("[{}]", filter_tag), yellow),
        ]
    };

    // Right side: runtime + counts
    let runtime_str = if let Some(info) = &app.system_info {
        format!("{} v{}", info.runtime_type, info.server_version)
    } else {
        "connecting...".to_string()
    };

    let mut row1_right: Vec<Span> = vec![
        Span::styled(format!("{} ", runtime_str), dim),
        Span::styled("| ", dim),
        Span::styled("C:", dim),
        Span::styled(format!("{}", running), green),
        Span::styled("/", dim),
        Span::styled(format!("{}", stopped), if stopped > 0 { red } else { dim }),
        Span::styled(" I:", dim),
        Span::styled(format!("{}", app.images.len()), cyan),
        Span::styled(" V:", dim),
        Span::styled(format!("{}", app.volumes.len()), cyan),
        Span::styled(" N:", dim),
        Span::styled(format!("{}", app.networks.len()), cyan),
        Span::raw(" "),
    ];

    let row1_left_len: usize = row1_left.iter().map(|s| s.width()).sum();
    let row1_right_len: usize = row1_right.iter().map(|s| s.width()).sum();
    let row1_pad = (area.width as usize).saturating_sub(row1_left_len + row1_right_len);

    let mut row1_spans = row1_left;
    row1_spans.push(Span::raw(" ".repeat(row1_pad)));
    row1_spans.append(&mut row1_right);

    // --- Row 2 ---
    // Left: context-sensitive shortcuts
    // Right: CPU:12% RAM:68% | 8cores 16GB
    let row2_left_spans = if app.detail_view_active {
        // Detail view shortcuts
        vec![
            Span::raw(" "),
            Span::styled("<Esc>", Style::default().fg(Color::Cyan)),
            Span::styled("back ", dim),
            Span::styled("<j/k>", Style::default().fg(Color::Cyan)),
            Span::styled("scroll ", dim),
            Span::styled("<g>", Style::default().fg(Color::Cyan)),
            Span::styled("top ", dim),
            Span::styled("<G>", Style::default().fg(Color::Cyan)),
            Span::styled("bottom", dim),
        ]
    } else {
        build_shortcut_row(resource)
    };

    let cpu_pct = app.host_cpu_percent;
    let mem_pct = if app.host_memory_total > 0 {
        (app.host_memory_used as f64 / app.host_memory_total as f64) * 100.0
    } else {
        0.0
    };
    let cpu_color = percentage_color(cpu_pct);
    let mem_color = percentage_color(mem_pct);

    let mut row2_right: Vec<Span> = vec![
        Span::styled("CPU:", dim),
        Span::styled(format!("{:.0}%", cpu_pct), Style::default().fg(cpu_color)),
        Span::styled(" RAM:", dim),
        Span::styled(format!("{:.0}%", mem_pct), Style::default().fg(mem_color)),
    ];

    if let Some(info) = &app.system_info {
        row2_right.push(Span::styled(
            format!(
                " | {}cores {}  ",
                info.cpus,
                format_bytes(info.memory_total)
            ),
            dim,
        ));
    } else {
        row2_right.push(Span::raw(" "));
    }

    let row2_left_len: usize = row2_left_spans.iter().map(|s| s.width()).sum();
    let row2_right_len: usize = row2_right.iter().map(|s| s.width()).sum();
    let row2_pad = (area.width as usize).saturating_sub(row2_left_len + row2_right_len);

    let mut row2_spans = row2_left_spans;
    row2_spans.push(Span::raw(" ".repeat(row2_pad)));
    row2_spans.append(&mut row2_right);

    let lines = vec![Line::from(row1_spans), Line::from(row2_spans)];

    let header = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, area);
}

/// Build the context-sensitive shortcut spans for the header row 3.
fn build_shortcut_row(resource: &str) -> Vec<Span<'static>> {
    let key_style = Style::default().fg(Color::Cyan);
    let label_style = Style::default().fg(Color::DarkGray);

    let shortcuts: Vec<(&str, &str)> = match resource {
        "containers" => vec![
            ("<d>", "describe "),
            ("<e>", "explain "),
            ("<s>", "stop "),
            ("<r>", "restart "),
            ("<x>", "delete "),
            ("<l>", "logs "),
            ("<E>", "exec"),
        ],
        "images" => vec![("<d>", "describe "), ("<e>", "explain")],
        "volumes" => vec![("<d>", "describe "), ("<e>", "explain")],
        "networks" => vec![("<d>", "describe "), ("<e>", "explain")],
        _ => vec![],
    };

    let mut spans = vec![Span::raw(" ")];
    for (key, label) in shortcuts {
        spans.push(Span::styled(key.to_string(), key_style));
        spans.push(Span::styled(label.to_string(), label_style));
    }
    spans
}

fn draw_resource_table(frame: &mut Frame, area: Rect, app: &App, config: &DockctlConfig) {
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

fn draw_containers_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
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

fn draw_images_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
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

fn draw_volumes_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
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

fn draw_networks_table(frame: &mut Frame, area: Rect, app: &App, columns: &[String]) {
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

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let key_style = Style::default().fg(Color::Cyan);
    let label_style = Style::default().fg(Color::DarkGray);
    let sep_style = Style::default().fg(Color::DarkGray);
    let input_style = Style::default().fg(Color::Yellow);

    let spans = if app.detail_view_active {
        // Detail view footer
        vec![
            Span::styled(" Esc:", label_style),
            Span::styled("back ", key_style),
            Span::styled("j/k:", label_style),
            Span::styled("scroll ", key_style),
            Span::styled("g:", label_style),
            Span::styled("top ", key_style),
            Span::styled("G:", label_style),
            Span::styled("bottom", key_style),
        ]
    } else if app.operator_command_mode {
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
        // Normal footer with row count
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

        // Row count display
        let (total, filtered) = row_counts(app);
        let row_str = if !app.operator_filter.is_empty() && filtered != total {
            format!("  rows:{}/{}", filtered, total)
        } else {
            format!("  rows:{}", total)
        };
        spans.push(Span::styled(row_str, label_style));

        spans
    };

    let line = Line::from(spans);
    let footer = Paragraph::new(line);
    frame.render_widget(footer, area);
}

// ---------------------------------------------------------------------------
// Sort-aware header
// ---------------------------------------------------------------------------

/// Create a table header row with sort indicators. If `sort_column` matches
/// a visible column name, an arrow is prepended to that column's header.
fn create_sorted_header(
    columns: &[String],
    sort_column: Option<&str>,
    sort_ascending: bool,
) -> Row<'static> {
    let cells: Vec<Cell> = columns
        .iter()
        .map(|c| {
            let label = if sort_column == Some(c.as_str()) {
                let arrow = if sort_ascending {
                    "\u{25b2}"
                } else {
                    "\u{25bc}"
                };
                format!(" {} {}", arrow, c)
            } else {
                format!(" {}", c)
            };
            Cell::from(label).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();
    Row::new(cells).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
}

// ---------------------------------------------------------------------------
// Status dot helpers
// ---------------------------------------------------------------------------

/// Return (dot_char, dot_color) for a container based on its state.
fn container_status_dot(state: &str) -> (&'static str, Color) {
    match state {
        "running" => ("\u{25cf}", Color::Green),
        "paused" | "restarting" => ("\u{25cf}", Color::Yellow),
        "created" => ("\u{25cb}", Color::DarkGray),
        "exited" | "dead" => ("\u{25cf}", Color::Red),
        _ => ("\u{25cf}", Color::Red),
    }
}

/// Return (dot_char, dot_color) for an image based on its tags.
fn image_status_dot(repo_tags: &[String]) -> (&'static str, Color) {
    let is_none = repo_tags.is_empty() || repo_tags.iter().all(|t| t == "<none>:<none>");
    if is_none {
        ("\u{25cf}", Color::Red)
    } else {
        ("\u{25cf}", Color::Green)
    }
}

/// Return (dot_char, dot_color) for a network based on connected containers.
fn network_status_dot(container_count: usize) -> (&'static str, Color) {
    if container_count > 0 {
        ("\u{25cf}", Color::Green)
    } else {
        ("\u{25cf}", Color::DarkGray)
    }
}

// ---------------------------------------------------------------------------
// K9s-style table title: " Containers(12)[all] [1-4/8] "
// ---------------------------------------------------------------------------

fn k9s_table_title(
    resource: &str,
    _total_count: usize,
    filtered_count: usize,
    filter: &str,
    scroll_hint: &str,
) -> String {
    let filter_tag = if filter.is_empty() {
        "all".to_string()
    } else {
        filter.to_string()
    };
    format!(
        " {}({})[{}] {} ",
        capitalize(resource),
        filtered_count,
        filter_tag,
        scroll_hint,
    )
}

// ---------------------------------------------------------------------------
// Pulses / Status view  (:status or :pu)
// ---------------------------------------------------------------------------

fn draw_status_view(frame: &mut Frame, area: Rect, app: &App) {
    let dim = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);
    let cyan = Style::default().fg(Color::Cyan);
    let green = Style::default().fg(Color::Green);
    let red = Style::default().fg(Color::Red);

    // Split area into top (system + docker) and bottom (top-cpu + top-mem + alerts)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // System stats + Docker stats
            Constraint::Min(1),    // Top containers + alerts
        ])
        .split(area);

    let top_halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[0]);

    let bottom_thirds = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(outer[1]);

    // --- System Stats ---
    let cpu_pct = app.host_cpu_percent;
    let mem_pct = if app.host_memory_total > 0 {
        (app.host_memory_used as f64 / app.host_memory_total as f64) * 100.0
    } else {
        0.0
    };
    let swap_pct = if app.host_swap_total > 0 {
        (app.host_swap_used as f64 / app.host_swap_total as f64) * 100.0
    } else {
        0.0
    };

    let cpu_bar = gauge_bar(cpu_pct, 20);
    let mem_bar = gauge_bar(mem_pct, 20);
    let swap_bar = gauge_bar(swap_pct, 20);

    let system_lines = vec![
        Line::from(vec![
            Span::styled(" CPU  ", dim),
            Span::styled(cpu_bar, Style::default().fg(percentage_color(cpu_pct))),
            Span::styled(format!(" {:.1}%", cpu_pct), white),
        ]),
        Line::from(vec![
            Span::styled(" RAM  ", dim),
            Span::styled(mem_bar, Style::default().fg(percentage_color(mem_pct))),
            Span::styled(
                format!(
                    " {:.1}% ({}/{})",
                    mem_pct,
                    format_bytes(app.host_memory_used),
                    format_bytes(app.host_memory_total)
                ),
                white,
            ),
        ]),
        Line::from(vec![
            Span::styled(" Swap ", dim),
            Span::styled(swap_bar, Style::default().fg(percentage_color(swap_pct))),
            Span::styled(
                format!(
                    " {:.1}% ({}/{})",
                    swap_pct,
                    format_bytes(app.host_swap_used),
                    format_bytes(app.host_swap_total)
                ),
                white,
            ),
        ]),
    ];

    let system_block = Block::default()
        .title(" System ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(dim);
    let system_widget = Paragraph::new(system_lines).block(system_block);
    frame.render_widget(system_widget, top_halves[0]);

    // --- Docker Stats ---
    let running = app
        .containers
        .iter()
        .filter(|c| c.state == "running")
        .count();
    let stopped = app.containers.len() - running;

    let docker_lines = vec![
        Line::from(vec![
            Span::styled(" Containers  ", dim),
            Span::styled(format!("{}", running), green),
            Span::styled(" running  ", dim),
            Span::styled(format!("{}", stopped), if stopped > 0 { red } else { dim }),
            Span::styled(" stopped", dim),
        ]),
        Line::from(vec![
            Span::styled(" Images      ", dim),
            Span::styled(format!("{}", app.images.len()), cyan),
        ]),
        Line::from(vec![
            Span::styled(" Volumes     ", dim),
            Span::styled(format!("{}", app.volumes.len()), cyan),
        ]),
        Line::from(vec![
            Span::styled(" Networks    ", dim),
            Span::styled(format!("{}", app.networks.len()), cyan),
        ]),
    ];

    let docker_block = Block::default()
        .title(" Docker ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(dim);
    let docker_widget = Paragraph::new(docker_lines).block(docker_block);
    frame.render_widget(docker_widget, top_halves[1]);

    // --- Top 5 by CPU ---
    let mut cpu_sorted: Vec<(&String, f64)> = app
        .container_stats
        .iter()
        .map(|(id, (cpu, _))| (id, *cpu))
        .collect();
    cpu_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    cpu_sorted.truncate(5);

    let cpu_lines: Vec<Line> = cpu_sorted
        .iter()
        .map(|(id, cpu)| {
            let name = app
                .containers
                .iter()
                .find(|c| &c.id == *id)
                .map(|c| container_display_name(&c.names))
                .unwrap_or_else(|| short_id(id));
            Line::from(vec![
                Span::styled(
                    format!(" {:.1}%", cpu),
                    Style::default().fg(percentage_color(*cpu)),
                ),
                Span::styled(format!("  {}", name), white),
            ])
        })
        .collect();

    let cpu_block = Block::default()
        .title(" Top CPU ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(dim);
    let cpu_widget = Paragraph::new(cpu_lines).block(cpu_block);
    frame.render_widget(cpu_widget, bottom_thirds[0]);

    // --- Top 5 by Memory ---
    let mut mem_sorted: Vec<(&String, u64)> = app
        .container_stats
        .iter()
        .map(|(id, (_, mem))| (id, *mem))
        .collect();
    mem_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    mem_sorted.truncate(5);

    let mem_lines: Vec<Line> = mem_sorted
        .iter()
        .map(|(id, mem)| {
            let name = app
                .containers
                .iter()
                .find(|c| &c.id == *id)
                .map(|c| container_display_name(&c.names))
                .unwrap_or_else(|| short_id(id));
            Line::from(vec![
                Span::styled(format!(" {}", format_bytes(*mem)), cyan),
                Span::styled(format!("  {}", name), white),
            ])
        })
        .collect();

    let mem_block = Block::default()
        .title(" Top Memory ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(dim);
    let mem_widget = Paragraph::new(mem_lines).block(mem_block);
    frame.render_widget(mem_widget, bottom_thirds[1]);

    // --- Alerts ---
    let alert_lines: Vec<Line> = if app.alerts.is_empty() {
        vec![Line::from(Span::styled(" No active alerts", dim))]
    } else {
        app.alerts
            .iter()
            .map(|alert| {
                let (icon, color) = match alert.level {
                    AlertLevel::Critical => ("\u{25cf}", Color::Red),
                    AlertLevel::Warning => ("\u{25cf}", Color::Yellow),
                };
                Line::from(vec![
                    Span::styled(format!(" {} ", icon), Style::default().fg(color)),
                    Span::styled(alert.message.clone(), Style::default().fg(color)),
                ])
            })
            .collect()
    };

    let alert_block = Block::default()
        .title(" Alerts ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(dim);
    let alert_widget = Paragraph::new(alert_lines).block(alert_block);
    frame.render_widget(alert_widget, bottom_thirds[2]);
}

/// Build a text-based gauge bar: [========------] for a given percentage.
fn gauge_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!(
        "[{}{}]",
        "\u{2588}".repeat(filled),
        "\u{2500}".repeat(empty),
    )
}

/// Return (total_rows, filtered_rows) for the current resource view.
fn row_counts(app: &App) -> (usize, usize) {
    let filter = &app.operator_filter;
    match app.operator_resource.as_str() {
        "containers" => {
            let total = app.containers.len();
            let filtered = if filter.is_empty() {
                total
            } else {
                let f = filter.to_lowercase();
                app.containers
                    .iter()
                    .filter(|c| {
                        container_display_name(&c.names).to_lowercase().contains(&f)
                            || c.image.to_lowercase().contains(&f)
                            || c.state.to_lowercase().contains(&f)
                    })
                    .count()
            };
            (total, filtered)
        }
        "images" => {
            let total = app.images.len();
            let filtered = if filter.is_empty() {
                total
            } else {
                let f = filter.to_lowercase();
                app.images
                    .iter()
                    .filter(|img| {
                        img.repo_tags.iter().any(|t| t.to_lowercase().contains(&f))
                            || short_id(&img.id).contains(&f)
                    })
                    .count()
            };
            (total, filtered)
        }
        "volumes" => {
            let total = app.volumes.len();
            let filtered = if filter.is_empty() {
                total
            } else {
                let f = filter.to_lowercase();
                app.volumes
                    .iter()
                    .filter(|vol| {
                        vol.name.to_lowercase().contains(&f)
                            || vol.driver.to_lowercase().contains(&f)
                    })
                    .count()
            };
            (total, filtered)
        }
        "networks" => {
            let total = app.networks.len();
            let filtered = if filter.is_empty() {
                total
            } else {
                let f = filter.to_lowercase();
                app.networks
                    .iter()
                    .filter(|net| {
                        net.name.to_lowercase().contains(&f)
                            || net.driver.to_lowercase().contains(&f)
                    })
                    .count()
            };
            (total, filtered)
        }
        _ => (0, 0),
    }
}

/// Inline detail view — replaces the table entirely (K9s-style).
fn draw_detail_view(frame: &mut Frame, area: Rect, app: &App) {
    let dim = Style::default().fg(Color::DarkGray);
    let cyan = Style::default().fg(Color::Cyan);
    let white = Style::default().fg(Color::White);

    let block = Block::default()
        .title(format!(" {} ", app.detail_view_title))
        .title_style(cyan.add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let content_lines: Vec<Line> = app
        .detail_view_content
        .iter()
        .map(|line| {
            if line.is_empty() {
                Line::from("")
            } else if line.ends_with(':')
                && line
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
            {
                // Section header (e.g. "Connected Containers:")
                Line::from(Span::styled(
                    line.clone(),
                    cyan.add_modifier(Modifier::BOLD),
                ))
            } else if let Some(colon_pos) = line.find(':') {
                // Key-value line: key in dim, value in white
                let (key, val) = line.split_at(colon_pos + 1);
                Line::from(vec![
                    Span::styled(key.to_string(), dim),
                    Span::styled(val.to_string(), white),
                ])
            } else {
                Line::from(Span::styled(line.clone(), white))
            }
        })
        .collect();

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_view_scroll, 0));

    frame.render_widget(paragraph, area);
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

fn help_lines() -> Vec<String> {
    let sep = "─────────────────────────────────";
    vec![
        "Navigation".into(),
        sep.into(),
        "j / ↓        Move down".into(),
        "k / ↑        Move up".into(),
        "h / ←        Scroll columns left".into(),
        "l / →        Scroll columns right".into(),
        "g            Jump to top".into(),
        "G            Jump to bottom".into(),
        "Esc          Go back / clear filter".into(),
        String::new(),
        "Commands".into(),
        sep.into(),
        ":            Command mode".into(),
        "             :containers  :images  :volumes  :networks".into(),
        "             :status  :pu  (pulses dashboard)".into(),
        "/            Filter (regex)".into(),
        "Ctrl+A       Show all resource aliases".into(),
        String::new(),
        "Views".into(),
        sep.into(),
        "d            Describe (inline detail view)".into(),
        "e / Enter    Explain (inline detail view)".into(),
        "             j/k to scroll, g/G top/bottom, Esc back".into(),
        String::new(),
        "Sort & Display".into(),
        sep.into(),
        "Shift+O      Cycle sort column".into(),
        "Shift+N      Sort by name".into(),
        "Shift+S      Sort by state".into(),
        "Shift+A      Sort by age/created".into(),
        "Ctrl+W       Toggle wide mode (extra columns)".into(),
        String::new(),
        "Container Actions (in :containers)".into(),
        sep.into(),
        "s            Stop container".into(),
        "r            Restart container".into(),
        "x            Remove container".into(),
        "l            View logs".into(),
        "E            Exec into container (tmux)".into(),
        String::new(),
        "General".into(),
        sep.into(),
        "?            This help".into(),
        "q            Quit (or close detail/help)".into(),
    ]
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}
