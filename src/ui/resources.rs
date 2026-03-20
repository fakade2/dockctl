use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::{App, PanelId};
use crate::ui::{panel_block, selected_row_style};

/// Renders the Resources panel (panel 2, left column).
///
/// Shows 4 selectable rows:
/// - Containers: running/stopped counts with color coding
/// - Images: total count
/// - Volumes: total count
/// - Networks: total count
pub fn draw_resources_panel(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let block = panel_block("Resources", PanelId::Resources.number(), is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let selected = app.panel_selection(PanelId::Resources);

    // Compute counts
    let (running, stopped) = match &app.system_info {
        Some(info) => (info.containers_running, info.containers_stopped),
        None => {
            let running = app
                .containers
                .iter()
                .filter(|c| c.state == "running")
                .count() as i64;
            let stopped = app.containers.len() as i64 - running;
            (running, stopped)
        }
    };

    let image_count = match &app.system_info {
        Some(info) => info.images,
        None => app.images.len() as i64,
    };

    let volume_count = app.volumes.len();
    let network_count = app.networks.len();

    let rows: Vec<Line> = vec![
        // Row 0: Containers
        make_row(
            "C",
            vec![
                Span::styled(
                    format!("  {}", running),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" running  ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{}", stopped),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" stopped", Style::default().fg(Color::Red)),
            ],
            is_focused && selected == 0,
        ),
        // Row 1: Images
        make_row(
            "I",
            vec![Span::styled(
                format!("  {} images", image_count),
                Style::default().fg(Color::White),
            )],
            is_focused && selected == 1,
        ),
        // Row 2: Volumes
        make_row(
            "V",
            vec![Span::styled(
                format!("  {} volumes", volume_count),
                Style::default().fg(Color::White),
            )],
            is_focused && selected == 2,
        ),
        // Row 3: Networks
        make_row(
            "N",
            vec![Span::styled(
                format!("  {} networks", network_count),
                Style::default().fg(Color::White),
            )],
            is_focused && selected == 3,
        ),
    ];

    let paragraph = Paragraph::new(rows);
    frame.render_widget(paragraph, inner);
}

/// Creates a single resource row with a letter badge, value spans, and optional
/// highlight background.
fn make_row<'a>(badge: &str, value_spans: Vec<Span<'a>>, is_selected: bool) -> Line<'a> {
    let base = selected_row_style(true, is_selected);

    let mut spans = vec![Span::styled(
        format!(" {} ", badge),
        base.fg(Color::DarkGray).add_modifier(Modifier::BOLD),
    )];

    for mut span in value_spans {
        // Merge base selection style into each span's existing style
        if is_selected {
            span.style = span.style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
        }
        spans.push(span);
    }

    Line::from(spans)
}
