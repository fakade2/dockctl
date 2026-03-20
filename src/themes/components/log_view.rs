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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn log_view_fills_entire_area() {
        let mut app = App::new(false);
        app.open_log_view(
            "abc".into(),
            "nginx".into(),
            vec!["short log".into(), "another".into()],
        );
        app.log_view_follow = false;
        app.log_view_scroll = 0;

        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                draw_log_view(frame, area, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Every inner cell (inside the border) should be explicitly written.
        // No cell should have an empty symbol, which would indicate bleed-through.
        for y in 1..9 {
            for x in 1..59 {
                let cell = buffer.cell((x, y)).unwrap();
                assert!(
                    !cell.symbol().is_empty(),
                    "Cell at ({}, {}) is empty - potential bleed-through",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn log_view_short_lines_padded() {
        let mut app = App::new(false);
        app.open_log_view("abc".into(), "test".into(), vec!["hi".into()]);
        app.log_view_follow = false;
        app.log_view_scroll = 0;

        let backend = TestBackend::new(40, 8);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Row 1 (first content row inside border) should have "hi" at start
        // and spaces for the rest - no empty cells
        for x in 1..39 {
            let cell = buffer.cell((x, 1u16)).unwrap();
            assert!(!cell.symbol().is_empty(), "Empty cell at ({}, 1)", x);
        }
    }

    #[test]
    fn log_view_empty_lines_filled() {
        let mut app = App::new(false);
        app.open_log_view(
            "abc".into(),
            "test".into(),
            vec!["line1".into()], // only 1 line, but area has many rows
        );
        app.log_view_follow = false;
        app.log_view_scroll = 0;

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // All inner rows should be filled (not just row 1)
        for y in 1..9u16 {
            for x in 1..39u16 {
                let cell = buffer.cell((x, y)).unwrap();
                assert!(!cell.symbol().is_empty(), "Empty cell at ({}, {})", x, y);
            }
        }
    }

    #[test]
    fn log_view_color_coding() {
        let mut app = App::new(false);
        app.open_log_view(
            "abc".into(),
            "test".into(),
            vec![
                "ERROR something failed".into(),
                "WARN careful here".into(),
                "INFO all good".into(),
                "DEBUG verbose stuff".into(),
                "plain line".into(),
            ],
        );
        app.log_view_follow = false;
        app.log_view_scroll = 0;

        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Row 1 (ERROR) should be red
        let error_cell = buffer.cell((1u16, 1u16)).unwrap();
        assert_eq!(error_cell.fg, Color::Red, "ERROR line should be red");

        // Row 2 (WARN) should be yellow
        let warn_cell = buffer.cell((1u16, 2u16)).unwrap();
        assert_eq!(warn_cell.fg, Color::Yellow, "WARN line should be yellow");

        // Row 3 (INFO) should be green
        let info_cell = buffer.cell((1u16, 3u16)).unwrap();
        assert_eq!(info_cell.fg, Color::Green, "INFO line should be green");

        // Row 4 (DEBUG) should be dark gray
        let debug_cell = buffer.cell((1u16, 4u16)).unwrap();
        assert_eq!(
            debug_cell.fg,
            Color::DarkGray,
            "DEBUG line should be dark gray"
        );

        // Row 5 (plain) should be white
        let plain_cell = buffer.cell((1u16, 5u16)).unwrap();
        assert_eq!(plain_cell.fg, Color::White, "plain line should be white");
    }

    #[test]
    fn log_view_wrap_mode() {
        let mut app = App::new(false);
        let long_line = "A".repeat(100); // longer than 40-char terminal
        app.open_log_view("abc".into(), "test".into(), vec![long_line]);
        app.log_view_wrap = true;
        app.log_view_follow = false;
        app.log_view_scroll = 0;

        let backend = TestBackend::new(40, 8);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // With wrap on, the long line should span multiple rows.
        // Row 1 and row 2 should both have 'A' characters.
        let cell_r1 = buffer.cell((1u16, 1u16)).unwrap();
        let cell_r2 = buffer.cell((1u16, 2u16)).unwrap();
        assert_eq!(cell_r1.symbol(), "A");
        assert_eq!(
            cell_r2.symbol(),
            "A",
            "Wrapped content should appear on row 2"
        );
    }

    #[test]
    fn log_view_border_characters() {
        let mut app = App::new(false);
        app.open_log_view("abc".into(), "test".into(), vec!["line".into()]);
        app.log_view_follow = false;
        app.log_view_scroll = 0;

        let backend = TestBackend::new(40, 8);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // Verify border corners exist
        let top_left = buffer.cell((0u16, 0u16)).unwrap();
        assert!(
            ["┌", "╭"].contains(&top_left.symbol()),
            "Expected top-left corner, got '{}'",
            top_left.symbol()
        );

        let top_right = buffer.cell((39u16, 0u16)).unwrap();
        assert!(
            ["┐", "╮"].contains(&top_right.symbol()),
            "Expected top-right corner, got '{}'",
            top_right.symbol()
        );

        let bottom_left = buffer.cell((0u16, 7u16)).unwrap();
        assert!(
            ["└", "╰"].contains(&bottom_left.symbol()),
            "Expected bottom-left corner, got '{}'",
            bottom_left.symbol()
        );
    }

    #[test]
    fn log_view_follow_scrolls_to_bottom() {
        let mut app = App::new(false);
        let many_lines: Vec<String> = (0..50).map(|i| format!("log line {}", i)).collect();
        app.open_log_view("abc".into(), "test".into(), many_lines);
        // open_log_view sets follow=true by default

        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // With follow mode and 50 lines, the last visible line should contain
        // "log line 49" (the last line of content).
        // The inner height is 8 (10 - 2 for borders), so we check the last
        // content row (y=8, which is the last row before bottom border).
        let mut last_row_text = String::new();
        for x in 1..59u16 {
            let cell = buffer.cell((x, 8u16)).unwrap();
            last_row_text.push_str(cell.symbol());
        }
        assert!(
            last_row_text.contains("log line 49"),
            "Follow mode should show last line, got: '{}'",
            last_row_text.trim()
        );
    }
}
