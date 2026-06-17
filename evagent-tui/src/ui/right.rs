//! Right column: Summary panel — simplified.
//!
//! Shows key metrics at a glance: agent counts, tokens, cost.
//! No more separate tool_calls or file_activity panels.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::types::{fmt_cost, fmt_tokens, AgentState};

// ── Color Palette ──
const BG_DEEP: Color = Color::Rgb(0, 0, 0);
const BORDER: Color = Color::Rgb(35, 42, 54);
const TEXT_PRIMARY: Color = Color::Rgb(215, 220, 229);
const TEXT_MUTED: Color = Color::Rgb(127, 136, 150);
const CYAN: Color = Color::Rgb(79, 195, 247);
const GREEN: Color = Color::Rgb(74, 222, 128);

pub fn draw_right(f: &mut Frame, area: Rect, app: &App) {
    // Vertical split: Summary (top 100%)
    draw_summary(f, area, app);
}

fn draw_summary(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(
            " Summary ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let tokens_str = fmt_tokens(app.stats.total_tokens);
    let cost_str = fmt_cost(app.stats.total_cost);

    let running = app
        .active_agents
        .iter()
        .filter(|a| a.status == AgentState::Running)
        .count();
    let completed = app
        .active_agents
        .iter()
        .filter(|a| a.status == AgentState::Completed)
        .count();
    let total = app.active_agents.len();

    let mut lines = Vec::new();

    // Agent count
    lines.push(Line::from(vec![
        Span::styled(" Agents: ", Style::default().fg(TEXT_MUTED)),
        Span::styled(
            total.to_string(),
            Style::default().fg(TEXT_PRIMARY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   Running: ", Style::default().fg(TEXT_MUTED)),
        Span::styled(
            running.to_string(),
            Style::default().fg(CYAN),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   Done: ", Style::default().fg(TEXT_MUTED)),
        Span::styled(
            completed.to_string(),
            Style::default().fg(GREEN),
        ),
    ]));
    lines.push(Line::from(""));

    // Tokens
    lines.push(Line::from(vec![
        Span::styled(" Tokens: ", Style::default().fg(TEXT_MUTED)),
        Span::styled(tokens_str, Style::default().fg(TEXT_PRIMARY)),
    ]));

    // Cost
    lines.push(Line::from(vec![
        Span::styled(" Cost: ", Style::default().fg(TEXT_MUTED)),
        Span::styled(cost_str, Style::default().fg(TEXT_PRIMARY)),
    ]));
    lines.push(Line::from(""));

    // Domain
    lines.push(Line::from(vec![
        Span::styled(" Domain: ", Style::default().fg(TEXT_MUTED)),
        Span::styled(&app.domain, Style::default().fg(CYAN)),
    ]));

    // Available agents
    if !app.available_agents.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Available Agents",
            Style::default().fg(TEXT_MUTED).add_modifier(Modifier::BOLD),
        )));
        for agent_name in app.available_agents.iter().take(5) {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default().fg(TEXT_MUTED)),
                Span::styled(agent_name, Style::default().fg(TEXT_MUTED)),
            ]));
        }
        if app.available_agents.len() > 5 {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  ... and {} more", app.available_agents.len() - 5),
                    Style::default().fg(TEXT_MUTED),
                ),
            ]));
        }
    }

    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG_DEEP))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}
