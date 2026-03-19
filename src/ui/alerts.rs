use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{AlertLevel, App, PanelId};
use crate::ui::{panel_block, selected_row_style};

/// Renders the Alerts panel (panel 1, left column).
///
/// Shows a list of system alerts with colored severity indicators.
/// If no alerts exist, shows a green "No alerts" message.
pub fn draw_alerts_panel(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let block = panel_block("Alerts", PanelId::Alerts.number(), is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.alerts.is_empty() {
        let lines = vec![Line::from(Span::styled(
            " No alerts",
            Style::default().fg(Color::Green),
        ))];
        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
        return;
    }

    let selected = app.panel_selection(PanelId::Alerts);

    let lines: Vec<Line> = app
        .alerts
        .iter()
        .enumerate()
        .map(|(i, alert)| {
            let (indicator, color) = match alert.level {
                AlertLevel::Critical => ("!!", Color::Red),
                AlertLevel::Warning => ("! ", Color::Yellow),
            };

            let row_style = selected_row_style(is_focused, i == selected);

            Line::from(vec![
                Span::styled(
                    format!(" {} ", indicator),
                    row_style.fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    alert.message.clone(),
                    row_style.fg(color),
                ),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
