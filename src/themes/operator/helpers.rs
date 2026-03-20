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
