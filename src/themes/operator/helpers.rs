use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Cell, Row},
};

use crate::app::App;
use crate::config::DockctlConfig;
use crate::display::{container_display_name, short_id};

pub(super) fn state_color(state: &str) -> Color {
    match state {
        "running" => Color::Green,
        "paused" => Color::Yellow,
        "restarting" => Color::Blue,
        "created" => Color::Gray,
        _ => Color::Red,
    }
}

/// Return (dot_char, dot_color) for a container based on its state.
pub(super) fn container_status_dot(state: &str) -> (&'static str, Color) {
    match state {
        "running" => ("\u{25cf}", Color::Green),
        "paused" | "restarting" => ("\u{25cf}", Color::Yellow),
        "created" => ("\u{25cb}", Color::DarkGray),
        "exited" | "dead" => ("\u{25cf}", Color::Red),
        _ => ("\u{25cf}", Color::Red),
    }
}

/// Return (dot_char, dot_color) for an image based on its tags.
pub(super) fn image_status_dot(repo_tags: &[String]) -> (&'static str, Color) {
    let is_none = repo_tags.is_empty() || repo_tags.iter().all(|t| t == "<none>:<none>");
    if is_none {
        ("\u{25cf}", Color::Red)
    } else {
        ("\u{25cf}", Color::Green)
    }
}

/// Return (dot_char, dot_color) for a network based on connected containers.
pub(super) fn network_status_dot(container_count: usize) -> (&'static str, Color) {
    if container_count > 0 {
        ("\u{25cf}", Color::Green)
    } else {
        ("\u{25cf}", Color::DarkGray)
    }
}

pub(super) fn k9s_table_title(
    resource: &str,
    _total_count: usize,
    filtered_count: usize,
    _filter: &str,
    scroll_hint: &str,
) -> String {
    format!(
        " {}({}) {} ",
        capitalize(resource),
        filtered_count,
        scroll_hint,
    )
}

/// Create a table header row with sort indicators. If `sort_column` matches
/// a visible column name, an arrow is prepended to that column's header.
pub(super) fn create_sorted_header(
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

/// Return (total_rows, filtered_rows) for the current resource view.
pub(super) fn row_counts(app: &App) -> (usize, usize) {
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
        "compose" => {
            if let Some(ref project_name) = app.compose_selected_project {
                let total = app
                    .compose_projects
                    .iter()
                    .find(|p| &p.name == project_name)
                    .map(|p| p.services.len())
                    .unwrap_or(0);
                (total, total) // no filtering for now
            } else {
                (app.compose_projects.len(), app.compose_projects.len())
            }
        }
        _ => (0, 0),
    }
}

pub(super) fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

pub(super) fn help_lines() -> Vec<String> {
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
        "Esc          Command mode / go back / clear filter".into(),
        String::new(),
        "Commands (Esc or : to enter command mode)".into(),
        sep.into(),
        ":containers  :images  :volumes  :networks".into(),
        ":status      :pu       Pulses dashboard".into(),
        ":q           :quit     Quit dockctl".into(),
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
        "Compose Actions (in :compose)".into(),
        sep.into(),
        "u            Compose up (project level)".into(),
        "x            Compose down (project level)".into(),
        "r            Restart project/service".into(),
        "Enter        Drill into project".into(),
        "Esc          Back to project list".into(),
        "l            View logs (service level)".into(),
        "s            Stop service (service level)".into(),
        String::new(),
        "General".into(),
        sep.into(),
        "?            This help".into(),
        ":q           Quit dockctl".into(),
        "Ctrl+C       Force quit".into(),
    ]
}

pub(super) fn current_columns<'a>(app: &App, config: &'a DockctlConfig) -> &'a [String] {
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

