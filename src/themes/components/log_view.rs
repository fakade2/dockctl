use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use crate::app::App;

/// Shared log viewer component — renders color-coded, scrollable container logs.
/// Supports vertical + horizontal scrolling, wrap toggle, and a scrollbar.
pub fn draw_log_view(frame: &mut Frame, area: Rect, app: &App) {
    // Clear entire area first
    frame.render_widget(Clear, area);

    let follow_indicator = if app.log_view_follow { " [FOLLOW]" } else { "" };
    let wrap_indicator = if app.log_view_wrap { " [WRAP]" } else { "" };
    let line_count = app.log_view_lines.len();
    let title = format!(
        " Logs: {}{}{} ({} lines) ",
        app.log_view_container_name, follow_indicator, wrap_indicator, line_count
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
    let inner_width = inner.width as usize;

    // Pad each log line to fill the full inner width
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
            let padded = if line.len() < inner_width {
                format!("{:<width$}", line, width = inner_width)
            } else {
                line.clone()
            };
            Line::from(Span::styled(padded, style))
        })
        .collect();

    // Pad empty lines below content
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

    let content_len = all_lines.len();
    let max_v_scroll = content_len.saturating_sub(inner_height);

    let v_scroll = if app.log_view_follow {
        max_v_scroll as u16
    } else {
        app.log_view_scroll.min(max_v_scroll as u16)
    };

    let h_scroll = if app.log_view_wrap {
        0
    } else {
        let max_line_len = app
            .log_view_lines
            .iter()
            .map(|l| l.len())
            .max()
            .unwrap_or(0);
        let max_h = max_line_len.saturating_sub(inner.width as usize) as u16;
        app.log_view_h_scroll.min(max_h)
    };

    let mut paragraph = Paragraph::new(all_lines).scroll((v_scroll, h_scroll));

    if app.log_view_wrap {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }

    frame.render_widget(paragraph, inner);

    // Vertical scrollbar on the right edge
    if content_len > inner_height {
        let scrollbar_area = Rect::new(inner.x + inner.width, inner.y, 1, inner.height);
        // Only render if there's space (the border column)
        if scrollbar_area.x < area.x + area.width {
            let mut scrollbar_state = ScrollbarState::new(max_v_scroll).position(v_scroll as usize);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(Color::Cyan))
                .track_style(Style::default().fg(Color::DarkGray));
            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn log_view_fills_entire_area() {
        let mut app = crate::app::App::new(false);
        app.open_log_view(
            "abc".into(),
            "nginx".into(),
            vec!["short log".into(), "another".into()],
        );

        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                draw_log_view(frame, area, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        for y in 1..9 {
            for x in 1..59 {
                let cell = buffer.cell((x, y)).unwrap();
                assert!(!cell.symbol().is_empty(), "Cell at ({}, {}) is empty", x, y);
            }
        }
    }

    #[test]
    fn log_view_short_lines_padded() {
        let mut app = crate::app::App::new(false);
        app.open_log_view("abc".into(), "test".into(), vec!["hi".into()]);

        let backend = TestBackend::new(40, 8);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        for x in 1..39 {
            let cell = buffer.cell((x, 1)).unwrap();
            assert!(!cell.symbol().is_empty(), "Empty cell at ({}, 1)", x);
        }
    }

    #[test]
    fn log_view_empty_lines_filled() {
        let mut app = crate::app::App::new(false);
        app.open_log_view("abc".into(), "test".into(), vec!["line1".into()]);

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        for y in 1..9 {
            for x in 1..39 {
                let cell = buffer.cell((x, y)).unwrap();
                assert!(!cell.symbol().is_empty(), "Empty cell at ({}, {})", x, y);
            }
        }
    }

    #[test]
    fn log_view_color_coding() {
        let mut app = crate::app::App::new(false);
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

        let error_cell = buffer.cell((1, 1)).unwrap();
        assert_eq!(error_cell.fg, Color::Red);

        let warn_cell = buffer.cell((1, 2)).unwrap();
        assert_eq!(warn_cell.fg, Color::Yellow);

        let info_cell = buffer.cell((1, 3)).unwrap();
        assert_eq!(info_cell.fg, Color::Green);
    }

    #[test]
    fn log_view_wrap_mode() {
        let mut app = crate::app::App::new(false);
        let long_line = "A".repeat(100);
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

        let cell_r1 = buffer.cell((1, 1)).unwrap();
        let cell_r2 = buffer.cell((1, 2)).unwrap();
        assert_eq!(cell_r1.symbol(), "A");
        assert_eq!(cell_r2.symbol(), "A");
    }

    #[test]
    fn log_view_border_characters() {
        let mut app = crate::app::App::new(false);
        app.open_log_view("abc".into(), "test".into(), vec!["line".into()]);

        let backend = TestBackend::new(40, 8);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let top_left = buffer.cell((0, 0)).unwrap();
        assert_eq!(top_left.symbol(), "┌");
    }

    #[test]
    fn log_view_follow_scrolls_to_bottom() {
        let mut app = crate::app::App::new(false);
        let lines: Vec<String> = (0..50).map(|i| format!("line {}", i)).collect();
        app.open_log_view("abc".into(), "test".into(), lines);
        // follow is true by default from open_log_view

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        // Last visible row should contain "line 49"
        let mut found_last = false;
        for x in 0..40 {
            if buffer.cell((x, 7)).unwrap().symbol() == "4" {
                found_last = true;
                break;
            }
        }
        assert!(found_last, "Follow mode should show the last lines");
    }

    #[test]
    fn log_view_scrollbar_renders_when_content_overflows() {
        let mut app = crate::app::App::new(false);
        let lines: Vec<String> = (0..50).map(|i| format!("line {}", i)).collect();
        app.open_log_view("abc".into(), "test".into(), lines);
        app.log_view_follow = false;
        app.log_view_scroll = 0;

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                draw_log_view(frame, frame.area(), &app);
            })
            .unwrap();

        // Should not panic — scrollbar renders on the border column
    }
}
