use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;

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
        // Delegate to the existing lazygit-style UI rendering.
        crate::ui::draw(frame, app, config);
    }

    fn handle_key(&self, key: KeyEvent, app: &mut App, config: &DockctlConfig) -> ThemeAction {
        dashboard_handle_key(key, app, config)
    }
}

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
                    .map(|c| crate::ui::container_display_name(&c.names))
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
