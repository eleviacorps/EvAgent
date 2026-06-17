//! Bottom command input bar.
//!
//! Single-line prompt with `> ` prefix and right-aligned status dot.
//! ```text
//! > Type your prompt to dispatch agents...                                          ◉
//! ```

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::ConnectionState;

// ── Color Palette ──
const BG_DEEP: Color = Color::Rgb(3, 4, 10);
const BORDER: Color = Color::Rgb(37, 44, 82);
const TEXT_PRIMARY: Color = Color::Rgb(201, 209, 255);
const TEXT_MUTED: Color = Color::Rgb(126, 136, 181);
const GREEN: Color = Color::Rgb(60, 229, 154);
const AMBER: Color = Color::Rgb(255, 200, 87);
const RED: Color = Color::Rgb(255, 90, 110);

pub fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let connected = app.connection_status == ConnectionState::Connected;

    // Thin top border as separator, no bottom/left/right borders
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Connection status dot (right-aligned)
    let (dot, dot_color) = match app.connection_status {
        ConnectionState::Connected => ("◉", GREEN),
        ConnectionState::Connecting => ("◐", AMBER),
        ConnectionState::Disconnected => ("○", RED),
    };
    let status_span = Span::styled(
        format!(" {} ", dot),
        Style::default().fg(dot_color).add_modifier(Modifier::BOLD),
    );

    // Prompt prefix
    let prefix = Span::styled(
        "> ",
        Style::default()
            .fg(if connected { GREEN } else { RED })
            .add_modifier(Modifier::BOLD),
    );

    // Input text or placeholder
    let (input_span, show_cursor) = if !connected {
        (
            Span::styled(
                "Reconnecting...",
                Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
            ),
            false,
        )
    } else if app.input.is_empty() {
        (
            Span::styled(
                "Type your prompt to dispatch agents...",
                Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
            ),
            false,
        )
    } else {
        (
            Span::styled(app.input.clone(), Style::default().fg(TEXT_PRIMARY)),
            true,
        )
    };

    // Calculate widths for right-aligned status dot
    let status_width = status_span.width() as u16;
    let input_text = format!("{}{}", prefix.content, input_span.content);
    let input_width = input_text.chars().count() as u16 + 2; // +2 for "> "
    let available = inner.width.saturating_sub(2); // 1 char padding each side
    let dot_padding = if available > input_width + status_width {
        available - input_width
    } else {
        1
    };

    let line = Line::from(vec![
        prefix,
        input_span,
        Span::raw(" ".repeat(dot_padding.max(1) as usize)),
        status_span,
    ]);

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(BG_DEEP)),
        inner,
    );

    // Cursor positioning
    if show_cursor {
        let cursor_x = inner.x + 2 + (app.input_cursor as u16).min(app.input.len() as u16);
        let cursor_y = inner.y;
        if cursor_x < inner.x + inner.width {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
