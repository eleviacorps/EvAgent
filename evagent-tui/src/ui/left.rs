//! Left column: Agent Tree + Token Usage + Session Stats.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::types::{AgentState, fmt_tokens, fmt_tokens_exact};

// ── Color Palette ──
const BG_DEEP: Color = Color::Rgb(3, 4, 10);
const BG_NAVY: Color = Color::Rgb(5, 7, 19);
const BORDER: Color = Color::Rgb(37, 44, 82);
const TEXT_PRIMARY: Color = Color::Rgb(201, 209, 255);
const TEXT_SECONDARY: Color = Color::Rgb(166, 175, 216);
const TEXT_MUTED: Color = Color::Rgb(126, 136, 181);
const PURPLE: Color = Color::Rgb(197, 111, 255);
const PURPLE_DIM: Color = Color::Rgb(180, 94, 255);
const CYAN: Color = Color::Rgb(57, 216, 255);
const GREEN: Color = Color::Rgb(60, 229, 154);
const RED: Color = Color::Rgb(255, 90, 110);

pub fn draw_left(f: &mut Frame, area: Rect, app: &App) {
    // Vertical split: Agent tree (top 60%) | Token usage (bottom 40%)
    let chunks = Layout::vertical([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ])
    .split(area);

    draw_agent_tree(f, chunks[0], app);
    draw_token_usage(f, chunks[1], app);
}

// ── Agent Tree Panel ──

fn draw_agent_tree(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Agents ", Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let tree_nodes = app.get_agent_tree();

    if tree_nodes.is_empty() {
        let msg = Paragraph::new(Text::from(Line::from(Span::styled(
            " No agents active",
            Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
        ))))
        .style(Style::default().bg(BG_DEEP));
        f.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let mut prev_level = 0usize;
    let mut stack: Vec<bool> = Vec::new(); // track which levels have more siblings

    for (i, node) in tree_nodes.iter().enumerate() {
        // Determine if this node has a sibling after it at the same level
        let has_sibling = tree_nodes[i + 1..]
            .iter()
            .take_while(|n| n.level > node.level)
            .chain(tree_nodes[i + 1..].iter().take(1))
            .any(|n| n.level == node.level);

        // Build the connector prefix
        let mut indent = String::new();

        // Level 0: no indent
        if node.level == 0 {
            // Root node
        } else {
            // Build indentation for each level
            for l in 0..node.level {
                if l < stack.len() {
                    if stack[l] {
                        indent.push_str("│   ");
                    } else {
                        indent.push_str("    ");
                    }
                }
            }

            // Connector
            if has_sibling {
                indent.push_str("├── ");
            } else {
                indent.push_str("└── ");
            }
        }

        // Update stack for this level
        while stack.len() <= node.level {
            stack.push(false);
        }
        // If level went down, mark previous level as having more siblings
        if i > 0 && node.level <= prev_level {
            if let Some(val) = stack.get_mut(node.level) {
                *val = true;
            }
        }
        // Mark current level's sibling status
        if node.level < stack.len() {
            stack[node.level] = has_sibling;
        }

        prev_level = node.level;

        let status_indicator = match node.status {
            AgentState::Completed => ("●", GREEN),
            AgentState::Running => ("▶", CYAN),
            AgentState::Failed => ("●", RED),
            AgentState::Timeout => ("●", RED),
            AgentState::Idle => ("○", TEXT_MUTED),
        };

        let line = Line::from(vec![
            Span::styled(indent, Style::default().fg(BORDER)),
            Span::styled(node.name.clone(), Style::default().fg(TEXT_PRIMARY)),
            Span::raw(" "),
            Span::styled(
                status_indicator.0,
                Style::default().fg(status_indicator.1).add_modifier(Modifier::BOLD),
            ),
        ]);

        lines.push(line);
    }

    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG_DEEP))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

// ── Token Usage Panel ──

fn draw_token_usage(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Resources ", Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_DEEP));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Tokens gauge
    let tokens_pct = if app.token_limit > 0 {
        ((app.stats.total_tokens as f64 / app.token_limit as f64) * 100.0) as u16
    } else {
        0
    };

    let token_label = format!(
        " {} / {} ",
        fmt_tokens_exact(app.stats.total_tokens),
        fmt_tokens_exact(app.token_limit)
    );

    let gauge_tokens = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(PURPLE)
                .bg(BG_NAVY),
        )
        .percent(tokens_pct.min(100))
        .label(Span::styled(token_label, Style::default().fg(TEXT_PRIMARY)));
    f.render_widget(gauge_tokens, Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(2),
        height: 1,
    });

    // Cost gauge
    let cost_max = 0.10; // assume $0.10 max for display purposes
    let cost_pct = ((app.stats.total_cost / cost_max) * 100.0) as u16;
    let cost_label = format!(" ${:.4} ", app.stats.total_cost);

    let gauge_cost = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(CYAN)
                .bg(BG_NAVY),
        )
        .percent(cost_pct.min(100))
        .label(Span::styled(cost_label, Style::default().fg(TEXT_PRIMARY)));
    f.render_widget(gauge_cost, Rect {
        x: inner.x + 1,
        y: inner.y + 2,
        width: inner.width.saturating_sub(2),
        height: 1,
    });

    // Context usage gauge
    let context_pct = tokens_pct.min(100);
    let context_label = format!(" {}% ", context_pct);

    let gauge_context = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(PURPLE_DIM)
                .bg(BG_NAVY),
        )
        .percent(context_pct)
        .label(Span::styled(context_label, Style::default().fg(TEXT_PRIMARY)));
    f.render_widget(gauge_context, Rect {
        x: inner.x + 1,
        y: inner.y + 3,
        width: inner.width.saturating_sub(2),
        height: 1,
    });

    // Session Stats
    let stats_lines = vec![
        Line::from(vec![
            Span::styled(" Agents: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                format!("{}/{}", app.stats.completed_agents, app.stats.total_agents),
                Style::default().fg(if app.stats.total_agents > 0
                    && app.stats.completed_agents == app.stats.total_agents
                {
                    GREEN
                } else {
                    TEXT_PRIMARY
                }),
            ),
            Span::raw("   "),
            Span::styled("Domain: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(&app.domain, Style::default().fg(TEXT_SECONDARY)),
        ]),
        Line::from(vec![
            Span::styled(" Tokens: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                fmt_tokens(app.stats.total_tokens),
                Style::default().fg(TEXT_PRIMARY),
            ),
            Span::raw("   "),
            Span::styled("Cost: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                format!("${:.4}", app.stats.total_cost),
                Style::default().fg(TEXT_SECONDARY),
            ),
        ]),
    ];

    let stats_y = inner.y + 5;
    let stats_area = Rect {
        x: inner.x + 1,
        y: stats_y,
        width: inner.width.saturating_sub(2),
        height: 2.max(inner.height.saturating_sub(6)),
    };

    let stats_para = Paragraph::new(Text::from(stats_lines))
        .style(Style::default().bg(BG_DEEP));
    f.render_widget(stats_para, stats_area);
}
