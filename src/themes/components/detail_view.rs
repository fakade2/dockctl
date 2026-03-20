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

    let inner = block.inner(area);
    let inner_height = inner.height as usize;

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

    let max_scroll = content_lines.len().saturating_sub(inner_height) as u16;
    let capped_scroll = app.detail_view_scroll.min(max_scroll);

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((capped_scroll, 0));

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn detail_view_fills_area() {
        let mut app = App::new(false);
        app.open_detail_view(
            "Test Detail".into(),
            vec!["Name: nginx".into(), "Image: nginx:latest".into()],
        );

        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_detail_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // All inner cells should be written - no bleed-through
        for y in 1..9u16 {
            for x in 1..49u16 {
                let cell = buffer.cell((x, y)).unwrap();
                assert!(
                    !cell.symbol().is_empty(),
                    "Empty cell at ({}, {}) - potential bleed-through",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn detail_view_key_value_styling() {
        let mut app = App::new(false);
        app.open_detail_view(
            "Container".into(),
            vec!["Name: nginx".into(), "State: running".into()],
        );

        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_detail_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Key portion ("Name:") should be DarkGray
        let key_cell = buffer.cell((1u16, 1u16)).unwrap();
        assert_eq!(
            key_cell.fg,
            Color::DarkGray,
            "Key text should be DarkGray, got {:?}",
            key_cell.fg
        );
    }

    #[test]
    fn detail_view_section_header_styling() {
        let mut app = App::new(false);
        app.open_detail_view(
            "Container".into(),
            vec!["Details:".into(), "Name: nginx".into()],
        );

        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_detail_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Section header ("Details:") should be Cyan + Bold
        let header_cell = buffer.cell((1u16, 1u16)).unwrap();
        assert_eq!(
            header_cell.fg,
            Color::Cyan,
            "Section header should be Cyan, got {:?}",
            header_cell.fg
        );
    }

    #[test]
    fn detail_view_scroll_capped() {
        let mut app = App::new(false);
        app.open_detail_view("Test".into(), vec!["a".into(), "b".into()]);
        app.detail_view_scroll = 100; // way past content

        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        // Should not panic
        terminal
            .draw(|frame| {
                draw_detail_view(frame, frame.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn detail_view_empty_content() {
        let mut app = App::new(false);
        app.open_detail_view("Empty".into(), vec![]);

        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        // Should not panic with empty content
        terminal
            .draw(|frame| {
                draw_detail_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Border should still render
        let top_left = buffer.cell((0u16, 0u16)).unwrap();
        assert!(
            ["┌", "╭"].contains(&top_left.symbol()),
            "Expected top-left corner, got '{}'",
            top_left.symbol()
        );
    }

    #[test]
    fn detail_view_title_rendered() {
        let mut app = App::new(false);
        app.open_detail_view("My Title".into(), vec!["content".into()]);

        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_detail_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // The title " My Title " should appear in the top border row
        let mut top_row = String::new();
        for x in 0..50u16 {
            let cell = buffer.cell((x, 0u16)).unwrap();
            top_row.push_str(cell.symbol());
        }
        assert!(
            top_row.contains("My Title"),
            "Title should appear in top row, got: '{}'",
            top_row
        );
    }
}
