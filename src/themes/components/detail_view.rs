use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::app::App;

/// Shared detail view component — renders scrollable key-value content.
/// Used by both operator and dashboard themes.
pub fn draw_detail_view(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(Clear, area);

    let dim = Style::default().fg(Color::DarkGray);
    let cyan = Style::default().fg(Color::Cyan);
    let white = Style::default().fg(Color::White);

    let block = Block::default()
        .title(format!(" {} ", app.detail_view_title))
        .title_style(cyan.add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let content_lines: Vec<Line> = app
        .detail_view_content
        .iter()
        .map(|line| {
            if line.is_empty() {
                Line::from("")
            } else if line.ends_with(':')
                && line
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
            {
                // Section header
                Line::from(Span::styled(
                    line.clone(),
                    cyan.add_modifier(Modifier::BOLD),
                ))
            } else if let Some(colon_pos) = line.find(':') {
                // Key-value line
                let (key, val) = line.split_at(colon_pos + 1);
                Line::from(vec![
                    Span::styled(key.to_string(), dim),
                    Span::styled(val.to_string(), white),
                ])
            } else {
                Line::from(Span::styled(line.clone(), white))
            }
        })
        .collect();

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_view_scroll, 0));

    frame.render_widget(paragraph, area);
}
