mod alerts;
mod containers;
mod images;
mod networks;
#[allow(dead_code)]
mod pods;
mod resources;
mod status;
mod volumes;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::{App, InputMode, PanelId};
use crate::config::DockctlConfig;
use crate::themes::{Theme, ThemeAction};

pub struct DashboardTheme;

impl Default for DashboardTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardTheme {
    pub fn new() -> Self {
        Self
    }
}

impl Theme for DashboardTheme {
    fn name(&self) -> &str {
        "dashboard"
    }

    fn draw(&self, frame: &mut Frame, app: &App, config: &DockctlConfig) {
        draw(frame, app, config);
    }

    fn handle_key(&self, key: KeyEvent, app: &mut App, config: &DockctlConfig) -> ThemeAction {
        dashboard_handle_key(key, app, config)
    }
}

// ─── Rendering ───────────────────────────────────────────────────────────────

/// Root render function — 7-panel layout with left sidebar, right accordion,
/// bottom status bar, and optional popup overlay.
fn draw(frame: &mut Frame, app: &App, config: &DockctlConfig) {
    let layout = &config.layout;
    // 1. Split vertically: main area (fills) + bottom status bar (1 line)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let main_area = outer[0];
    let bar_area = outer[1];

    // 2. Split main area horizontally: left column + right column
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(layout.left_panel_percent),
            Constraint::Percentage(100 - layout.left_panel_percent),
        ])
        .split(main_area);

    let left_area = columns[0];
    let right_area = columns[1];

    // 3. Left column: 3 panels stacked equally
    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(left_area);

    alerts::draw_alerts_panel(
        frame,
        left_panels[0],
        app,
        app.active_panel == PanelId::Alerts,
    );
    resources::draw_resources_panel(
        frame,
        left_panels[1],
        app,
        app.active_panel == PanelId::Resources,
    );
    status::draw_status_panel(
        frame,
        left_panels[2],
        app,
        app.active_panel == PanelId::Status,
    );

    // 4. Right column: log view, detail view, or accordion
    if app.log_view_active {
        super::components::draw_log_view(frame, right_area, app);
    } else if app.detail_view_active {
        super::components::draw_detail_view(frame, right_area, app);
    } else {
        draw_right_accordion(frame, right_area, app, config);
    }

    // 5. Bottom status bar
    draw_status_bar(frame, bar_area);

    // 6. Popup overlay (if open)
    if app.show_popup {
        super::components::draw_popup(frame, app);
    }
}

/// Renders the right column with accordion behavior: the expanded panel gets
/// most of the vertical space, collapsed panels get just 3 lines (title bar).
fn draw_right_accordion(frame: &mut Frame, area: Rect, app: &App, config: &DockctlConfig) {
    let right_panels = PanelId::RIGHT;
    let expanded = app.expanded_right;

    // Build constraints: expanded gets Min(8), collapsed get Length(1)
    let constraints: Vec<Constraint> = right_panels
        .iter()
        .map(|&p| {
            if p == expanded {
                Constraint::Min(8)
            } else {
                Constraint::Length(1)
            }
        })
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (i, &panel) in right_panels.iter().enumerate() {
        let is_expanded = panel == expanded;
        let is_focused = panel == app.active_panel;

        match panel {
            PanelId::Containers => {
                containers::draw_container_panel(
                    frame,
                    chunks[i],
                    app,
                    is_expanded,
                    is_focused,
                    &config.columns.containers,
                );
            }
            PanelId::Images => {
                images::draw_image_panel(
                    frame,
                    chunks[i],
                    app,
                    is_expanded,
                    is_focused,
                    &config.columns.images,
                );
            }
            PanelId::Volumes => {
                volumes::draw_volume_panel(
                    frame,
                    chunks[i],
                    app,
                    is_expanded,
                    is_focused,
                    &config.columns.volumes,
                );
            }
            PanelId::Networks => {
                networks::draw_network_panel(
                    frame,
                    chunks[i],
                    app,
                    is_expanded,
                    is_focused,
                    &config.columns.networks,
                );
            }
            _ => {}
        }
    }
}