pub(super) fn operator_max_cols(app: &App, config: &DockctlConfig) -> usize {
    current_columns(app, config).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::runtime::*;
    use std::collections::HashMap;

    #[test]
    fn state_color_running() {
        assert_eq!(state_color("running"), Color::Green);
    }
    #[test]
    fn state_color_paused() {
        assert_eq!(state_color("paused"), Color::Yellow);
    }
    #[test]
    fn state_color_exited() {
        assert_eq!(state_color("exited"), Color::Red);
    }
    #[test]
    fn state_color_created() {
        assert_eq!(state_color("created"), Color::Gray);
    }
    #[test]
    fn state_color_restarting() {
        assert_eq!(state_color("restarting"), Color::Blue);
    }
    #[test]
    fn state_color_unknown() {
        assert_eq!(state_color("dead"), Color::Red);
    }

    #[test]
    fn container_dot_running() {
        let (dot, color) = container_status_dot("running");
        assert_eq!(color, Color::Green);
        assert_eq!(dot, "\u{25cf}");
    }
    #[test]
    fn container_dot_exited() {
        let (_, color) = container_status_dot("exited");
        assert_eq!(color, Color::Red);
    }
    #[test]
    fn container_dot_paused() {
        let (_, color) = container_status_dot("paused");
        assert_eq!(color, Color::Yellow);
    }
    #[test]
    fn container_dot_created() {
        let (dot, color) = container_status_dot("created");
        assert_eq!(color, Color::DarkGray);
        assert_eq!(dot, "\u{25cb}");
    }

    #[test]
    fn image_dot_tagged() {
        let (_, color) = image_status_dot(&["nginx:latest".into()]);
        assert_eq!(color, Color::Green);
    }
    #[test]
    fn image_dot_none() {
        let (_, color) = image_status_dot(&["<none>:<none>".into()]);
        assert_eq!(color, Color::Red);
    }
    #[test]
    fn image_dot_empty() {
        let (_, color) = image_status_dot(&[]);
        assert_eq!(color, Color::Red);
    }

    #[test]
    fn network_dot_with_containers() {
        let (_, color) = network_status_dot(3);
        assert_eq!(color, Color::Green);
    }
    #[test]
    fn network_dot_empty() {
        let (_, color) = network_status_dot(0);
        assert_eq!(color, Color::DarkGray);
    }

    #[test]
    fn capitalize_empty() {
        assert_eq!(capitalize(""), "");
    }
    #[test]
    fn capitalize_word() {
        assert_eq!(capitalize("hello"), "Hello");
    }
    #[test]
    fn capitalize_already() {
        assert_eq!(capitalize("Hello"), "Hello");
    }
    #[test]
    fn capitalize_single_char() {
        assert_eq!(capitalize("x"), "X");
    }

    #[test]
    fn row_counts_empty() {
        let app = App::new(false);
        let (total, filtered) = row_counts(&app);
        assert_eq!(total, 0);
        assert_eq!(filtered, 0);
    }
    #[test]
    fn row_counts_containers() {
        let mut app = App::new(false);
        app.containers = vec![
            ContainerSummary {
                id: "a".into(),
                name: "nginx".into(),
                names: vec!["nginx".into()],
                image: "nginx:latest".into(),
                image_id: "sha256:a".into(),
                command: "/bin/sh".into(),
                state: "running".into(),
                status: "Up".into(),
                created: 0,
                ports: vec![],
                labels: HashMap::new(),
            },
            ContainerSummary {
                id: "b".into(),
                name: "redis".into(),
                names: vec!["redis".into()],
                image: "redis:latest".into(),
                image_id: "sha256:b".into(),
                command: "/bin/sh".into(),
                state: "exited".into(),
                status: "Exited".into(),
                created: 0,
                ports: vec![],
                labels: HashMap::new(),
            },
        ];
        let (total, filtered) = row_counts(&app);
        assert_eq!(total, 2);
        assert_eq!(filtered, 2);
    }
    #[test]
    fn row_counts_with_filter() {
        let mut app = App::new(false);
        app.containers = vec![
            ContainerSummary {
                id: "a".into(),
                name: "nginx".into(),
                names: vec!["nginx".into()],
                image: "nginx:latest".into(),
                image_id: "sha256:a".into(),
                command: "/bin/sh".into(),
                state: "running".into(),
                status: "Up".into(),
                created: 0,
                ports: vec![],
                labels: HashMap::new(),
            },
            ContainerSummary {
                id: "b".into(),
                name: "redis".into(),
                names: vec!["redis".into()],
                image: "redis:latest".into(),
                image_id: "sha256:b".into(),
                command: "/bin/sh".into(),
                state: "exited".into(),
                status: "Exited".into(),
                created: 0,
                ports: vec![],
                labels: HashMap::new(),
            },
        ];
        app.operator_filter = "nginx".into();
        let (total, filtered) = row_counts(&app);
        assert_eq!(total, 2);
        assert_eq!(filtered, 1);
    }
    #[test]
    fn row_counts_images() {
        let mut app = App::new(false);
        app.operator_resource = "images".into();
        app.images = vec![ImageSummary {
            id: "sha256:abc".into(),
            repo_tags: vec!["nginx:latest".into()],
            repo_digests: vec![],
            size: 1000,
            created: 0,
            labels: HashMap::new(),
        }];
        let (total, filtered) = row_counts(&app);
        assert_eq!(total, 1);
        assert_eq!(filtered, 1);
    }
    #[test]
    fn row_counts_volumes() {
        let mut app = App::new(false);
        app.operator_resource = "volumes".into();
        app.volumes = vec![VolumeSummary {
            name: "data".into(),
            driver: "local".into(),
            mountpoint: "/var".into(),
            labels: HashMap::new(),
            scope: "local".into(),
            created_at: "2026-01-01".into(),
        }];
        let (total, filtered) = row_counts(&app);
        assert_eq!(total, 1);
        assert_eq!(filtered, 1);
    }
    #[test]
    fn row_counts_networks() {
        let mut app = App::new(false);
        app.operator_resource = "networks".into();
        app.networks = vec![NetworkSummary {
            id: "net1".into(),
            name: "bridge".into(),
            driver: "bridge".into(),
            scope: "local".into(),
            internal: false,
            ipv6_enabled: false,
            labels: HashMap::new(),
            containers: HashMap::new(),
        }];
        let (total, filtered) = row_counts(&app);
        assert_eq!(total, 1);
        assert_eq!(filtered, 1);
    }
    #[test]
    fn row_counts_unknown_resource() {
        let mut app = App::new(false);
        app.operator_resource = "unknown".into();
        let (total, filtered) = row_counts(&app);
        assert_eq!(total, 0);
        assert_eq!(filtered, 0);
    }

    #[test]
    fn help_lines_not_empty() {
        let lines = help_lines();
        assert!(!lines.is_empty());
        assert!(lines.iter().any(|l| l.contains("Navigation")));
    }
    #[test]
    fn help_lines_contains_quit() {
        let lines = help_lines();
        assert!(lines.iter().any(|l| l.contains(":q")));
    }

    #[test]
    fn k9s_title_no_filter() {
        let title = k9s_table_title("containers", 10, 10, "", "");
        assert!(title.contains("Containers"));
        assert!(title.contains("10"));
    }
    #[test]
    fn k9s_title_with_scroll_hint() {
        let title = k9s_table_title("images", 5, 3, "", " [1-3/5]");
        assert!(title.contains("Images"));
        assert!(title.contains("[1-3/5]"));
    }

    #[test]
    fn create_sorted_header_no_sort() {
        let cols = vec!["Name".into(), "Status".into()];
        let _row = create_sorted_header(&cols, None, true);
    }
    #[test]
    fn create_sorted_header_ascending() {
        let cols = vec!["Name".into(), "Status".into()];
        let _row = create_sorted_header(&cols, Some("Name"), true);
    }
    #[test]
    fn create_sorted_header_descending() {
        let cols = vec!["Name".into(), "Status".into()];
        let _row = create_sorted_header(&cols, Some("Name"), false);
    }

    #[test]
    fn current_columns_default() {
        let app = App::new(false);
        let config = DockctlConfig::default();
        let cols = current_columns(&app, &config);
        assert!(!cols.is_empty());
        assert!(cols.iter().any(|c| c == "Name"));
    }
    #[test]
    fn current_columns_wide_mode() {
        let mut app = App::new(false);
        app.wide_mode = true;
        let config = DockctlConfig::default();
        let cols = current_columns(&app, &config);
        assert!(cols.iter().any(|c| c == "Image"));
    }
    #[test]
    fn operator_max_cols_matches() {
        let app = App::new(false);
        let config = DockctlConfig::default();
        let max = operator_max_cols(&app, &config);
        assert_eq!(max, current_columns(&app, &config).len());
    }
}
