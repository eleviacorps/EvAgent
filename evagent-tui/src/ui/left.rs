//! Left column: Simplified agent status panel.
//! No mission, no lifecycle, no gauges — just active agent list.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::AgentState;

const BG_DEEP: Color = Color::Rgb(0, 0, 0);
const BORDER: Color = Color::Rgb(35, 42, 54);
const TEXT_PRIMARY: Color = Color::Rgb(215, 220, 229);
const TEXT_MUTED: Color = Color::Rgb(127, 136, 150);
const CYAN: Color = Color::Rgb(79, 195, 247);
const GREEN: Color = Color::Rgb(74, 222, 128);
const RED: Color = Color::Rgb(239, 68, 68);

pub fn draw_left(f: &mut Frame, area: Rect, app: &App) {
    // Vertical split: Agents (top 70%) | Info (bottom 30%)
    let chunks = Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)]).split(area);
    draw_agents(f, chunks[0], app);
    draw_info(f, chunks[1], app);
}

fn draw_agents(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Agents ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)))
        .borders(Borders::LEFT | Borders::TOP)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.active_agents.is_empty() {
        let msg = Paragraph::new(Text::from(Line::from(Span::styled(
            " No agents active",
            Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
        ))))
        .style(Style::default().bg(BG_DEEP));
        f.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    for agent in app.active_agents.iter().take(20) {
        let (icon, color) = match agent.status {
            AgentState::Running => ("▶", CYAN),
            AgentState::Completed => ("✓", GREEN),
            AgentState::Failed => ("✗", RED),
            AgentState::Timeout => ("✗", RED),
            AgentState::Idle => ("○", TEXT_MUTED),
        };
        let name_w = inner.width.saturating_sub(6) as usize;
        let name = if agent.agent_name.len() > name_w && name_w > 3 {
            format!("{}…", &agent.agent_name[..name_w.saturating_sub(1)])
        } else {
            agent.agent_name.clone()
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", icon), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled(name, Style::default().fg(TEXT_PRIMARY)),
        ]));
    }

    let para = Paragraph::new(Text::from(lines)).style(Style::default().bg(BG_DEEP));
    f.render_widget(para, inner);
}

fn draw_info(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Info ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)))
        .borders(Borders::LEFT | Borders::BOTTOM)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled(" Agents: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(format!("{}/{}", app.stats.completed_agents, app.stats.total_agents), Style::default().fg(TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled(" Tokens: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(format!("{}", app.stats.total_tokens), Style::default().fg(TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled(" Domain: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(&app.domain, Style::default().fg(CYAN)),
        ]),
    ];

    let para = Paragraph::new(Text::from(lines)).style(Style::default().bg(BG_DEEP));
    f.render_widget(para, inner);
}
