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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn popup_renders_centered() {
        let mut app = App::new(false);
        app.open_popup("Help".into(), vec!["line1".into(), "line2".into()]);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_popup(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Popup is 60% of 80 = 48 wide, centered at x = (80 - 48) / 2 = 16
        // Popup is 70% of 24 = 16 tall, centered at y = (24 - 16) / 2 = 4
        let top_left = buffer.cell((16u16, 4u16)).unwrap();
        assert!(
            ["┌", "╭"].contains(&top_left.symbol()),
            "Expected top-left corner at (16, 4), got '{}'",
            top_left.symbol()
        );

        let top_right = buffer.cell((63u16, 4u16)).unwrap();
        assert!(
            ["┐", "╮"].contains(&top_right.symbol()),
            "Expected top-right corner at (63, 4), got '{}'",
            top_right.symbol()
        );

        let bottom_left = buffer.cell((16u16, 19u16)).unwrap();
        assert!(
            ["└", "╰"].contains(&bottom_left.symbol()),
            "Expected bottom-left corner at (16, 19), got '{}'",
            bottom_left.symbol()
        );
    }

    #[test]
    fn popup_scroll_capped() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["a".into(), "b".into()]);
        app.popup_scroll = 100; // way past content

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Should not panic
        terminal
            .draw(|frame| {
                draw_popup(frame, &app);
            })
            .unwrap();
    }

    #[test]
    fn popup_clears_underlying_area() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["content".into()]);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_popup(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // The popup area (inner cells) should all be explicitly written
        // Popup: x=16..64, y=4..20
        // Inner: x=17..63, y=5..19
        for y in 5..19u16 {
            for x in 17..63u16 {
                let cell = buffer.cell((x, y)).unwrap();
                assert!(
                    !cell.symbol().is_empty(),
                    "Empty cell at ({}, {}) inside popup area - potential bleed-through",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn popup_content_rendered() {
        let mut app = App::new(false);
        app.open_popup(
            "Help".into(),
            vec!["first line".into(), "second line".into()],
        );

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_popup(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Read the first content row inside the popup (y=5, x=17..63)
        let mut first_row = String::new();
        for x in 17..63u16 {
            let cell = buffer.cell((x, 5u16)).unwrap();
            first_row.push_str(cell.symbol());
        }
        assert!(
            first_row.contains("first line"),
            "First content line should be visible, got: '{}'",
            first_row.trim()
        );
    }

    #[test]
    fn popup_title_rendered() {
        let mut app = App::new(false);
        app.open_popup("My Popup".into(), vec!["content".into()]);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_popup(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Title should appear in the top border row of the popup (y=4)
        let mut title_row = String::new();
        for x in 16..64u16 {
            let cell = buffer.cell((x, 4u16)).unwrap();
            title_row.push_str(cell.symbol());
        }
        assert!(
            title_row.contains("My Popup"),
            "Title should appear in popup border, got: '{}'",
            title_row
        );
    }

    #[test]
    fn popup_empty_content() {
        let mut app = App::new(false);
        app.open_popup("Empty".into(), vec![]);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Should not panic with empty content
        terminal
            .draw(|frame| {
                draw_popup(frame, &app);
            })
            .unwrap();
    }

    #[test]
    fn popup_content_is_white() {
        let mut app = App::new(false);
        app.open_popup("Test".into(), vec!["hello".into()]);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_popup(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Content text should be white
        let content_cell = buffer.cell((17u16, 5u16)).unwrap();
        assert_eq!(
            content_cell.fg,
            Color::White,
            "Popup content should be white, got {:?}",
            content_cell.fg
        );
    }
}
