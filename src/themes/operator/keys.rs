use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
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
