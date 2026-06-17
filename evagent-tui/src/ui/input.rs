//! Bottom input bar — redesigned.
//!
//! Larger (3 lines), bordered, prompt in CYAN.
//! ```text
//! ┌─────────────────────────────────────┐
//! │ > build simulator                   │
//! └─────────────────────────────────────┘
//! ```

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::ConnectionState;

// ── Color Palette ──
const BG_DEEP: Color = Color::Rgb(9, 11, 17);
const BORDER: Color = Color::Rgb(35, 42, 54);
const TEXT_PRIMARY: Color = Color::Rgb(215, 220, 229);
const TEXT_MUTED: Color = Color::Rgb(127, 136, 150);
const CYAN: Color = Color::Rgb(79, 195, 247);
const GREEN: Color = Color::Rgb(74, 222, 128);
const AMBER: Color = Color::Rgb(251, 191, 36);
const RED: Color = Color::Rgb(239, 68, 68);

pub fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let connected = app.connection_status == ConnectionState::Connected;

    // Border around the input area
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Connection status dot (right-aligned)
    let (dot, dot_color) = match app.connection_status {
        ConnectionState::Connected => ("●", GREEN),
        ConnectionState::Connecting => ("●", AMBER),
        ConnectionState::Disconnected => ("●", RED),
    };
    let status_span = Span::styled(
        format!(" {} ", dot),
        Style::default()
            .fg(dot_color)
            .add_modifier(Modifier::BOLD),
    );

    // Prompt prefix in CYAN
    let prefix = Span::styled(
        "> ",
        Style::default()
            .fg(if connected { CYAN } else { RED })
            .add_modifier(Modifier::BOLD),
    );

    // Input text or placeholder
    let (input_span, show_cursor) = if !connected {
        (
            Span::styled(
                "Reconnecting...",
                Style::default()
                    .fg(TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
            false,
        )
    } else if app.input.is_empty() {
        (
            Span::styled(
                "Type your prompt here...",
                Style::default()
                    .fg(TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
            false,
        )
    } else {
        (
            Span::styled(app.input.clone(), Style::default().fg(TEXT_PRIMARY)),
            true,
        )
    };

    // Build the content line
    let status_width = status_span.width() as u16;
    let input_display = format!("{}{}", prefix.content, input_span.content);
    let input_width = input_display.chars().count() as u16 + 2;
    let available = inner.width.saturating_sub(2);
    let dot_padding = if available > input_width + status_width {
        available - input_width
    } else {
        1
    };

    let content_line = Line::from(vec![
        prefix,
        input_span,
        Span::raw(" ".repeat(dot_padding.max(1) as usize)),
        status_span,
    ]);

    // Center the content vertically in the 3-line area
    let text_widget = Paragraph::new(Text::from(content_line))
        .style(Style::default().bg(BG_DEEP));

    // Render at the vertical center of the inner area
    let content_y = inner.y + (inner.height.saturating_sub(1)) / 2;
    let content_area = Rect {
        x: inner.x + 1,
        y: content_y,
        width: inner.width.saturating_sub(2),
        height: 1,
    };

    f.render_widget(text_widget, content_area);

    // Cursor positioning
    if show_cursor {
        let cursor_x = inner.x + 3 + (app.input_cursor as u16).min(app.input.len() as u16);
        let cursor_y = content_y;
        if cursor_x < inner.x + inner.width {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
