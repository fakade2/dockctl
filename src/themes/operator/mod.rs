mod commands;
pub(super) mod detail_view;
mod helpers;
mod keys;
mod status_view;
mod tables;

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::config::DockctlConfig;
use crate::display::{format_bytes, percentage_color};
use crate::themes::{Theme, ThemeAction};

use super::components::{draw_detail_view, draw_log_view};
use commands::autocomplete_suggestion;
use helpers::{capitalize, row_counts};
use keys::operator_handle_key;
use tables::draw_resource_table;

pub struct OperatorTheme;

impl Default for OperatorTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl OperatorTheme {
    pub fn new() -> Self {
        Self
    }
}

impl Theme for OperatorTheme {
    fn name(&self) -> &str {
        "operator"
    }

    fn draw(&self, frame: &mut Frame, app: &App, config: &DockctlConfig) {
        draw_operator(frame, app, config);
    }

    fn handle_key(&self, key: KeyEvent, app: &mut App, config: &DockctlConfig) -> ThemeAction {
        operator_handle_key(key, app, config)
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn draw_operator(frame: &mut Frame, app: &App, config: &DockctlConfig) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header (2 content rows + bottom border)
            Constraint::Length(1), // command line
            Constraint::Min(1),    // table / detail / status
            Constraint::Length(1), // footer
        ])
        .split(frame.area());

    draw_header(frame, outer[0], app);
    draw_command_line(frame, outer[1], app);

    if app.log_view_active {
        draw_log_view(frame, outer[2], app);
    } else if app.detail_view_active {
        draw_detail_view(frame, outer[2], app);
    } else {
        draw_resource_table(frame, outer[2], app, config);
    }

    draw_footer(frame, outer[3], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let resource = &app.operator_resource;
    let resource_cap = capitalize(resource);

    let logo_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);
    let green = Style::default().fg(Color::Green);
    let red = Style::default().fg(Color::Red);

    // --- Row 1 ---
    // Left: "dockctl    Docker > Containers(9)"
    // Right: "v29.2.1 . 9 up 2 down . 16GB"
    let running = app
        .containers
        .iter()
        .filter(|c| c.state == "running")
        .count();
    let stopped = app.containers.len() - running;
    let (_total, filtered) = row_counts(app);

    let row1_left: Vec<Span> = if app.log_view_active {
        let follow_tag = if app.log_view_follow { " [FOLLOW]" } else { "" };
        vec![
            Span::styled(" dockctl", logo_style),
            Span::raw("    "),
            Span::styled("Docker", dim),
            Span::styled(" \u{203a} ", dim),
            Span::styled(resource_cap.clone(), white),
            Span::styled(" \u{203a} ", dim),
            Span::styled(app.log_view_container_name.clone(), white),
            Span::styled(" \u{203a} ", dim),
            Span::styled(format!("Logs{}", follow_tag), white),
        ]
    } else if app.detail_view_active {
        let detail_name = app
            .detail_view_title
            .split(": ")
            .nth(1)
            .unwrap_or(&app.detail_view_title);
        vec![
            Span::styled(" dockctl", logo_style),
            Span::raw("    "),
            Span::styled("Docker", dim),
            Span::styled(" \u{203a} ", dim),
            Span::styled(resource_cap.clone(), white),
            Span::styled(" \u{203a} ", dim),
            Span::styled(detail_name.to_string(), white),
        ]
    } else {
        vec![
            Span::styled(" dockctl", logo_style),
            Span::raw("    "),
            Span::styled("Docker", dim),
            Span::styled(" \u{203a} ", dim),
            Span::styled(format!("{}({})", resource_cap, filtered), white),
        ]
    };

    let runtime_version = if let Some(info) = &app.system_info {
        format!("v{}", info.server_version)
    } else {
        "connecting...".to_string()
    };
    let total_mem = if let Some(info) = &app.system_info {
        format_bytes(info.memory_total)
    } else {
        String::new()
    };

    let mut row1_right: Vec<Span> = vec![
        Span::styled(runtime_version, dim),
        Span::styled(" \u{00b7} ", dim),
        Span::styled(format!("{}", running), green),
        Span::styled("\u{2191}", green),
        Span::styled(" ", dim),
        Span::styled(format!("{}", stopped), if stopped > 0 { red } else { dim }),
        Span::styled("\u{2193}", if stopped > 0 { red } else { dim }),
        Span::styled(" \u{00b7} ", dim),
        Span::styled(total_mem, dim),
        Span::raw(" "),
    ];

    let row1_left_len: usize = row1_left.iter().map(|s| s.width()).sum();
    let row1_right_len: usize = row1_right.iter().map(|s| s.width()).sum();
    let row1_pad = (area.width as usize).saturating_sub(row1_left_len + row1_right_len);

    let mut row1_spans = row1_left;
    row1_spans.push(Span::raw(" ".repeat(row1_pad)));
    row1_spans.append(&mut row1_right);

    // --- Row 2 ---
    // Left: CPU/RAM/Swap percentages
    // Right: core count
    let cpu_pct = app.host_cpu_percent;
    let mem_pct = if app.host_memory_total > 0 {
        (app.host_memory_used as f64 / app.host_memory_total as f64) * 100.0
    } else {
        0.0
    };
    let swap_pct = if app.host_swap_total > 0 {
        (app.host_swap_used as f64 / app.host_swap_total as f64) * 100.0
    } else {
        0.0
    };
    let cpu_color = percentage_color(cpu_pct);
    let mem_color = percentage_color(mem_pct);
    let swap_color = percentage_color(swap_pct);

    let key_hint = Style::default().fg(Color::Cyan);

    let (row2_left, row2_right): (Vec<Span>, Vec<Span>) = if app.log_view_active {
        // Show tail shortcuts when log view is open
        let lines_count = app.log_view_lines.len();
        (
            vec![
                Span::styled(" Tail: ", dim),
                Span::styled("0", key_hint),
                Span::styled("=all ", dim),
                Span::styled("1", key_hint),
                Span::styled("=50 ", dim),
                Span::styled("2", key_hint),
                Span::styled("=100 ", dim),
                Span::styled("3", key_hint),
                Span::styled("=200 ", dim),
                Span::styled("4", key_hint),
                Span::styled("=500 ", dim),
                Span::styled("5", key_hint),
                Span::styled("=1k ", dim),
            ],
            vec![Span::styled(format!("{} lines ", lines_count), dim)],
        )
    } else {
        // Normal: CPU/RAM/Swap stats
        (
            vec![
                Span::styled(" CPU: ", dim),
                Span::styled(format!("{:.1}%", cpu_pct), Style::default().fg(cpu_color)),
                Span::styled("  RAM: ", dim),
                Span::styled(format!("{:.1}%", mem_pct), Style::default().fg(mem_color)),
                Span::styled("  Swap: ", dim),
                Span::styled(format!("{:.1}%", swap_pct), Style::default().fg(swap_color)),
            ],
            if let Some(info) = &app.system_info {
                vec![
                    Span::styled(format!("{} cores", info.cpus), dim),
                    Span::raw(" "),
                ]
            } else {
                vec![Span::raw(" ")]
            },
        )
    };

    let row2_left_len: usize = row2_left.iter().map(|s| s.width()).sum();
    let row2_right_len: usize = row2_right.iter().map(|s| s.width()).sum();
    let row2_pad = (area.width as usize).saturating_sub(row2_left_len + row2_right_len);

    let mut row2_spans = row2_left;
    row2_spans.push(Span::raw(" ".repeat(row2_pad)));
    row2_spans.extend(row2_right);

    let lines = vec![Line::from(row1_spans), Line::from(row2_spans)];

    let header = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, area);
}

