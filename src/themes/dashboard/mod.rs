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
    let has_status_line = app.input_mode == InputMode::PullInput || app.status_message.is_some();

    // 1. Split vertically: main area + optional command line + bottom status bar
    let outer = if has_status_line {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(frame.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(frame.area())
    };

    let main_area = outer[0];
    let bar_area = if has_status_line {
        // Draw the command/status line
        draw_command_line(frame, outer[1], app);
        outer[2]
    } else {
        outer[1]
    };

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

    // 5. Bottom status bar (context-sensitive shortcuts)
    draw_status_bar(frame, bar_area, app);

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

/// Renders the command/status line (shown only when pull input or status message is active).
fn draw_command_line(frame: &mut Frame, area: Rect, app: &App) {
    let line = if app.input_mode == InputMode::PullInput {
        Line::from(vec![
            Span::styled(" Pull: ", Style::default().fg(Color::Cyan)),
            Span::styled(app.pull_input.clone(), Style::default().fg(Color::White)),
            Span::styled("\u{2588}", Style::default().fg(Color::White)),
        ])
    } else if let Some(ref msg) = app.status_message {
        Line::from(Span::styled(
            format!(" {}", msg),
            Style::default().fg(Color::Yellow),
        ))
    } else {
        Line::from("")
    };

    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Rgb(22, 27, 34)));
    frame.render_widget(paragraph, area);
}

