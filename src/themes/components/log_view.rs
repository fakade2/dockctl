use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::app::App;

/// Shared log viewer component — renders color-coded, scrollable container logs.
/// Supports vertical + horizontal scrolling and a wrap toggle.
pub fn draw_log_view(frame: &mut Frame, area: Rect, app: &App) {
    // Clear previous frame content
    frame.render_widget(Clear, area);

    let follow_indicator = if app.log_view_follow { " [FOLLOW]" } else { "" };
    let wrap_indicator = if app.log_view_wrap { " [WRAP]" } else { "" };
    let title = format!(
        " Logs: {}{}{} ",
        app.log_view_container_name, follow_indicator, wrap_indicator
    );

    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    let inner_height = inner.height as usize;

    let styled_lines: Vec<Line> = app
        .log_view_lines
        .iter()
        .map(|line| {
            let style =
                if line.contains("ERROR") || line.contains("FATAL") || line.contains("PANIC") {
                    Style::default().fg(Color::Red)
                } else if line.contains("WARN") {
                    Style::default().fg(Color::Yellow)
                } else if line.contains("INFO") {
                    Style::default().fg(Color::Green)
                } else if line.contains("DEBUG") || line.contains("TRACE") {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };
            Line::from(Span::styled(line.as_str(), style))
        })
        .collect();

    let v_scroll = if app.log_view_follow {
        styled_lines.len().saturating_sub(inner_height) as u16
    } else {
        // Cap manual scroll to prevent scrolling past content
        app.log_view_scroll
            .min(styled_lines.len().saturating_sub(inner_height) as u16)
    };

    let h_scroll = if app.log_view_wrap {
        0 // no horizontal scroll when wrapping
    } else {
        app.log_view_h_scroll
    };

    let mut paragraph = Paragraph::new(styled_lines)
        .block(block)
        .scroll((v_scroll, h_scroll));

    if app.log_view_wrap {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }

    frame.render_widget(paragraph, area);
}
