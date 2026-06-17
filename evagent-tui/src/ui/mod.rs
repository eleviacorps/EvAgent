//! EvAgent Neo-Terminal Command Center UI
//!
//! Three-column layout: [Left 28% | Center 44% | Right 28%]
//! Single outer borders — no double borders between panels.

pub mod header;
pub mod input;
pub mod left;
pub mod center;
pub mod right;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::app::App;

pub use header::draw_header;
pub use input::draw_input;
pub use left::draw_left;
pub use center::draw_center;
pub use right::draw_right;

/// Main draw function — called every frame by the event loop.
pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Vertical split: Header (1) | Content (flex) | Input Bar (1)
    let main = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    draw_header(f, main[0], app);
    draw_content(f, main[1], app);
    draw_input(f, main[2], app);
}

/// Draw the main content area: three-column layout.
fn draw_content(f: &mut Frame, area: Rect, app: &mut App) {
    if area.width < 60 || area.height < 5 {
        return;
    }

    let columns = Layout::horizontal([
        Constraint::Percentage(28),
        Constraint::Percentage(44),
        Constraint::Percentage(28),
    ])
    .split(area);

    draw_left(f, columns[0], app);
    draw_center(f, columns[1], app);
    draw_right(f, columns[2], app);
}
