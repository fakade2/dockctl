pub mod components;
pub mod dashboard;
pub mod operator;

use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::app::App;
use crate::config::DockctlConfig;

/// Action returned by a theme's key handler.
pub enum ThemeAction {
    /// Key was consumed by the theme.
    Consumed,
    /// Key was not handled by the theme — engine ignores it.
    NotHandled,
    /// Theme wants to quit.
    Quit,
    /// Theme wants to open explain popup with these lines.
    OpenPopup { title: String, content: Vec<String> },
    /// Theme wants to fetch logs for a container.
    FetchLogs {
        id: String,
        name: String,
        tail: usize,
    },
    /// Theme wants to fetch detailed inspect data for a container.
    FetchDetail { id: String },
    /// Theme wants to refresh logs with a different tail count (number keys).
    RefreshLogs { id: String, tail: usize },
    /// Theme wants to stop a container.
    StopContainer { id: String },
    /// Theme wants to restart a container.
    RestartContainer { id: String },
    /// Theme wants to remove a container.
    RemoveContainer { id: String },
    /// Theme wants to exec into a container via tmux.
    ExecContainer { id: String },
    /// Theme wants to open the explain/detail view for the current selection.
    ExplainItem { use_inline: bool },
    /// Theme wants to run `docker compose up -d` in a working directory.
    ComposeUp { working_dir: String },
    /// Theme wants to run `docker compose down` in a working directory.
    ComposeDown { working_dir: String },
    /// Theme wants to run `docker compose restart` in a working directory.
    ComposeRestart { working_dir: String },
    /// Theme wants to pull an image by reference (e.g. "nginx:latest").
    PullImage { reference: String },
    /// Theme wants to remove an image by ID.
    RemoveImage { id: String, force: bool },
}

/// Trait that all themes must implement.
pub trait Theme: Send + Sync {
    /// Display name of the theme.
    fn name(&self) -> &str;

    /// Render the full UI frame.
    fn draw(&self, frame: &mut Frame, app: &App, config: &DockctlConfig);

    /// Handle a key event. Return ThemeAction to indicate what happened.
    fn handle_key(&self, key: KeyEvent, app: &mut App, config: &DockctlConfig) -> ThemeAction;
}

/// Create a theme instance by name.
pub fn create_theme(name: &str) -> Box<dyn Theme> {
    match name {
        "operator" => Box::new(operator::OperatorTheme::new()),
        _ => Box::new(dashboard::DashboardTheme::new()), // "dashboard" is the default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_dashboard_theme() {
        let theme = create_theme("dashboard");
        assert_eq!(theme.name(), "dashboard");
    }

    #[test]
    fn create_operator_theme() {
        let theme = create_theme("operator");
        assert_eq!(theme.name(), "operator");
    }

    #[test]
    fn unknown_theme_defaults_to_dashboard() {
        let theme = create_theme("nonexistent");
        assert_eq!(theme.name(), "dashboard");
    }

    #[test]
    fn empty_theme_defaults_to_dashboard() {
        let theme = create_theme("");
        assert_eq!(theme.name(), "dashboard");
    }
}
