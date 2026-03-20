use crate::app::{App, PanelId};
use crate::themes::ThemeAction;

/// All available commands for autocomplete.
pub(super) const COMMANDS: &[&str] = &[
    "containers",
    "images",
    "volumes",
    "networks",
    "status",
    "q",
    "quit",
    "exit",
];

/// Find the best autocomplete suggestion for the current input.
pub(super) fn autocomplete_suggestion(input: &str) -> Option<&'static str> {
    if input.is_empty() {
        return None;
    }
    let input_lower = input.to_lowercase();
    // Exact prefix match — return the first matching command
    COMMANDS
        .iter()
        .find(|cmd| cmd.starts_with(&input_lower))
        .copied()
}

/// Execute a : command — switch resource view (with history stack).
/// Execute a : command. Returns Quit if :q or :quit.
pub(super) fn execute_command(app: &mut App, cmd: &str) -> ThemeAction {
    let cmd = cmd.trim().to_lowercase();

    // Quit commands
    if cmd == "q" || cmd == "quit" || cmd == "exit" {
        return ThemeAction::Quit;
    }

    // Resource switching
    let resource = fuzzy_match_resource(&cmd);
    if let Some(res) = resource {
        app.push_view(res);
        match res {
            "containers" => app.focus_panel(PanelId::Containers),
            "images" => app.focus_panel(PanelId::Images),
            "volumes" => app.focus_panel(PanelId::Volumes),
            "networks" => app.focus_panel(PanelId::Networks),
            _ => {}
        }
    }
    ThemeAction::Consumed
}

/// Fuzzy match a command input to a resource name.
pub(super) fn fuzzy_match_resource(input: &str) -> Option<&'static str> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autocomplete_empty_input() {
        assert_eq!(autocomplete_suggestion(""), None);
    }

    #[test]
    fn autocomplete_prefix_match() {
        assert_eq!(autocomplete_suggestion("c"), Some("containers"));
        assert_eq!(autocomplete_suggestion("con"), Some("containers"));
        assert_eq!(autocomplete_suggestion("containers"), Some("containers"));
    }

    #[test]
    fn autocomplete_other_resources() {
        assert_eq!(autocomplete_suggestion("i"), Some("images"));
        assert_eq!(autocomplete_suggestion("v"), Some("volumes"));
        assert_eq!(autocomplete_suggestion("n"), Some("networks"));
        assert_eq!(autocomplete_suggestion("s"), Some("status"));
        assert_eq!(autocomplete_suggestion("q"), Some("q"));
    }

    #[test]
    fn autocomplete_no_match() {
        assert_eq!(autocomplete_suggestion("xyz"), None);
        assert_eq!(autocomplete_suggestion("zzzz"), None);
    }

    #[test]
    fn fuzzy_match_exact() {
        assert_eq!(fuzzy_match_resource("containers"), Some("containers"));
        assert_eq!(fuzzy_match_resource("images"), Some("images"));
        assert_eq!(fuzzy_match_resource("volumes"), Some("volumes"));
        assert_eq!(fuzzy_match_resource("networks"), Some("networks"));
        assert_eq!(fuzzy_match_resource("status"), Some("status"));
    }

    #[test]
    fn fuzzy_match_abbreviations() {
        assert_eq!(fuzzy_match_resource("c"), Some("containers"));
        assert_eq!(fuzzy_match_resource("con"), Some("containers"));
        assert_eq!(fuzzy_match_resource("cnt"), Some("containers"));
        assert_eq!(fuzzy_match_resource("img"), Some("images"));
        assert_eq!(fuzzy_match_resource("im"), Some("images"));
        assert_eq!(fuzzy_match_resource("i"), Some("images"));
        assert_eq!(fuzzy_match_resource("vol"), Some("volumes"));
        assert_eq!(fuzzy_match_resource("v"), Some("volumes"));
        assert_eq!(fuzzy_match_resource("net"), Some("networks"));
        assert_eq!(fuzzy_match_resource("n"), Some("networks"));
        assert_eq!(fuzzy_match_resource("pu"), Some("status"));
        assert_eq!(fuzzy_match_resource("st"), Some("status"));
    }

    #[test]
    fn fuzzy_match_no_match() {
        assert_eq!(fuzzy_match_resource("xyz"), None);
        assert_eq!(fuzzy_match_resource(""), None);
    }

    #[test]
    fn execute_command_quit() {
        let mut app = crate::app::App::new(false);
        let result = execute_command(&mut app, "q");
        assert!(matches!(result, ThemeAction::Quit));

        let result = execute_command(&mut app, "quit");
        assert!(matches!(result, ThemeAction::Quit));

        let result = execute_command(&mut app, "exit");
        assert!(matches!(result, ThemeAction::Quit));
    }

    #[test]
    fn execute_command_switch_resource() {
        let mut app = crate::app::App::new(false);
        let result = execute_command(&mut app, "images");
        assert!(matches!(result, ThemeAction::Consumed));
        assert_eq!(app.operator_resource, "images");
    }

    #[test]
    fn execute_command_invalid() {
        let mut app = crate::app::App::new(false);
        let original = app.operator_resource.clone();
        let result = execute_command(&mut app, "nonexistent");
        assert!(matches!(result, ThemeAction::Consumed));
        assert_eq!(app.operator_resource, original); // unchanged
    }
}
