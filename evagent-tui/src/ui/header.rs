//! Neo-Terminal header bar.
//!
//! ```text
//! EVAGENT  ● coding  $0.02  3,400 tokens  00:12:34  ● connected
//! ```

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::types::{fmt_runtime, fmt_tokens_exact, ConnectionState};

// ── Color Palette (Neo-Terminal Design Spec) ──
const BG_DEEP: Color = Color::Rgb(3, 4, 10);
const TEXT_SECONDARY: Color = Color::Rgb(166, 175, 216);
const TEXT_MUTED: Color = Color::Rgb(126, 136, 181);
const PURPLE: Color = Color::Rgb(197, 111, 255);
const GREEN: Color = Color::Rgb(60, 229, 154);
const AMBER: Color = Color::Rgb(255, 200, 87);
const RED: Color = Color::Rgb(255, 90, 110);

pub fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    // Background fill
    f.render_widget(
        Paragraph::new(Line::from("")).style(Style::default().bg(BG_DEEP)),
        area,
    );

    let status_dot = match app.connection_status {
        ConnectionState::Connected => ("◉", GREEN),
        ConnectionState::Connecting => ("◐", AMBER),
        ConnectionState::Disconnected => ("○", RED),
    };

    let runtime_str = fmt_runtime(app.runtime);
    let tokens_str = fmt_tokens_exact(app.stats.total_tokens);
    let cost_str = format!("${:.2}", app.stats.total_cost);
    let domain = &app.stats.domain;

    // Left part: EVAGENT + domain badge
    let left = Line::from(vec![
        Span::styled(" EVAGENT ", Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)),
        Span::styled(" ", Style::default().fg(TEXT_MUTED)),
        Span::styled(
            format!("● {}", domain),
            Style::default().fg(TEXT_SECONDARY),
        ),
    ]);

    // Right part: cost, tokens, runtime, status
    let right = Line::from(vec![
        Span::styled(format!(" {}", cost_str), Style::default().fg(TEXT_MUTED)),
        Span::styled(" ", Style::default().fg(TEXT_MUTED)),
        Span::styled(
            format!("{} tokens", tokens_str),
            Style::default().fg(TEXT_MUTED),
        ),
        Span::styled(" ", Style::default().fg(TEXT_MUTED)),
        Span::styled(
            format!(" {}", runtime_str),
            Style::default().fg(TEXT_SECONDARY),
        ),
        Span::styled(" ", Style::default().fg(TEXT_MUTED)),
        Span::styled(
            format!(" {} ", status_dot.0),
            Style::default().fg(status_dot.1).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            match app.connection_status {
                ConnectionState::Connected => "connected",
                ConnectionState::Connecting => "connecting",
                ConnectionState::Disconnected => "disconnected",
            },
            Style::default().fg(status_dot.1),
        ),
    ]);

    // Calculate spacing for right alignment
    let right_width = right.width() as u16;
    let left_width = left.width() as u16;
    let padding = if area.width > left_width + right_width + 2 {
        area.width - left_width - right_width
    } else {
        1
    };

    // Combine left + padding + right
    let full_line = Line::from({
        let mut spans = left.spans.clone();
        spans.push(Span::raw(" ".repeat(padding as usize)));
        spans.extend(right.spans);
        spans
    });

    f.render_widget(
        Paragraph::new(full_line).style(Style::default().bg(BG_DEEP)),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
    );
}
