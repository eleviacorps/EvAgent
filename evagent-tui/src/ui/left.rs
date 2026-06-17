//! Left column: Mission + Active Agents + Agent Lifecycle Rail.
//! No gauges, no token usage, no cost display.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::types::{AgentState, LifecyclePhase};

// ── Color Palette ──
const BG_DEEP: Color = Color::Rgb(9, 11, 17);
const BORDER: Color = Color::Rgb(35, 42, 54);
const TEXT_PRIMARY: Color = Color::Rgb(215, 220, 229);
const TEXT_MUTED: Color = Color::Rgb(127, 136, 150);
const CYAN: Color = Color::Rgb(79, 195, 247);
const GREEN: Color = Color::Rgb(74, 222, 128);
const AMBER: Color = Color::Rgb(251, 191, 36);
const RED: Color = Color::Rgb(239, 68, 68);

pub fn draw_left(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Min(3),    // Mission (flex, min 3)
        Constraint::Length(1), // separator
        Constraint::Min(4),    // Active Agents (flex, min 4)
        Constraint::Length(1), // separator
        Constraint::Length(4), // Lifecycle Rail (fixed 4)
    ])
    .split(area);

    draw_mission(f, chunks[0], app);
    draw_active_agents(f, chunks[2], app);
    draw_lifecycle_rail(f, chunks[4], app);
}

// ── Mission Panel ──

fn draw_mission(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(
            " Mission ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mission_text = if app.mission_text.is_empty() {
        "Awaiting your prompt..."
    } else {
        &app.mission_text
    };

    // Truncate to fit
    let max_w = inner.width.saturating_sub(2) as usize;
    let display = if mission_text.len() > max_w && max_w > 5 {
        format!("{}…", &mission_text[..max_w.saturating_sub(1)])
    } else {
        mission_text.to_string()
    };

    let para = Paragraph::new(Text::from(Line::from(Span::styled(
        display,
        Style::default().fg(TEXT_PRIMARY),
    ))))
    .style(Style::default().bg(BG_DEEP))
    .wrap(Wrap { trim: false });

    let text_area = Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(2),
        height: inner.height.saturating_sub(2).max(1),
    };
    f.render_widget(para, text_area);
}

// ── Active Agents Panel ──

fn draw_active_agents(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(
            " Active Agents ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    if app.active_agents.is_empty() {
        lines.push(Line::from(Span::styled(
            " No active agents",
            Style::default()
                .fg(TEXT_MUTED)
                .add_modifier(Modifier::ITALIC),
        )));
    } else {
        for agent in &app.active_agents {
            let (indicator, color) = match agent.status {
                AgentState::Running => ("●", CYAN),
                AgentState::Completed => ("✓", GREEN),
                AgentState::Failed => ("●", RED),
                AgentState::Timeout => ("●", AMBER),
                AgentState::Idle => ("○", TEXT_MUTED),
            };

            let name_display = agent
                .agent_name
                .split('.')
                .last()
                .unwrap_or(&agent.agent_name);

            let status_text = match agent.status {
                AgentState::Running => "running",
                AgentState::Completed => "complete",
                AgentState::Failed => "failed",
                AgentState::Timeout => "timeout",
                AgentState::Idle => "idle",
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", indicator),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    name_display.to_string(),
                    Style::default().fg(TEXT_PRIMARY),
                ),
                Span::styled(
                    format!(" ({})", status_text),
                    Style::default().fg(TEXT_MUTED),
                ),
            ]));

            // Add progress text if available and not empty
            if !agent.progress_text.is_empty() && agent.progress_text != name_display {
                let progress_display = if agent.progress_text.len() > 30 {
                    format!("{}…", &agent.progress_text[..27])
                } else {
                    agent.progress_text.clone()
                };
                lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(progress_display, Style::default().fg(TEXT_MUTED)),
                ]));
            }
        }
    }

    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG_DEEP))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

// ── Agent Lifecycle Rail ──

fn draw_lifecycle_rail(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(
            " Lifecycle ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let phases = LifecyclePhase::all();
    let current_phase = &app.lifecycle_phase;
    let current_idx = current_phase.index();

    let max_w = inner.width.saturating_sub(2) as usize;
    let mut spans: Vec<Span> = Vec::new();

    for (i, phase) in phases.iter().enumerate() {
        let (indicator, color) = if i < current_idx {
            ("✓", GREEN)
        } else if i == current_idx {
            ("●", CYAN)
        } else {
            ("○", TEXT_MUTED)
        };

        let label = phase.as_str();
        let text = format!(" {} {}", indicator, label);

        spans.push(Span::styled(
            text,
            Style::default().fg(color).add_modifier(if i == current_idx {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ));

        // Add arrow between phases (if fits)
        if i < phases.len() - 1 {
            let arrow = " → ";
            if spans.iter().map(|s| s.width()).sum::<usize>() + arrow.len() < max_w {
                spans.push(Span::styled(arrow, Style::default().fg(TEXT_MUTED)));
            } else {
                spans.push(Span::styled(" ", Style::default().fg(TEXT_MUTED)));
            }
        }
    }

    let para = Paragraph::new(Text::from(Line::from(spans)))
        .style(Style::default().bg(BG_DEEP));

    let content_area = Rect {
        x: inner.x + 1,
        y: inner.y + (inner.height.saturating_sub(1)) / 2,
        width: inner.width.saturating_sub(2),
        height: 1,
    };
    f.render_widget(para, content_area);
}