/// Renders the bottom status bar.
fn draw_status_bar(frame: &mut Frame, area: Rect) {
    let version = concat!("dockctl v", env!("CARGO_PKG_VERSION"));

    let key_style = Style::default().fg(Color::Cyan);
    let label_style = Style::default().fg(Color::DarkGray);
    let sep_style = Style::default().fg(Color::DarkGray);

    let left_spans = vec![
        Span::styled(" Explain: ", label_style),
        Span::styled("e", key_style),
        Span::styled(" | ", sep_style),
        Span::styled("Quit: ", label_style),
        Span::styled("q", key_style),
        Span::styled(" | ", sep_style),
        Span::styled("Keybindings: ", label_style),
        Span::styled("?", key_style),
    ];

    // Calculate padding to right-align the version string
    let left_len: usize = left_spans.iter().map(|s| s.width()).sum();
    let right_len = version.len() + 1;
    let total_width = area.width as usize;
    let padding = total_width.saturating_sub(left_len + right_len);

    let mut spans = left_spans;
    spans.push(Span::raw(" ".repeat(padding)));
    spans.push(Span::styled(
        format!("{} ", version),
        Style::default().fg(Color::DarkGray),
    ));

    let line = Line::from(spans);
    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
}

// ─── Key handling ────────────────────────────────────────────────────────────

