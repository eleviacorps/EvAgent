//! Center column: Execution Timeline.
//!
//! Vertical timeline with timestamp, agent, action, duration, status.
//! ```text
//! 12:31:22 ┃ code-writer     Modifying engine.py          14.2s  ●
//! 12:31:24 ┃ reviewer        Checking types                3.1s  ●
//! 12:31:27 ┃ planner         Planning dashboard            —     ▶
//! ```

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::types::{AgentState, fmt_duration};

// ── Color Palette ──
const BG_DEEP: Color = Color::Rgb(3, 4, 10);
const BORDER: Color = Color::Rgb(37, 44, 82);
const TEXT_PRIMARY: Color = Color::Rgb(201, 209, 255);
const TEXT_SECONDARY: Color = Color::Rgb(166, 175, 216);
const TEXT_MUTED: Color = Color::Rgb(126, 136, 181);
const PURPLE: Color = Color::Rgb(197, 111, 255);
const CYAN: Color = Color::Rgb(57, 216, 255);
const GREEN: Color = Color::Rgb(60, 229, 154);
const RED: Color = Color::Rgb(255, 90, 110);

pub fn draw_center(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(
            " Execution Timeline ",
            Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.timeline_events.is_empty() {
        let msg = Paragraph::new(Text::from(vec![
            Line::from(""),
            Line::from(Span::styled(
                " Awaiting agent activity...",
                Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
            )),
            Line::from(Span::styled(
                " Type a prompt to dispatch agents.",
                Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
            )),
        ]))
        .style(Style::default().bg(BG_DEEP));
        f.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let max_visible = inner.height.saturating_sub(1) as usize;

    // Show recent events (reversed chronological order, most recent first)
    let start = if app.timeline_events.len() > max_visible {
        app.timeline_events.len() - max_visible
    } else {
        0
    };

    for event in app.timeline_events[start..].iter() {
        let status_indicator = match event.status {
            AgentState::Completed => ("●", GREEN),
            AgentState::Running => ("▶", CYAN),
            AgentState::Failed => ("●", RED),
            AgentState::Timeout => ("●", RED),
            AgentState::Idle => ("○", TEXT_MUTED),
        };

        // Format the action text with width limiting
        let action_width = inner.width.saturating_sub(30) as usize;
        let action = if event.action.len() > action_width && action_width > 5 {
            format!("{}…", &event.action[..action_width.saturating_sub(1)])
        } else {
            event.action.clone()
        };

        // Format duration
        let duration_str = if event.duration_ms > 0 {
            fmt_duration(event.duration_ms)
        } else {
            "—".to_string()
        };

        // Agent name (use last part of dotted name)
        let agent_short = event
            .agent_name
            .split('.')
            .last()
            .unwrap_or(&event.agent_name);

        let line = Line::from(vec![
            // Timestamp
            Span::styled(
                format!(" {} ", event.timestamp),
                Style::default().fg(TEXT_MUTED),
            ),
            // Vertical line
            Span::styled("┃", Style::default().fg(BORDER)),
            // Agent name
            Span::styled(
                format!(" {:<14}", agent_short),
                Style::default().fg(TEXT_SECONDARY),
            ),
            // Action
            Span::styled(
                format!(" {:<20}", action),
                Style::default().fg(TEXT_PRIMARY),
            ),
            // Duration
            Span::styled(
                format!(" {:>6}", duration_str),
                Style::default().fg(TEXT_MUTED),
            ),
            // Status
            Span::raw(" "),
            Span::styled(
                status_indicator.0,
                Style::default()
                    .fg(status_indicator.1)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        lines.push(line);
    }

    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG_DEEP))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}
