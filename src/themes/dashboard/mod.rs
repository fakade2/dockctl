use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;

use crate::app::{App, PanelId};
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
/// Handles navigation and UI-related keys. Container actions and async
/// operations return `ThemeAction::NotHandled` so the main loop can handle them.
fn dashboard_handle_key(key: KeyEvent, app: &mut App, config: &DockctlConfig) -> ThemeAction {
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
        (KeyCode::Right | KeyCode::Char('l'), _) if app.active_panel.is_right() => {
            let max_cols = match app.active_panel {
                PanelId::Containers => config.columns.containers.len(),
                PanelId::Images => config.columns.images.len(),
                PanelId::Volumes => config.columns.volumes.len(),
                PanelId::Networks => config.columns.networks.len(),
                _ => 0,
            };
            app.scroll_columns_right(max_cols);
            ThemeAction::Consumed
        }
        (KeyCode::Left | KeyCode::Char('h'), _) if app.active_panel.is_right() => {
            app.scroll_columns_left();
            ThemeAction::Consumed
        }

        // Esc in normal mode — clear search
        (KeyCode::Esc, _) => {
            if !app.search_query.is_empty() {
                app.search_query.clear();
            }
            ThemeAction::Consumed
        }

        // Keybindings help
        (KeyCode::Char('?'), _) => ThemeAction::OpenPopup {
            title: "Keybindings".to_string(),
            content: vec![
                "Navigation".to_string(),
                "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
                "1-7          Focus panel".to_string(),
                "Tab          Next panel".to_string(),
                "Shift+Tab    Previous panel".to_string(),
                "j / Down     Move down".to_string(),
                "k / Up       Move up".to_string(),
                "h / Left     Scroll columns left".to_string(),
                "l / Right    Scroll columns right".to_string(),
                String::new(),
                "Actions".to_string(),
                "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
                "e / Enter    Explain (detail popup)".to_string(),
                "q            Quit (or close popup)".to_string(),
                "/            Search".to_string(),
                "?            This help".to_string(),
                String::new(),
                "Container Actions (panel 4)".to_string(),
                "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
                "s            Stop container".to_string(),
                "r            Restart container".to_string(),
                "x            Remove container".to_string(),
                "l            View logs".to_string(),
                "E            Exec into container (tmux)".to_string(),
            ],
        },

        // Everything else — not handled (main loop processes container actions, explain, search, etc.)
        _ => ThemeAction::NotHandled,
    }
}
