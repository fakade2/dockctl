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
    // Clear entire area first — fills every cell with space + default style
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
    frame.render_widget(block, area);
    let inner_height = inner.height as usize;

    // Pad each log line to fill the full inner width so no old content shows through
    let inner_width = inner.width as usize;

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
            // Pad line to full width to overwrite all cells
            let padded = if line.len() < inner_width {
                format!("{:<width$}", line, width = inner_width)
            } else {
                line.clone()
            };
            Line::from(Span::styled(padded, style))
        })
        .collect();

    // Also pad empty lines to fill the visible area
    let total_lines = styled_lines.len();
    let mut all_lines = styled_lines;
    if total_lines < inner_height {
        for _ in 0..(inner_height - total_lines) {
            all_lines.push(Line::from(Span::styled(
                " ".repeat(inner_width),
                Style::default(),
            )));
        }
    }

    let v_scroll = if app.log_view_follow {
        all_lines.len().saturating_sub(inner_height) as u16
    } else {
        app.log_view_scroll
            .min(all_lines.len().saturating_sub(inner_height) as u16)
    };

    let h_scroll = if app.log_view_wrap {
        0
    } else {
        app.log_view_h_scroll
    };

    let mut paragraph = Paragraph::new(all_lines).scroll((v_scroll, h_scroll));

    if app.log_view_wrap {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }

    frame.render_widget(paragraph, inner);
}
