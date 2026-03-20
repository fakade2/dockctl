use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::app::App;

/// Shared popup component — centered overlay with scrollable content.
/// Used by the dashboard theme (operator uses inline detail view instead).
pub fn draw_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let popup_width = (area.width as u32 * 60 / 100) as u16;
    let popup_height = (area.height as u32 * 70 / 100) as u16;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", app.popup_title))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    let inner_height = inner.height as usize;

    let content_lines: Vec<Line> = app
        .popup_content
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(Color::White))))
        .collect();

    let max_scroll = content_lines.len().saturating_sub(inner_height) as u16;
    let capped_scroll = app.popup_scroll.min(max_scroll);

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((capped_scroll, 0));

    frame.render_widget(paragraph, popup_area);
}