fn draw_command_line(frame: &mut Frame, area: Rect, app: &App) {
    let is_command_focused = app.operator_command_mode;
    let is_filter_focused = app.operator_filter_active;

    let line = if is_command_focused {
        let input = &app.operator_command_input;
        let mut spans = vec![
            Span::styled(":", Style::default().fg(Color::Cyan)),
            Span::styled(input.clone(), Style::default().fg(Color::White)),
        ];

        // Ghost text: show autocomplete suggestion after the cursor
        if let Some(suggestion) = autocomplete_suggestion(input) {
            if suggestion.len() > input.len() {
                let ghost = &suggestion[input.len()..];
                spans.push(Span::styled(
                    ghost.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            // Show Tab hint
            spans.push(Span::styled(
                "  Tab↹",
                Style::default().fg(Color::Rgb(60, 60, 60)),
            ));
        }

        // Block cursor
        spans.push(Span::styled("\u{2588}", Style::default().fg(Color::White)));
        Line::from(spans)
    } else if is_filter_focused {
        Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Cyan)),
            Span::styled(
                app.operator_filter.clone(),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("\u{2588}", Style::default().fg(Color::Yellow)),
        ])
    } else if app.log_view_active {
        let follow_tag = if app.log_view_follow { " [FOLLOW]" } else { "" };
        Line::from(vec![
            Span::styled(":logs ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.log_view_container_name.clone(),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(follow_tag.to_string(), Style::default().fg(Color::Green)),
        ])
    } else {
        let mut spans = vec![
            Span::styled(":", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.operator_resource.clone(),
                Style::default().fg(Color::DarkGray),
            ),
        ];
        if !app.operator_filter.is_empty() {
            spans.push(Span::styled("  /", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                app.operator_filter.clone(),
                Style::default().fg(Color::Yellow),
            ));
        }
        Line::from(spans)
    };

    let bg = if is_command_focused || is_filter_focused {
        Style::default().bg(Color::Rgb(22, 27, 34))
    } else {
        Style::default()
    };

    let paragraph = Paragraph::new(line).style(bg);
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let key_style = Style::default().fg(Color::Cyan);
    let label_style = Style::default().fg(Color::DarkGray);

    let shortcuts: Vec<Span> = if app.log_view_active {
        vec![
            Span::styled(" Esc", key_style),
            Span::styled(":back ", label_style),
            Span::styled("f", key_style),
            Span::styled(":follow ", label_style),
            Span::styled("j/k", key_style),
            Span::styled(":scroll ", label_style),
            Span::styled("g", key_style),
            Span::styled("/", label_style),
            Span::styled("G", key_style),
            Span::styled(":top/end ", label_style),
            Span::styled("0-5", key_style),
            Span::styled(":tail(50-1k) ", label_style),
        ]
    } else if app.detail_view_active {
        vec![
            Span::styled(" Esc", key_style),
            Span::styled(":back ", label_style),
            Span::styled("j/k", key_style),
            Span::styled(":scroll ", label_style),
            Span::styled("g", key_style),
            Span::styled(":top ", label_style),
            Span::styled("G", key_style),
            Span::styled(":bottom ", label_style),
        ]
    } else {
        let mut s: Vec<Span> = vec![
            Span::styled(" :", key_style),
            Span::styled("cmd ", label_style),
            Span::styled("/", key_style),
            Span::styled("filter ", label_style),
            Span::styled("d", key_style),
            Span::styled("describe ", label_style),
        ];
        if app.operator_resource == "containers" {
            s.extend(vec![
                Span::styled("s", key_style),
                Span::styled("stop ", label_style),
                Span::styled("r", key_style),
                Span::styled("restart ", label_style),
                Span::styled("x", key_style),
                Span::styled("rm ", label_style),
                Span::styled("l", key_style),
                Span::styled("logs ", label_style),
            ]);
        }
        s.extend(vec![
            Span::styled("?", key_style),
            Span::styled("help", label_style),
        ]);
        s
    };

    let (total, filtered) = row_counts(app);
    let rows_str = if total != filtered {
        format!("rows:{}/{}", filtered, total)
    } else {
        format!("rows:{}", total)
    };
    let version = concat!("dockctl v", env!("CARGO_PKG_VERSION"));

    let left_len: usize = shortcuts.iter().map(|s| s.width()).sum();
    let right_str = format!("{}  {}", rows_str, version);
    let right_len = right_str.len() + 1;
    let padding = (area.width as usize).saturating_sub(left_len + right_len);

    let mut spans = shortcuts;
    spans.push(Span::raw(" ".repeat(padding)));
    spans.push(Span::styled(
        right_str,
        Style::default().fg(Color::DarkGray),
    ));
    spans.push(Span::raw(" "));

    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}
