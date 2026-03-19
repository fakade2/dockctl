use ratatui::{layout::Rect, Frame};

use crate::app::App;
use crate::config::LayoutConfig;
use crate::ui::draw_placeholder;

/// Placeholder for the Builds view. Renders a centered "coming soon" message.
pub fn draw_builds(frame: &mut Frame, area: Rect, _app: &App, _layout: &LayoutConfig) {
    draw_placeholder(frame, area, "Builds view -- coming soon");
}