/// Dashboard-specific key handling.
///
/// Handles ALL key events: popup mode, log view, detail view, search mode,
/// navigation, container actions, and explain. The engine only executes
/// async ThemeAction results.
fn dashboard_handle_key(key: KeyEvent, app: &mut App, config: &DockctlConfig) -> ThemeAction {
    // ── Popup mode ─────────────────────────────────────────────────
    if app.show_popup {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                app.close_popup();
                return ThemeAction::Consumed;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = app.popup_content.len().saturating_sub(1) as u16;
                app.popup_scroll = app.popup_scroll.saturating_add(1).min(max);
                return ThemeAction::Consumed;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.popup_scroll = app.popup_scroll.saturating_sub(1);
                return ThemeAction::Consumed;
            }
            _ => return ThemeAction::Consumed,
        }
    }

    // ── Log view mode ──────────────────────────────────────────────
    if app.log_view_active {
        let max_scroll = app.log_view_lines.len().saturating_sub(1) as u16;
        return match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                app.close_log_view();
                ThemeAction::Consumed
            }
            KeyCode::Char('f') => {
                app.log_view_follow = !app.log_view_follow;
                if app.log_view_follow {
                    app.log_view_scroll = max_scroll;
                }
                ThemeAction::Consumed
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.log_view_follow = false;
                app.log_view_scroll = app.log_view_scroll.saturating_add(1).min(max_scroll);
                ThemeAction::Consumed
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.log_view_follow = false;
                app.log_view_scroll = app.log_view_scroll.saturating_sub(1);
                ThemeAction::Consumed
            }
            KeyCode::Char('g') => {
                app.log_view_follow = false;
                app.log_view_scroll = 0;
                ThemeAction::Consumed
            }
            KeyCode::Char('G') => {
                app.log_view_follow = true;
                app.log_view_scroll = max_scroll;
                ThemeAction::Consumed
            }
            KeyCode::Char(n @ '0'..='5') => {
                let tail = match n {
                    '0' => 500,
                    '1' => 50,
                    '2' => 100,
                    '3' => 200,
                    '4' => 500,
                    '5' => 1000,
                    _ => 500,
                };
                ThemeAction::RefreshLogs {
                    id: app.log_view_container_id.clone(),
                    tail,
                }
            }
            KeyCode::PageDown => {
                app.log_view_follow = false;
                app.log_view_scroll = app.log_view_scroll.saturating_add(20).min(max_scroll);
                ThemeAction::Consumed
            }
            KeyCode::PageUp => {
                app.log_view_follow = false;
                app.log_view_scroll = app.log_view_scroll.saturating_sub(20);
                ThemeAction::Consumed
            }
            _ => ThemeAction::Consumed,
        };
    }

    // ── Detail view mode ───────────────────────────────────────────
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

    // ── Search mode ────────────────────────────────────────────────
    if app.input_mode == InputMode::Search {
        match key.code {
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.search_query.clear();
            }
            KeyCode::Enter => {
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            _ => {}
        }
        return ThemeAction::Consumed;
    }

    // ── Normal mode ────────────────────────────────────────────────
    let is_container_context = app.active_panel == PanelId::Containers;

    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => ThemeAction::Quit,

        // Panel switching (1-7)
        (KeyCode::Char(n @ '1'..='7'), _) => {
            let num = n as u8 - b'0';
            if let Some(panel) = PanelId::from_number(num) {
                app.focus_panel(panel);
            }
            ThemeAction::Consumed
        }

        // Tab cycles panels
        (KeyCode::Tab, _) => {
            app.next_panel();
            ThemeAction::Consumed
        }
        (KeyCode::BackTab, _) => {
            app.previous_panel();
            ThemeAction::Consumed
        }

        // Vertical navigation within active panel
        (KeyCode::Down | KeyCode::Char('j'), _) => {
            app.next_item();
            ThemeAction::Consumed
        }
        (KeyCode::Up | KeyCode::Char('k'), _) => {
            app.previous_item();
            ThemeAction::Consumed
        }

        // Horizontal column scrolling (right panels only)
        // Note: 'l' is also logs for containers, so only scroll columns for non-container right panels
        (KeyCode::Right, _) if app.active_panel.is_right() => {
            let max_cols = dashboard_max_cols(app, config);
            app.scroll_columns_right(max_cols);
            ThemeAction::Consumed
        }
        (KeyCode::Char('l'), _) if app.active_panel.is_right() && !is_container_context => {
            let max_cols = dashboard_max_cols(app, config);
            app.scroll_columns_right(max_cols);
            ThemeAction::Consumed
        }
        (KeyCode::Left | KeyCode::Char('h'), _) if app.active_panel.is_right() => {
            app.scroll_columns_left();
            ThemeAction::Consumed
        }

        // Search
        (KeyCode::Char('/'), _) => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
            ThemeAction::Consumed
        }

        // Esc in normal mode — clear search
        (KeyCode::Esc, _) => {
            if !app.search_query.is_empty() {
                app.search_query.clear();
            }
            ThemeAction::Consumed
        }

        // Explain / Enter — open detail popup
        (KeyCode::Char('e') | KeyCode::Enter, _) => ThemeAction::ExplainItem { use_inline: false },

        // Keybindings help
        (KeyCode::Char('?'), _) => ThemeAction::OpenPopup {
            title: "Keybindings".to_string(),
            content: dashboard_help_lines(),
        },

        // ── Container actions (only when container panel is active) ─────
        (KeyCode::Char('s'), _) if is_container_context => {
            if let Some(id) = app.selected_container_id() {
                ThemeAction::StopContainer { id }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('r'), _) if is_container_context => {
            if let Some(id) = app.selected_container_id() {
                ThemeAction::RestartContainer { id }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('x'), _) if is_container_context => {
            if let Some(id) = app.selected_container_id() {
                ThemeAction::RemoveContainer { id }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('l'), _) if is_container_context => {
            if let Some(id) = app.selected_container_id() {
                let name = app
                    .containers
                    .get(app.selected_index())
                    .map(|c| crate::display::container_display_name(&c.names))
                    .unwrap_or_default();
                ThemeAction::FetchLogs {
                    id,
                    name,
                    tail: 500,
                }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('E'), _) if is_container_context => {
            if let Some(id) = app.selected_container_id() {
                ThemeAction::ExecContainer { id }
            } else {
                ThemeAction::Consumed
            }
        }

        // Everything else — not handled
        _ => ThemeAction::NotHandled,
    }
}

fn dashboard_max_cols(app: &App, config: &DockctlConfig) -> usize {
    match app.active_panel {
        PanelId::Containers => config.columns.containers.len(),
        PanelId::Images => config.columns.images.len(),
        PanelId::Volumes => config.columns.volumes.len(),
        PanelId::Networks => config.columns.networks.len(),
        _ => 0,
    }
}

fn dashboard_help_lines() -> Vec<String> {
    let sep = "─────────────────────────────────";
    vec![
        "Navigation".into(),
        sep.into(),
        "1-7          Focus panel".into(),
        "Tab          Next panel".into(),
        "Shift+Tab    Previous panel".into(),
        "j / ↓        Move down".into(),
        "k / ↑        Move up".into(),
        "h / ←        Scroll columns left".into(),
        "l / →        Scroll columns right".into(),
        String::new(),
        "Actions".into(),
        sep.into(),
        "e / Enter    Explain (detail popup)".into(),
        "d            Describe".into(),
        "/            Search".into(),
        "?            This help".into(),
        "q            Quit (or close popup)".into(),
        String::new(),
        "Container Actions (panel 4)".into(),
        sep.into(),
        "s            Stop container".into(),
        "r            Restart container".into(),
        "x            Remove container".into(),
        "l            View logs".into(),
        "E            Exec into container (tmux)".into(),
    ]
}
