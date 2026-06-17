//! Center column: Chat + Execution Timeline.
//! Top half: conversation view (user input + LLM responses)
//! Bottom half: agent execution timeline

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::types::{AgentState, ConnectionState, fmt_duration};

const BG_DEEP: Color = Color::Rgb(0, 0, 0);
const BORDER: Color = Color::Rgb(37, 44, 82);
const TEXT_PRIMARY: Color = Color::Rgb(201, 209, 255);
const TEXT_SECONDARY: Color = Color::Rgb(166, 175, 216);
const TEXT_MUTED: Color = Color::Rgb(126, 136, 181);
const PURPLE: Color = Color::Rgb(197, 111, 255);
const CYAN: Color = Color::Rgb(57, 216, 255);
const GREEN: Color = Color::Rgb(60, 229, 154);
const RED: Color = Color::Rgb(255, 90, 110);

pub fn draw_center(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(area);

    draw_chat(f, chunks[0], app);
    draw_timeline(f, chunks[1], app);
}

fn draw_chat(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Chat ", Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    if app.chat_messages.is_empty() {
        match app.connection_status {
            ConnectionState::Connected => {
                lines.push(Line::from(Span::styled(
                    "  Welcome to EvAgent. Type a prompt below.",
                    Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
                )));
            }
            ConnectionState::Connecting => {
                lines.push(Line::from(Span::styled(
                    "  Connecting to EvAgent core...",
                    Style::default().fg(CYAN),
                )));
            }
            ConnectionState::Disconnected => {
                lines.push(Line::from(Span::styled(
                    "  Connection lost. Reconnecting...",
                    Style::default().fg(RED),
                )));
            }
        }
    } else {
        for msg in app.chat_messages.iter().rev().take(100).rev() {
            let (prefix, style) = match msg.role.as_str() {
                "user" => ("┃ ", Style::default().fg(CYAN)),
                "assistant" => ("┃ ", Style::default().fg(PURPLE)),
                "system" => ("  ", Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC)),
                _ => ("  ", Style::default().fg(TEXT_SECONDARY)),
            };

            let content = &msg.content;
            // Split multi-line messages
            let content_lines: Vec<&str> = content.lines().collect();
            for (i, line_str) in content_lines.iter().enumerate() {
                let max_w = inner.width.saturating_sub(4) as usize;
                let display = if line_str.len() > max_w && max_w > 10 {
                    format!("{}…", &line_str[..max_w.saturating_sub(1)])
                } else {
                    line_str.to_string()
                };
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled(prefix, style),
                        Span::styled(display, Style::default().fg(TEXT_PRIMARY)),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("   {}", display),
                        Style::default().fg(TEXT_SECONDARY),
                    )));
                }
            }
        }
    }

    // Fill remaining space
    while lines.len() < inner.height.saturating_sub(1) as usize {
        lines.push(Line::from(""));
    }

    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG_DEEP))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn draw_timeline(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Execution Timeline ", Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.timeline_events.is_empty() {
        let msg = Paragraph::new(Text::from(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No agent activity yet.",
                Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
            )),
        ]))
        .style(Style::default().bg(BG_DEEP));
        f.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let max_visible = inner.height.saturating_sub(1) as usize;
    let start = if app.timeline_events.len() > max_visible {
        app.timeline_events.len() - max_visible
    } else {
        0
    };

    for event in app.timeline_events[start..].iter() {
        let (icon, scolor) = match event.status {
            AgentState::Completed => ("●", GREEN),
            AgentState::Running => ("▶", CYAN),
            AgentState::Failed => ("●", RED),
            AgentState::Timeout => ("●", RED),
            AgentState::Idle => ("○", TEXT_MUTED),
        };

        let action_w = (inner.width.saturating_sub(30) as usize).max(5);
        let action = if event.action.len() > action_w {
            format!("{}…", &event.action[..action_w.saturating_sub(1)])
        } else {
            event.action.clone()
        };

        let dur = if event.duration_ms > 0 {
            fmt_duration(event.duration_ms)
        } else { "—".into() };

        let agent_s = event.agent_name.split('.').last().unwrap_or(&event.agent_name);

        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", event.timestamp), Style::default().fg(TEXT_MUTED)),
            Span::styled("┃", Style::default().fg(BORDER)),
            Span::styled(format!(" {:<14}", agent_s), Style::default().fg(TEXT_SECONDARY)),
            Span::styled(format!(" {:<20}", action), Style::default().fg(TEXT_PRIMARY)),
            Span::styled(format!(" {:>6}", dur), Style::default().fg(TEXT_MUTED)),
            Span::raw(" "),
            Span::styled(icon, Style::default().fg(scolor).add_modifier(Modifier::BOLD)),
        ]));
    }

    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG_DEEP))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}
