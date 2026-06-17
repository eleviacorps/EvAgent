//! Right column: Tool Calls + File Activity.
//!
//! Shows tracked tool calls and file operations with icons, timestamps, and durations.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::types::fmt_duration;

// ── Color Palette ──
const BG_DEEP: Color = Color::Rgb(0, 0, 0);
const BORDER: Color = Color::Rgb(37, 44, 82);
const TEXT_PRIMARY: Color = Color::Rgb(201, 209, 255);
const TEXT_SECONDARY: Color = Color::Rgb(166, 175, 216);
const TEXT_MUTED: Color = Color::Rgb(126, 136, 181);
const PURPLE: Color = Color::Rgb(197, 111, 255);
const CYAN: Color = Color::Rgb(57, 216, 255);
const GREEN: Color = Color::Rgb(60, 229, 154);
const AMBER: Color = Color::Rgb(255, 200, 87);

pub fn draw_right(f: &mut Frame, area: Rect, app: &App) {
    // Vertical split: Tool Calls (top 60%) | File Activity (bottom 40%)
    let chunks = Layout::vertical([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ])
    .split(area);

    draw_tool_calls(f, chunks[0], app);
    draw_file_activity(f, chunks[1], app);
}

// ── Tool Calls Panel ──

fn draw_tool_calls(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(
            " Tool Calls ",
            Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::RIGHT | Borders::TOP)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.tool_calls.is_empty() {
        let msg = Paragraph::new(Text::from(Line::from(Span::styled(
            " No tool calls yet",
            Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
        ))))
        .style(Style::default().bg(BG_DEEP));
        f.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let max_visible = inner.height.saturating_sub(1) as usize;

    // Show recent tool calls (from end of vec)
    let start = if app.tool_calls.len() > max_visible {
        app.tool_calls.len() - max_visible
    } else {
        0
    };

    let tool_width = inner.width.saturating_sub(18) as usize;

    for tc in app.tool_calls[start..].iter() {
        let tool_name = if tc.tool_name.len() > tool_width && tool_width > 5 {
            format!("{}…", &tc.tool_name[..tool_width.saturating_sub(1)])
        } else {
            tc.tool_name.clone()
        };

        let target = if !tc.target.is_empty() {
            if tc.target.len() > tool_width.saturating_sub(4) && tool_width > 8 {
                format!("…{}", &tc.target[tc.target.len().saturating_sub(tool_width.saturating_sub(5))..])
            } else {
                tc.target.clone()
            }
        } else {
            String::new()
        };

        let duration_str = if tc.duration_ms > 0 {
            fmt_duration(tc.duration_ms)
        } else {
            String::new()
        };

        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", tc.icon),
                Style::default().fg(AMBER),
            ),
            Span::styled(
                format!(" {:<12}", tool_name),
                Style::default().fg(CYAN),
            ),
            Span::styled(
                if target.is_empty() {
                    String::new()
                } else {
                    format!(" {}", target)
                },
                Style::default().fg(TEXT_SECONDARY),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" {}", tc.timestamp),
                Style::default().fg(TEXT_MUTED),
            ),
            Span::raw(" "),
            Span::styled(
                duration_str,
                Style::default().fg(TEXT_MUTED),
            ),
        ]);

        lines.push(line);
    }

    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG_DEEP))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

// ── File Activity Panel ──

fn draw_file_activity(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(
            " File Activity ",
            Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.file_activities.is_empty() {
        let msg = Paragraph::new(Text::from(Line::from(Span::styled(
            " No file activity",
            Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
        ))))
        .style(Style::default().bg(BG_DEEP));
        f.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let max_visible = inner.height.saturating_sub(1) as usize;

    let start = if app.file_activities.len() > max_visible {
        app.file_activities.len() - max_visible
    } else {
        0
    };

    for fa in app.file_activities[start..].iter() {
        let path_width = inner.width.saturating_sub(14) as usize;
        let path = if fa.path.len() > path_width && path_width > 5 {
            format!("{}…", &fa.path[..path_width.saturating_sub(1)])
        } else {
            fa.path.clone()
        };

        let (action_style, action_color) = match fa.action.as_str() {
            "✚ created" => (Style::default(), GREEN),
            "◈ modified" => (Style::default(), AMBER),
            "○ read" => (Style::default(), CYAN),
            "✕ deleted" => (Style::default(), Color::Rgb(255, 90, 110)),
            _ => (Style::default(), TEXT_MUTED),
        };

        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", fa.action),
                action_style.fg(action_color),
            ),
            Span::styled(
                path,
                Style::default().fg(TEXT_PRIMARY),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" {}", fa.timestamp),
                Style::default().fg(TEXT_MUTED),
            ),
        ]);

        lines.push(line);
    }

    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG_DEEP))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}
