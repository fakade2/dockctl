use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::ui::titled_block;

/// Renders a scrollable log viewer panel.
pub fn draw_logs(area: Rect, frame: &mut Frame, lines: &[String]) {
    let block = titled_block("Logs");
    let inner_height = block.inner(area).height as usize;

    let styled_lines: Vec<Line> = lines
        .iter()
        .map(|line| {
            let style = if line.contains("ERROR") || line.contains("FATAL") {
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

    let scroll_offset = if styled_lines.len() > inner_height {
        (styled_lines.len() - inner_height) as u16
    } else {
        0
    };

    let paragraph = Paragraph::new(styled_lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, area);
}
