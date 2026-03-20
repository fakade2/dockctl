use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, PanelId};
use crate::config::DockctlConfig;
use crate::themes::ThemeAction;

use super::commands::{autocomplete_suggestion, execute_command};
use super::detail_view::describe_selected;
use super::helpers::{current_columns, help_lines, operator_max_cols};

pub(super) fn operator_handle_key(
    key: KeyEvent,
    app: &mut App,
    config: &DockctlConfig,
) -> ThemeAction {
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
            KeyCode::Char(':') => {
                app.operator_command_mode = true;
                app.operator_command_input.clear();
                ThemeAction::Consumed
            }
            KeyCode::Char('/') => {
                app.operator_filter_active = true;
                app.operator_filter.clear();
                ThemeAction::Consumed
            }
            _ => ThemeAction::Consumed,
        };
    }

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
        // Only Ctrl+C quits directly. Use :q or :quit in command mode.
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => ThemeAction::Quit,

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
                Some(col) => columns
                    .iter()
                    .position(|c| c == col)
                    .map(|i| (i + 1) % columns.len())
                    .unwrap_or(0),
                None => 0,
            };
            if let Some(col) = columns.get(next_idx) {
                app.toggle_sort(col);
            }
            ThemeAction::Consumed
        }

        // Column reorder (Shift+Arrow) — consumed but no-op; use config.toml
        (KeyCode::Left, KeyModifiers::SHIFT) => ThemeAction::Consumed,
        (KeyCode::Right, KeyModifiers::SHIFT) => ThemeAction::Consumed,

        // Column scrolling — l scrolls right UNLESS on containers (l = logs there)
        (KeyCode::Right, _) => {
            let max_cols = operator_max_cols(app, config);
            app.scroll_columns_right(max_cols);
            ThemeAction::Consumed
        }
        (KeyCode::Char('l'), _) if app.operator_resource != "containers" => {
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

        // Enter on compose — drill into project or explain service
        (KeyCode::Enter, _) if app.operator_resource == "compose" => {
            if app.compose_selected_project.is_none() {
                // Drill into project
                if let Some(project) = app.compose_projects.get(app.selected_index()) {
                    app.compose_selected_project = Some(project.name.clone());
                    let sel = app.selection.entry(PanelId::Containers).or_insert(0);
                    *sel = 0;
                    ThemeAction::Consumed
                } else {
                    ThemeAction::Consumed
                }
            } else {
                // At service level — explain/describe
                ThemeAction::ExplainItem { use_inline: true }
            }
        }

        // Explain / Enter — open inline detail view
        (KeyCode::Char('e') | KeyCode::Enter, _) => ThemeAction::ExplainItem { use_inline: true },

        // Wide mode toggle
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
            app.toggle_wide_mode();
            ThemeAction::Consumed
        }

        // Esc — back navigation / enter command mode
        (KeyCode::Esc, _) => {
            if app.detail_view_active {
                app.close_detail_view();
                ThemeAction::Consumed
            } else if app.operator_filter_active {
                app.operator_filter_active = false;
                app.operator_filter.clear();
                ThemeAction::Consumed
            } else if !app.operator_filter.is_empty() {
                app.operator_filter.clear();
                ThemeAction::Consumed
            } else if app.operator_resource == "compose" && app.compose_selected_project.is_some() {
                app.compose_selected_project = None;
                let sel = app.selection.entry(PanelId::Containers).or_insert(0);
                *sel = 0;
                ThemeAction::Consumed
            } else {
                // Enter command mode (vim-style)
                app.operator_command_mode = true;
                app.operator_command_input.clear();
                ThemeAction::Consumed
            }
        }

        // Show all resource aliases — inline detail view
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
            let sep = "─────────────────────────────────";
            app.open_detail_view(
                "Resource Aliases".to_string(),
                vec![
                    "Resource Commands".into(),
                    sep.into(),
                    ":compose     :comp :dc          Compose projects".into(),
                    ":containers  :con  :cnt  :c    Container list".into(),
                    ":images      :img  :im   :i    Image list".into(),
                    ":volumes     :vol  :v           Volume list".into(),
                    ":networks    :net  :n           Network list".into(),
                    ":status      :pu               System pulses/dashboard".into(),
                    ":q           :quit  :exit       Quit dockctl".into(),
                    String::new(),
                    "Navigation".into(),
                    sep.into(),
                    "Esc          Command mode / go back".into(),
                    "Ctrl+A       This alias list".into(),
                ],
            );
            ThemeAction::Consumed
        }

        // Help — opens inline detail view (not popup)
        (KeyCode::Char('?'), _) => {
            app.open_detail_view("Keybindings".to_string(), help_lines());
            ThemeAction::Consumed
        }

        // ── Compose actions (only when viewing compose, project level) ─────
        (KeyCode::Char('u'), _)
            if app.operator_resource == "compose" && app.compose_selected_project.is_none() =>
        {
            if let Some(project) = app.compose_projects.get(app.selected_index()) {
                if !project.working_dir.is_empty() {
                    ThemeAction::ComposeUp {
                        working_dir: project.working_dir.clone(),
                    }
                } else {
                    ThemeAction::Consumed
                }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('x'), _)
            if app.operator_resource == "compose" && app.compose_selected_project.is_none() =>
        {
            if let Some(project) = app.compose_projects.get(app.selected_index()) {
                if !project.working_dir.is_empty() {
                    ThemeAction::ComposeDown {
                        working_dir: project.working_dir.clone(),
                    }
                } else {
                    ThemeAction::Consumed
                }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('r'), _)
            if app.operator_resource == "compose" && app.compose_selected_project.is_none() =>
        {
            if let Some(project) = app.compose_projects.get(app.selected_index()) {
                if !project.working_dir.is_empty() {
                    ThemeAction::ComposeRestart {
                        working_dir: project.working_dir.clone(),
                    }
                } else {
                    ThemeAction::Consumed
                }
            } else {
                ThemeAction::Consumed
            }
        }

        // ── Compose service-level actions ─────
        (KeyCode::Char('s'), _)
            if app.operator_resource == "compose" && app.compose_selected_project.is_some() =>
        {
            // Stop selected service container
            let project_name = app.compose_selected_project.as_ref().unwrap();
            if let Some(project) = app
                .compose_projects
                .iter()
                .find(|p| &p.name == project_name)
            {
                if let Some(svc) = project.services.get(app.selected_index()) {
                    ThemeAction::StopContainer {
                        id: svc.container_id.clone(),
                    }
                } else {
                    ThemeAction::Consumed
                }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('r'), _)
            if app.operator_resource == "compose" && app.compose_selected_project.is_some() =>
        {
            // Restart selected service container
            let project_name = app.compose_selected_project.as_ref().unwrap();
            if let Some(project) = app
                .compose_projects
                .iter()
                .find(|p| &p.name == project_name)
            {
                if let Some(svc) = project.services.get(app.selected_index()) {
                    ThemeAction::RestartContainer {
                        id: svc.container_id.clone(),
                    }
                } else {
                    ThemeAction::Consumed
                }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('l'), _)
            if app.operator_resource == "compose" && app.compose_selected_project.is_some() =>
        {
            // Fetch logs for selected service container
            let project_name = app.compose_selected_project.as_ref().unwrap();
            if let Some(project) = app
                .compose_projects
                .iter()
                .find(|p| &p.name == project_name)
            {
                if let Some(svc) = project.services.get(app.selected_index()) {
                    ThemeAction::FetchLogs {
                        id: svc.container_id.clone(),
                        name: svc.name.clone(),
                        tail: 500,
                    }
                } else {
                    ThemeAction::Consumed
                }
            } else {
                ThemeAction::Consumed
            }
        }

        // ── Container actions (only when viewing containers) ─────
        (KeyCode::Char('s'), _) if app.operator_resource == "containers" => {
            if let Some(id) = app.selected_container_id() {
                ThemeAction::StopContainer { id }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('r'), _) if app.operator_resource == "containers" => {
            if let Some(id) = app.selected_container_id() {
                ThemeAction::RestartContainer { id }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('x'), _) if app.operator_resource == "containers" => {
            if let Some(id) = app.selected_container_id() {
                ThemeAction::RemoveContainer { id }
            } else {
                ThemeAction::Consumed
            }
        }
        (KeyCode::Char('l'), _) if app.operator_resource == "containers" => {
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
        (KeyCode::Char('E'), _) if app.operator_resource == "containers" => {
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
            execute_command(app, &cmd)
        }
        // Tab — accept autocomplete suggestion
        KeyCode::Tab => {
            if let Some(suggestion) = autocomplete_suggestion(&app.operator_command_input) {
                app.operator_command_input = suggestion.to_string();
            }
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

    fn key_shift(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SHIFT)
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

    // ── Popup mode ───────────────────────────────────────────────────

    #[test]
    fn popup_esc_closes() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
        assert!(!app.show_popup);
    }

    #[test]
    fn popup_q_closes() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('q')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
        assert!(!app.show_popup);
    }

    #[test]
    fn popup_scroll_capped() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["a".into(), "b".into()]);
        let config = DockctlConfig::default();
        for _ in 0..100 {
            operator_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        }
        assert!(app.popup_scroll <= 1); // max = content.len() - 1
    }

    #[test]
    fn popup_scroll_up_does_not_underflow() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["a".into(), "b".into()]);
        let config = DockctlConfig::default();
        for _ in 0..100 {
            operator_handle_key(key(KeyCode::Char('k')), &mut app, &config);
        }
        assert_eq!(app.popup_scroll, 0);
    }

    #[test]
    fn popup_unhandled_key_consumed() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('z')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
        assert!(app.show_popup); // still open
    }

    // ── Log view mode ────────────────────────────────────────────────

    #[test]
    fn log_follow_toggle() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        assert!(app.log_view_follow);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('f')), &mut app, &config);
        assert!(!app.log_view_follow);
        operator_handle_key(key(KeyCode::Char('f')), &mut app, &config);
        assert!(app.log_view_follow);
    }

    #[test]
    fn log_scroll_capped() {
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
            operator_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        }
        assert!(app.log_view_scroll <= 2);
    }

    #[test]
    fn log_esc_closes() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert!(!app.log_view_active);
    }

    #[test]
    fn log_q_closes() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('q')), &mut app, &config);
        assert!(!app.log_view_active);
    }

    #[test]
    fn log_number_key_returns_refresh() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('1')), &mut app, &config);
        assert!(matches!(result, ThemeAction::RefreshLogs { tail: 50, .. }));
    }

    #[test]
    fn log_number_key_2_returns_100() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('2')), &mut app, &config);
        assert!(matches!(result, ThemeAction::RefreshLogs { tail: 100, .. }));
    }

    #[test]
    fn log_g_jumps_to_top() {
        let mut app = App::new(false);
        app.open_log_view(
            "id".into(),
            "name".into(),
            vec!["a".into(), "b".into(), "c".into()],
        );
        // scroll is at bottom (2) from open_log_view
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('g')), &mut app, &config);
        assert_eq!(app.log_view_scroll, 0);
        assert!(!app.log_view_follow);
    }

    #[test]
    fn log_shift_g_jumps_to_bottom() {
        let mut app = App::new(false);
        app.open_log_view(
            "id".into(),
            "name".into(),
            vec!["a".into(), "b".into(), "c".into()],
        );
        app.log_view_follow = false;
        app.log_view_scroll = 0;
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('G')), &mut app, &config);
        assert_eq!(app.log_view_scroll, 2);
        assert!(app.log_view_follow);
    }

    #[test]
    fn log_page_down_scrolls_by_20() {
        let mut app = App::new(false);
        let lines: Vec<String> = (0..50).map(|i| format!("line {i}")).collect();
        app.open_log_view("id".into(), "name".into(), lines);
        app.log_view_follow = false;
        app.log_view_scroll = 0;
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::PageDown), &mut app, &config);
        assert_eq!(app.log_view_scroll, 20);
    }

    #[test]
    fn log_colon_enters_command_mode() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char(':')), &mut app, &config);
        assert!(app.operator_command_mode);
    }

    #[test]
    fn log_slash_enters_filter_mode() {
        let mut app = App::new(false);
        app.open_log_view("id".into(), "name".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('/')), &mut app, &config);
        assert!(app.operator_filter_active);
    }

    // ── Detail view mode ─────────────────────────────────────────────

    #[test]
    fn detail_esc_closes() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert!(!app.detail_view_active);
    }

    #[test]
    fn detail_q_closes() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["line".into()]);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('q')), &mut app, &config);
        assert!(!app.detail_view_active);
    }

    #[test]
    fn detail_j_scrolls_down() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["a".into(), "b".into(), "c".into()]);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        assert_eq!(app.detail_view_scroll, 1);
    }

    #[test]
    fn detail_g_jumps_to_top() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["a".into(), "b".into(), "c".into()]);
        app.detail_view_scroll = 2;
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('g')), &mut app, &config);
        assert_eq!(app.detail_view_scroll, 0);
    }

    #[test]
    fn detail_shift_g_jumps_to_bottom() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["a".into(), "b".into(), "c".into()]);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('G')), &mut app, &config);
        assert_eq!(app.detail_view_scroll, 2); // len=3, saturating_sub(1)=2
    }

    // ── Command mode ─────────────────────────────────────────────────

    #[test]
    fn colon_enters_command_mode() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char(':')), &mut app, &config);
        assert!(app.operator_command_mode);
    }

    #[test]
    fn esc_enters_command_mode_when_normal() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert!(app.operator_command_mode);
    }

    #[test]
    fn command_mode_esc_exits() {
        let mut app = App::new(false);
        app.operator_command_mode = true;
        app.operator_command_input = "con".into();
        let result = handle_command_mode(key(KeyCode::Esc), &mut app);
        assert!(matches!(result, ThemeAction::Consumed));
        assert!(!app.operator_command_mode);
        assert!(app.operator_command_input.is_empty());
    }

    #[test]
    fn command_mode_typing() {
        let mut app = App::new(false);
        app.operator_command_mode = true;
        handle_command_mode(key(KeyCode::Char('c')), &mut app);
        handle_command_mode(key(KeyCode::Char('o')), &mut app);
        handle_command_mode(key(KeyCode::Char('n')), &mut app);
        assert_eq!(app.operator_command_input, "con");
    }

    #[test]
    fn command_mode_backspace() {
        let mut app = App::new(false);
        app.operator_command_mode = true;
        app.operator_command_input = "con".into();
        handle_command_mode(key(KeyCode::Backspace), &mut app);
        assert_eq!(app.operator_command_input, "co");
    }

    // ── Filter mode ──────────────────────────────────────────────────

    #[test]
    fn slash_enters_filter_mode() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('/')), &mut app, &config);
        assert!(app.operator_filter_active);
        assert!(app.operator_filter.is_empty());
    }

    #[test]
    fn filter_mode_typing() {
        let mut app = App::new(false);
        app.operator_filter_active = true;
        handle_filter_mode(key(KeyCode::Char('n')), &mut app);
        handle_filter_mode(key(KeyCode::Char('g')), &mut app);
        assert_eq!(app.operator_filter, "ng");
    }

    #[test]
    fn filter_mode_esc_clears() {
        let mut app = App::new(false);
        app.operator_filter_active = true;
        app.operator_filter = "test".into();
        handle_filter_mode(key(KeyCode::Esc), &mut app);
        assert!(!app.operator_filter_active);
        assert!(app.operator_filter.is_empty());
    }

    #[test]
    fn filter_mode_enter_keeps_filter() {
        let mut app = App::new(false);
        app.operator_filter_active = true;
        app.operator_filter = "test".into();
        handle_filter_mode(key(KeyCode::Enter), &mut app);
        assert!(!app.operator_filter_active);
        assert_eq!(app.operator_filter, "test"); // preserved
    }

    #[test]
    fn filter_mode_backspace() {
        let mut app = App::new(false);
        app.operator_filter_active = true;
        app.operator_filter = "abc".into();
        handle_filter_mode(key(KeyCode::Backspace), &mut app);
        assert_eq!(app.operator_filter, "ab");
    }

    // ── Container actions ────────────────────────────────────────────

    #[test]
    fn s_returns_stop_on_containers() {
        let mut app = App::new(false);
        app.operator_resource = "containers".into();
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('s')), &mut app, &config);
        assert!(matches!(result, ThemeAction::StopContainer { id } if id == "c1"));
    }

    #[test]
    fn s_consumed_when_no_containers() {
        let mut app = App::new(false);
        app.operator_resource = "containers".into();
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('s')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
    }

    #[test]
    fn r_returns_restart_on_containers() {
        let mut app = App::new(false);
        app.operator_resource = "containers".into();
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('r')), &mut app, &config);
        assert!(matches!(result, ThemeAction::RestartContainer { id } if id == "c1"));
    }

    #[test]
    fn x_returns_remove_on_containers() {
        let mut app = App::new(false);
        app.operator_resource = "containers".into();
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('x')), &mut app, &config);
        assert!(matches!(result, ThemeAction::RemoveContainer { id } if id == "c1"));
    }

    #[test]
    fn l_returns_fetch_logs_on_containers() {
        let mut app = App::new(false);
        app.operator_resource = "containers".into();
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('l')), &mut app, &config);
        assert!(matches!(result, ThemeAction::FetchLogs { .. }));
    }

    #[test]
    fn l_consumed_when_empty_containers() {
        let mut app = App::new(false);
        app.operator_resource = "containers".into();
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('l')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
    }

    #[test]
    fn shift_e_returns_exec_on_containers() {
        let mut app = App::new(false);
        app.operator_resource = "containers".into();
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('E')), &mut app, &config);
        assert!(matches!(result, ThemeAction::ExecContainer { id } if id == "c1"));
    }

    #[test]
    fn s_not_handled_on_non_containers() {
        let mut app = App::new(false);
        app.operator_resource = "images".into();
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('s')), &mut app, &config);
        assert!(matches!(result, ThemeAction::NotHandled));
    }

    // ── Ctrl+C quits ─────────────────────────────────────────────────

    #[test]
    fn ctrl_c_quits() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        let result = operator_handle_key(key_ctrl(KeyCode::Char('c')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Quit));
    }

    // ── Sort keys ────────────────────────────────────────────────────

    #[test]
    fn shift_n_sorts_by_name() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        operator_handle_key(key_shift(KeyCode::Char('N')), &mut app, &config);
        assert_eq!(app.sort_column, Some("Name".into()));
    }

    #[test]
    fn shift_s_sorts_by_state() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        operator_handle_key(key_shift(KeyCode::Char('S')), &mut app, &config);
        assert_eq!(app.sort_column, Some("State".into()));
    }

    #[test]
    fn shift_a_sorts_by_created() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        operator_handle_key(key_shift(KeyCode::Char('A')), &mut app, &config);
        assert_eq!(app.sort_column, Some("Created".into()));
    }

    // ── Wide mode ────────────────────────────────────────────────────

    #[test]
    fn ctrl_w_toggles_wide() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        assert!(!app.wide_mode);
        operator_handle_key(key_ctrl(KeyCode::Char('w')), &mut app, &config);
        assert!(app.wide_mode);
        operator_handle_key(key_ctrl(KeyCode::Char('w')), &mut app, &config);
        assert!(!app.wide_mode);
    }

    // ── ? opens help ─────────────────────────────────────────────────

    #[test]
    fn question_mark_opens_help() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('?')), &mut app, &config);
        assert!(app.detail_view_active);
        assert!(app.detail_view_title.contains("Keybindings"));
    }

    // ── d opens describe ─────────────────────────────────────────────

    #[test]
    fn d_opens_describe() {
        let mut app = App::new(false);
        app.operator_resource = "containers".into();
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('d')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
        assert!(app.detail_view_active);
    }

    #[test]
    fn d_with_container_shows_detail() {
        let mut app = App::new(false);
        app.operator_resource = "containers".into();
        app.containers = vec![make_container("c1")];
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('d')), &mut app, &config);
        assert!(app.detail_view_active);
        assert!(app.detail_view_title.contains("Container"));
    }

    // ── e / Enter opens explain ──────────────────────────────────────

    #[test]
    fn e_returns_explain() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('e')), &mut app, &config);
        assert!(matches!(
            result,
            ThemeAction::ExplainItem { use_inline: true }
        ));
    }

    #[test]
    fn enter_returns_explain() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Enter), &mut app, &config);
        assert!(matches!(
            result,
            ThemeAction::ExplainItem { use_inline: true }
        ));
    }

    // ── Navigation ───────────────────────────────────────────────────

    #[test]
    fn j_moves_down() {
        let mut app = App::new(false);
        app.containers = vec![make_container("a"), make_container("b")];
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        assert_eq!(app.selected_index(), 1);
    }

    #[test]
    fn k_moves_up() {
        let mut app = App::new(false);
        app.containers = vec![make_container("a"), make_container("b")];
        let config = DockctlConfig::default();
        // Move to index 1 first
        operator_handle_key(key(KeyCode::Char('j')), &mut app, &config);
        assert_eq!(app.selected_index(), 1);
        operator_handle_key(key(KeyCode::Char('k')), &mut app, &config);
        assert_eq!(app.selected_index(), 0);
    }

    // ── Esc behavior in normal mode ──────────────────────────────────

    #[test]
    fn esc_clears_filter_when_set() {
        let mut app = App::new(false);
        app.operator_filter = "test".into();
        let config = DockctlConfig::default();
        operator_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert!(app.operator_filter.is_empty());
        assert!(!app.operator_command_mode); // should not enter command mode
    }

    #[test]
    fn esc_exits_filter_active() {
        let mut app = App::new(false);
        app.operator_filter_active = true;
        app.operator_filter = "test".into();
        let config = DockctlConfig::default();
        // This goes through filter_mode handler directly since filter_active is true
        operator_handle_key(key(KeyCode::Esc), &mut app, &config);
        assert!(!app.operator_filter_active);
    }

    // ── Ctrl+A opens alias list ──────────────────────────────────────

    #[test]
    fn ctrl_a_opens_alias_list() {
        let mut app = App::new(false);
        let config = DockctlConfig::default();
        operator_handle_key(key_ctrl(KeyCode::Char('a')), &mut app, &config);
        assert!(app.detail_view_active);
        assert!(app.detail_view_title.contains("Resource Aliases"));
    }

    // ── Column scrolling on non-container resource ───────────────────

    #[test]
    fn l_scrolls_columns_on_images() {
        let mut app = App::new(false);
        app.operator_resource = "images".into();
        let config = DockctlConfig::default();
        let result = operator_handle_key(key(KeyCode::Char('l')), &mut app, &config);
        assert!(matches!(result, ThemeAction::Consumed));
        // column offset should have advanced
        assert!(app.panel_column_offset(app.active_panel) > 0);
    }

    #[test]
    fn h_scrolls_columns_left() {
        let mut app = App::new(false);
        app.operator_resource = "images".into();
        let config = DockctlConfig::default();
        // Scroll right first
        operator_handle_key(key(KeyCode::Char('l')), &mut app, &config);
        let offset_after_right = app.panel_column_offset(app.active_panel);
        operator_handle_key(key(KeyCode::Char('h')), &mut app, &config);
        assert!(app.panel_column_offset(app.active_panel) < offset_after_right);
    }
}
