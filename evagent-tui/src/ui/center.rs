//! Center column: Conversation Stream — merged chat + agent timeline.
//!
//! Shows user messages, agent response cards, and system messages in a
//! unified scrollable stream. Each agent message is rendered as a bordered
//! card with embedded tool calls and diff summaries.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::types::{ChatMessage, ConnectionState};
use crate::ui::markdown::{render_markdown, render_progress_bar};

// ── Color Palette ──
const BG_DEEP: Color = Color::Rgb(9, 11, 17);
const BORDER: Color = Color::Rgb(35, 42, 54);
const TEXT_PRIMARY: Color = Color::Rgb(215, 220, 229);
const TEXT_MUTED: Color = Color::Rgb(127, 136, 150);
const CYAN: Color = Color::Rgb(79, 195, 247);
const GREEN: Color = Color::Rgb(74, 222, 128);
const RED: Color = Color::Rgb(239, 68, 68);

pub fn draw_center(f: &mut Frame, area: Rect, app: &App) {
    let max_h = area.height.saturating_sub(1) as usize;
    let max_w = area.width.saturating_sub(2) as usize;

    let mut lines: Vec<Line> = Vec::new();

    if app.chat_messages.is_empty() {
        // Empty state
        let msg = match app.connection_status {
            ConnectionState::Connected => {
                "  Welcome to EvAgent. Type a prompt below."
            }
            ConnectionState::Connecting => {
                "  Connecting to EvAgent core..."
            }
            ConnectionState::Disconnected => {
                "  Connection lost. Reconnecting..."
            }
        };
        let color = match app.connection_status {
            ConnectionState::Connected => TEXT_MUTED,
            ConnectionState::Connecting => CYAN,
            ConnectionState::Disconnected => RED,
        };
        lines.push(Line::from(Span::styled(
            msg,
            Style::default().fg(color).add_modifier(Modifier::ITALIC),
        )));
    } else {
        for msg in app.chat_messages.iter() {
            match msg.role.as_str() {
                "user" => {
                    render_user_message(&mut lines, msg, max_w);
                }
                "agent" => {
                    render_agent_card(&mut lines, msg, max_w);
                }
                "assistant" => {
                    render_assistant_message(&mut lines, msg, max_w);
                }
                "system" => {
                    render_system_message(&mut lines, msg, max_w);
                }
                _ => {
                    render_system_message(&mut lines, msg, max_w);
                }
            }

            // Add small space between messages
            lines.push(Line::from(""));
        }
    }

    // Fill remaining space with empty lines
    let visible_count = lines.len().min(max_h);
    let scroll_offset = if visible_count > 0 {
        lines.len().saturating_sub(max_h)
    } else {
        0
    };

    // Only show the last `max_h` lines
    if scroll_offset > 0 {
        lines = lines.split_off(scroll_offset);
    }

    // Pad to fill height
    while lines.len() < max_h {
        lines.push(Line::from(""));
    }

    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG_DEEP))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

// ── Render Helpers ──

fn render_user_message(lines: &mut Vec<Line>, msg: &ChatMessage, max_w: usize) {
    // "You" header
    lines.push(Line::from(vec![
        Span::styled(
            " You ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
    ]));

    // Content with markdown rendering
    let content_lines = render_markdown(&msg.content);
    for cl in content_lines {
        let truncated = truncate_line(cl, max_w.saturating_sub(2));
        lines.push(truncated);
    }
}

fn render_agent_card(lines: &mut Vec<Line>, msg: &ChatMessage, max_w: usize) {
    let agent_name = msg.agent_name.as_deref().unwrap_or("Agent");
    let progress = msg.agent_progress.unwrap_or(0.0);

    // Card top border with agent name and progress
    let bar_width = (max_w as u16).saturating_sub(agent_name.len() as u16 + 10).max(4);
    let progress_str = render_progress_bar(progress, bar_width);

    let title = format!(" {} {} ", agent_name, progress_str);
    let title_len = title.len();
    let border_len = max_w.saturating_sub(2).min(title_len + 4);

    let top_border = format!(
        "┌{}┐",
        "─".repeat(border_len)
    );
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(BORDER),
    )));

    // Title line inside the card
    let title_fill = if title_len < border_len {
        " ".repeat(border_len - title_len)
    } else {
        String::new()
    };
    lines.push(Line::from(vec![
        Span::styled("│", Style::default().fg(BORDER)),
        Span::styled(
            format!(" {}{} ", agent_name, title_fill),
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:3.0}%", progress),
            Style::default().fg(if progress >= 100.0 {
                GREEN
            } else {
                TEXT_MUTED
            }),
        ),
        Span::styled(" │", Style::default().fg(BORDER)),
    ]));

    // Content lines (progress text)
    let content_lines = render_markdown(&msg.content);
    for cl in content_lines {
        let wrap_w = max_w.saturating_sub(4);
        let display = if cl.width() > wrap_w && wrap_w > 10 {
            let s = cl.to_string();
            format!("{}…", &s[..wrap_w.saturating_sub(1)])
        } else {
            cl.to_string()
        };
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(BORDER)),
            Span::styled(display, Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" │", Style::default().fg(BORDER)),
        ]));
    }

    // Tools used
    if !msg.agent_tools.is_empty() {
        let tools_str = msg
            .agent_tools
            .iter()
            .map(|t| {
                if t.target.is_empty() {
                    t.name.clone()
                } else {
                    format!("{} {}", t.name, t.target)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let tools_display = if tools_str.len() > max_w.saturating_sub(10) {
            format!("{}…", &tools_str[..max_w.saturating_sub(13)])
        } else {
            tools_str
        };

        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(BORDER)),
            Span::styled("Tools: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(tools_display, Style::default().fg(CYAN)),
            Span::styled(" │", Style::default().fg(BORDER)),
        ]));
    }

    // Diff summary
    if !msg.agent_diff.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(BORDER)),
            Span::styled("Diff: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                msg.agent_diff.clone(),
                Style::default().fg(GREEN),
            ),
            Span::styled(" │", Style::default().fg(BORDER)),
        ]));
    }

    // Card bottom border
    let bottom_border = format!(
        "└{}┘",
        "─".repeat(border_len)
    );
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(BORDER),
    )));
}

fn render_assistant_message(lines: &mut Vec<Line>, msg: &ChatMessage, max_w: usize) {
    lines.push(Line::from(vec![
        Span::styled(
            " EvAgent ",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ),
    ]));

    let content_lines = render_markdown(&msg.content);
    for cl in content_lines {
        let truncated = truncate_line(cl, max_w.saturating_sub(2));
        lines.push(truncated);
    }
}

fn render_system_message(lines: &mut Vec<Line>, msg: &ChatMessage, max_w: usize) {
    // System messages as horizontal rule style
    let line_len = max_w.min(40);
    lines.push(Line::from(Span::styled(
        "─".repeat(line_len),
        Style::default().fg(BORDER),
    )));

    let content_lines = render_markdown(&msg.content);
    for cl in content_lines {
        let display = if cl.width() > max_w.saturating_sub(4) && max_w > 10 {
            let s = cl.to_string();
            format!("{}…", &s[..max_w.saturating_sub(5)])
        } else {
            cl.to_string()
        };
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                display,
                Style::default()
                    .fg(TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }
}

/// Truncate a Line to fit within max_w characters.
fn truncate_line(line: Line<'static>, max_w: usize) -> Line<'static> {
    if line.width() <= max_w || max_w < 10 {
        return line;
    }
    let s = line.to_string();
    let truncated = format!("{}…", &s[..max_w.saturating_sub(1)]);
    Line::from(Span::raw(truncated))
}
