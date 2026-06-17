//! Header bar — redesigned.
//!
//! ```text
//! EVAGENT    Project: EvAgent    Branch: main    184K tokens    $0.23    ● connected
//! ```

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::types::{ConnectionState, fmt_cost, fmt_tokens};

// ── Color Palette (90% grayscale, 5% green, 5% blue) ──
const BG_DEEP: Color = Color::Rgb(9, 11, 17);
const TEXT_PRIMARY: Color = Color::Rgb(215, 220, 229);
const TEXT_MUTED: Color = Color::Rgb(127, 136, 150);
const CYAN: Color = Color::Rgb(79, 195, 247);
const GREEN: Color = Color::Rgb(74, 222, 128);
const AMBER: Color = Color::Rgb(251, 191, 36);
const RED: Color = Color::Rgb(239, 68, 68);

pub fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let status_dot = match app.connection_status {
        ConnectionState::Connected => ("●", GREEN),
        ConnectionState::Connecting => ("●", AMBER),
        ConnectionState::Disconnected => ("●", RED),
    };

    let tokens_str = fmt_tokens(app.stats.total_tokens);
    let cost_str = fmt_cost(app.stats.total_cost);

    let left = Line::from(vec![
        Span::styled(
            " EVAGENT ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(TEXT_MUTED)),
        Span::styled(
            format!("Project: {}", app.stats.domain),
            Style::default().fg(TEXT_PRIMARY),
        ),
        Span::styled(" │ ", Style::default().fg(TEXT_MUTED)),
        Span::styled("main", Style::default().fg(TEXT_MUTED)),
    ]);

    let right = Line::from(vec![
        Span::styled(
            format!("{} tokens", tokens_str),
            Style::default().fg(TEXT_MUTED),
        ),
        Span::styled("  ", Style::default().fg(TEXT_MUTED)),
        Span::styled(cost_str, Style::default().fg(TEXT_MUTED)),
        Span::styled("  ", Style::default().fg(TEXT_MUTED)),
        Span::styled(
            status_dot.0,
            Style::default()
                .fg(status_dot.1)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            match app.connection_status {
                ConnectionState::Connected => " connected",
                ConnectionState::Connecting => " connecting",
                ConnectionState::Disconnected => " disconnected",
            },
            Style::default().fg(status_dot.1),
        ),
    ]);

    let left_width = left.width() as u16;
    let right_width = right.width() as u16;
    let padding = if area.width > left_width + right_width + 4 {
        area.width - left_width - right_width
    } else {
        1
    };

    let full_line = {
        let mut spans = left.spans.clone();
        spans.push(Span::raw(" ".repeat(padding as usize)));
        spans.extend(right.spans);
        spans
    };

    f.render_widget(
        Paragraph::new(Line::from(full_line))
            .style(Style::default().bg(BG_DEEP)),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
        },
    );
}