/// Renders the bottom status bar (context-sensitive shortcuts + version).
fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let version = concat!("dockctl v", env!("CARGO_PKG_VERSION"));

    let key_style = Style::default().fg(Color::Cyan);
    let label_style = Style::default().fg(Color::DarkGray);

    let mut left_spans: Vec<Span> = vec![
        Span::styled(" ", label_style),
        Span::styled("e", key_style),
        Span::styled(":explain ", label_style),
    ];

    // Panel-specific shortcuts
    match app.active_panel {
        PanelId::Containers => {
            left_spans.extend(vec![
                Span::styled("s", key_style),
                Span::styled(":stop ", label_style),
                Span::styled("r", key_style),
                Span::styled(":restart ", label_style),
                Span::styled("x", key_style),
                Span::styled(":rm ", label_style),
                Span::styled("l", key_style),
                Span::styled(":logs ", label_style),
                Span::styled("E", key_style),
                Span::styled(":exec ", label_style),
            ]);
        }
        PanelId::Images => {
            left_spans.extend(vec![
                Span::styled("p", key_style),
                Span::styled(":pull ", label_style),
                Span::styled("x", key_style),
                Span::styled(":rm ", label_style),
            ]);
        }
        _ => {}
    }

    left_spans.extend(vec![
        Span::styled("/", key_style),
        Span::styled("search ", label_style),
        Span::styled("?", key_style),
        Span::styled("help ", label_style),
        Span::styled("q", key_style),
        Span::styled(":quit", label_style),
    ]);

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
            // Horizontal scroll
            KeyCode::Right | KeyCode::Char('l') => {
                if !app.log_view_wrap {
                    app.log_view_h_scroll = app.log_view_h_scroll.saturating_add(4);
                }
                ThemeAction::Consumed
            }
            KeyCode::Left | KeyCode::Char('h') => {
                app.log_view_h_scroll = app.log_view_h_scroll.saturating_sub(4);
                ThemeAction::Consumed
            }
            KeyCode::Home => {
                app.log_view_h_scroll = 0;
                ThemeAction::Consumed
            }
            // Wrap toggle
            KeyCode::Char('w') => {
                app.log_view_wrap = !app.log_view_wrap;
                app.log_view_h_scroll = 0;
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

    // ── Pull input mode ─────────────────────────────────────────────
    if app.input_mode == InputMode::PullInput {
        match key.code {
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.pull_input.clear();
            }
            KeyCode::Enter => {
                let reference = app.pull_input.trim().to_string();
                app.input_mode = InputMode::Normal;
                app.pull_input.clear();
                if !reference.is_empty() {
                    return ThemeAction::PullImage { reference };
                }
            }
            KeyCode::Backspace => {
                app.pull_input.pop();
            }
            KeyCode::Char(c) => {
                app.pull_input.push(c);
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

        // ── Image actions (only when image panel is active) ─────
        (KeyCode::Char('p'), _) if app.active_panel == PanelId::Images => {
            app.input_mode = InputMode::PullInput;
            app.pull_input.clear();
            ThemeAction::Consumed
        }
        (KeyCode::Char('x'), _) if app.active_panel == PanelId::Images => {
            if let Some(id) = app.selected_image_id() {
                ThemeAction::RemoveImage { id, force: false }
            } else {
                ThemeAction::Consumed
            }
        }

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
        String::new(),
        "Image Actions (panel 5)".into(),
        sep.into(),
        "p            Pull image (enter reference)".into(),
        "x            Remove image".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::config::DockctlConfig;
    use crate::runtime::ContainerSummary;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::collections::HashMap;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn make_container(id: &str) -> ContainerSummary {
        ContainerSummary {
            id: id.into(),
            name: id.into(),
            names: vec![id.into()],
            image: "img".into(),
            image_id: format!("sha256:{id}"),
            command: "/bin/sh".into(),
            state: "running".into(),
            status: "Up 1 hour".into(),
            created: 1700000000,
            ports: vec![],
            labels: HashMap::new(),
        }
    }

    // ── Panel switching ──────────────────────────────────────────────

    #[test]
    fn number_keys_switch_panels() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('4')), &mut app, &config);
        assert_eq!(app.active_panel, PanelId::Containers);
    }

    #[test]
    fn number_1_focuses_alerts() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('1')), &mut app, &config);
        assert_eq!(app.active_panel, PanelId::Alerts);
    }

    #[test]
    fn number_5_focuses_images() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('5')), &mut app, &config);
        assert_eq!(app.active_panel, PanelId::Images);
    }

    #[test]
    fn number_7_focuses_networks() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('7')), &mut app, &config);
        assert_eq!(app.active_panel, PanelId::Networks);
    }

    #[test]
    fn tab_cycles_panels() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        let initial = app.active_panel;
        dashboard_handle_key(key(KeyCode::Tab), &mut app, &config);
        assert_ne!(app.active_panel, initial);
    }

    #[test]
    fn backtab_cycles_panels_backward() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        // Start at Containers (4), go backward to Status (3)
        dashboard_handle_key(key(KeyCode::BackTab), &mut app, &config);
        assert_eq!(app.active_panel, PanelId::Status);
    }

    // ── j/k navigation ──────────────────────────────────────────────

    #[test]
    fn j_navigates_down() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        app.containers = vec![make_container("a"), make_container("b")];
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        assert_eq!(app.selected_index(), 1);
    }

    #[test]
    fn k_navigates_up() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        app.containers = vec![make_container("a"), make_container("b")];
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        dashboard_handle_key(key(KeyCode::Char('k')), &mut app, &config);
        assert_eq!(app.selected_index(), 0);
    }

    #[test]
    fn j_on_empty_list_does_not_crash() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        // Should not panic
        assert_eq!(app.selected_index(), 0);
    }

    // ── Popup mode ───────────────────────────────────────────────────

    #[test]
    fn popup_q_closes() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('q')), &mut app, &config);
        assert!(!app.show_popup);
    }

    #[test]
    fn popup_esc_closes() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert!(!app.show_popup);
    }

    #[test]
    fn popup_j_scrolls_down() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["a".into(), "b".into(), "c".into()]);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        assert_eq!(app.popup_scroll, 1);
    }

    #[test]
    fn popup_scroll_capped() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["a".into(), "b".into()]);
        let config = DockctlConfig::default();
        for _ in 0..100 {
            dashboard_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        }
        assert!(app.popup_scroll <= 1);
    }

    #[test]
    fn popup_k_scrolls_up() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["a".into(), "b".into()]);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        dashboard_handle_key(key(KeyCode::Char('k')), &mut app, &config);
        assert_eq!(app.popup_scroll, 0);
    }

    #[test]
    fn popup_unhandled_key_consumed() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('z')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
        assert!(app.show_popup); // still open
    }

    // ── Log view ─────────────────────────────────────────────────────

    #[test]
    fn log_view_esc_closes() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert!(!app.log_view_active);
    }

    #[test]
    fn log_view_q_closes() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('q')), &mut app, &config);
        assert!(!app.log_view_active);
    }

    #[test]
    fn log_view_f_toggles_follow() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        assert!(app.log_view_follow);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('f')), &mut app, &config);
        assert!(!app.log_view_follow);
        dashboard_handle_key(key(KeyCode::Char('f')), &mut app, &config);
        assert!(app.log_view_follow);
    }

    #[test]
    fn log_view_number_returns_refresh() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('1')), &mut app, &config);
        assert!(matches!(result, ThemeAction::RefreshLogs { tail: 50, .. }));
    }

    #[test]
    fn log_view_g_jumps_to_top() {
        let mut app = App::new(false);
        app.open_log_view(
            "id".into(),
            "name".into(),
            vec!["a".into(), "b".into(), "c".into()],
        );
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('g')), &mut app, &config);
        assert_eq!(app.log_view_scroll, 0);
        assert!(!app.log_view_follow);
    }

    #[test]
    fn log_view_scroll_capped() {
        let mut app = App::new(false);
        app.open_log_view(
            "id".into(),
            "name".into(),
            vec!["a".into(), "b".into(), "c".into()],
        );
        app.log_view_follow = false;
        app.log_view_scroll = 0;
        let config = DockctlConfig::default();
        for _ in 0..100 {
            dashboard_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        }
        assert!(app.log_view_scroll <= 2);
    }

    // ── Detail view ──────────────────────────────────────────────────

    #[test]
    fn detail_view_esc_closes() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert!(!app.detail_view_active);
    }

    #[test]
    fn detail_view_q_closes() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('q')), &mut app, &config);
        assert!(!app.detail_view_active);
    }

    #[test]
    fn detail_view_j_scrolls_down() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["a".into(), "b".into(), "c".into()]);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        assert_eq!(app.detail_view_scroll, 1);
    }

    #[test]
    fn detail_view_g_jumps_to_top() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["a".into(), "b".into(), "c".into()]);
        app.detail_view_scroll = 2;
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('g')), &mut app, &config);
        assert_eq!(app.detail_view_scroll, 0);
    }

    #[test]
    fn detail_view_shift_g_jumps_to_bottom() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["a".into(), "b".into(), "c".into()]);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('G')), &mut app, &config);
        assert_eq!(app.detail_view_scroll, 2);
    }

    // ── Search mode ──────────────────────────────────────────────────

    #[test]
    fn slash_enters_search_mode() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('/')), &mut app, &config);
        assert_eq!(app.input_mode, InputMode::Search);
    }

    #[test]
    fn search_mode_esc_returns_to_normal() {
        let mut app = App::new(false);
        app.input_mode = InputMode::Search;
        app.search_query = "test".into();
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn search_mode_typing() {
        let mut app = App::new(false);
        app.input_mode = InputMode::Search;
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('n')), &mut app, &config);
        dashboard_handle_key(key(KeyCode::Char('g')), &mut app, &config);
        assert_eq!(app.search_query, "ng");
    }

    #[test]
    fn search_mode_enter_applies() {
        let mut app = App::new(false);
        app.input_mode = InputMode::Search;
        app.search_query = "test".into();
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Enter), &mut app, &config);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.search_query, "test"); // preserved
    }

    #[test]
    fn search_mode_backspace() {
        let mut app = App::new(false);
        app.input_mode = InputMode::Search;
        app.search_query = "abc".into();
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Backspace), &mut app, &config);
        assert_eq!(app.search_query, "ab");
    }

    // ── ? opens help popup ───────────────────────────────────────────

    #[test]
    fn help_opens_popup() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('?')), &mut app, &config);
        assert!(matches!(result, ThemeAction::OpenPopup { .. }));
    }

    // ── Container actions ────────────────────────────────────────────

    #[test]
    fn s_on_containers_returns_stop() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('s')), &mut app, &config);
        assert!(matches!(result, ThemeAction::StopContainer { id } if id == "c1"));
    }

    #[test]
    fn s_on_containers_consumed_when_empty() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('s')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
    }

    #[test]
    fn r_on_containers_returns_restart() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('r')), &mut app, &config);
        assert!(matches!(result, ThemeAction::RestartContainer { id } if id == "c1"));
    }

    #[test]
    fn x_on_containers_returns_remove() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('x')), &mut app, &config);
        assert!(matches!(result, ThemeAction::RemoveContainer { id } if id == "c1"));
    }

    #[test]
    fn l_on_containers_returns_fetch_logs() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('l')), &mut app, &config);
        assert!(matches!(result, ThemeAction::FetchLogs { .. }));
    }

    #[test]
    fn shift_e_on_containers_returns_exec() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('E')), &mut app, &config);
        assert!(matches!(result, ThemeAction::ExecContainer { id } if id == "c1"));
    }

    #[test]
    fn s_on_non_container_panel_not_handled() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Images;
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('s')), &mut app, &config);
        assert!(matches!(result, ThemeAction::NotHandled));
    }

    // ── Image actions ────────────────────────────────────────────────

    #[test]
    fn x_on_images_returns_remove_image() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Images;
        app.images = vec![crate::runtime::ImageSummary {
            id: "img1".into(),
            repo_tags: vec!["nginx:latest".into()],
            repo_digests: vec![],
            size: 100_000,
            created: 1700000000,
            labels: std::collections::HashMap::new(),
        }];
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('x')), &mut app, &config);
        assert!(matches!(result, ThemeAction::RemoveImage { id, force: false } if id == "img1"));
    }

    #[test]
    fn x_on_images_consumed_when_empty() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Images;
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('x')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
    }

    #[test]
    fn p_on_images_enters_pull_input() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Images;
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('p')), &mut app, &config);
        assert_eq!(app.input_mode, InputMode::PullInput);
        assert!(app.pull_input.is_empty());
    }

    #[test]
    fn p_on_non_images_not_pull() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('p')), &mut app, &config);
        assert!(matches!(result, ThemeAction::NotHandled));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn pull_input_typing_and_submit() {
        let mut app = App::new(false);
        app.input_mode = InputMode::PullInput;
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Char('n')), &mut app, &config);
        dashboard_handle_key(key(KeyCode::Char('g')), &mut app, &config);
        assert_eq!(app.pull_input, "ng");
        let result = dashboard_handle_key(key(KeyCode::Enter), &mut app, &config);
        assert!(matches!(result, ThemeAction::PullImage { reference } if reference == "ng"));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn pull_input_esc_cancels() {
        let mut app = App::new(false);
        app.input_mode = InputMode::PullInput;
        app.pull_input = "nginx".into();
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.pull_input.is_empty());
    }

    #[test]
    fn pull_input_empty_submit_consumed() {
        let mut app = App::new(false);
        app.input_mode = InputMode::PullInput;
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Enter), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
    }

    #[test]
    fn pull_input_backspace() {
        let mut app = App::new(false);
        app.input_mode = InputMode::PullInput;
        app.pull_input = "abc".into();
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Backspace), &mut app, &config);
        assert_eq!(app.pull_input, "ab");
    }

    #[test]
    fn x_on_containers_still_returns_remove_container() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Containers;
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('x')), &mut app, &config);
        // Should be RemoveContainer, not RemoveImage
        assert!(matches!(result, ThemeAction::RemoveContainer { .. }));
    }

    // ── Explain ──────────────────────────────────────────────────────

    #[test]
    fn e_returns_explain() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('e')), &mut app, &config);
        assert!(matches!(
            result,
            ThemeAction::ExplainItem { use_inline: false }
        ));
    }

    #[test]
    fn enter_returns_explain() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Enter), &mut app, &config);
        assert!(matches!(
            result,
            ThemeAction::ExplainItem { use_inline: false }
        ));
    }

    // ── Quit ─────────────────────────────────────────────────────────

    #[test]
    fn q_quits() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Char('q')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Quit));
    }

    #[test]
    fn ctrl_c_quits() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key_ctrl(KeyCode::Char('c')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Quit));
    }

    // ── Esc in normal mode ───────────────────────────────────────────

    #[test]
    fn esc_clears_search_query() {
        let mut app = App::new(false);
        app.search_query = "test".into();
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert!(app.search_query.is_empty());
    }

    // ── Column scrolling ─────────────────────────────────────────────

    #[test]
    fn right_arrow_scrolls_columns_on_right_panel() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Images;
        let config = DockctlConfig::default();
        dashboard_handle_key(key(KeyCode::Right), &mut app, &config);
        assert!(app.panel_column_offset(PanelId::Images) > 0);
    }

    #[test]
    fn right_arrow_not_handled_on_left_panel() {
        let mut app = App::new(false);
        app.active_panel = PanelId::Alerts;
        let config = DockctlConfig::default();
        let result = dashboard_handle_key(key(KeyCode::Right), &mut app, &config);
        assert!(matches!(result, ThemeAction::NotHandled));
    }
}
