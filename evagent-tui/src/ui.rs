//! Beautiful, OpenCode-quality terminal UI for EvAgent.
//! Dark navy theme, clean minimal borders, professional status indicators.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Gauge, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::App;
use crate::types::{AgentState, ConnectionState, domain_color};

// ── Color Palette ──────────────────────────────────────────────────────────
// Dark navy professional theme
const BG: Color = Color::Rgb(18, 18, 30);         // near-black navy
const SURFACE: Color = Color::Rgb(30, 30, 50);    // card surfaces
const BORDER: Color = Color::Rgb(60, 60, 90);     // subtle borders
const TEXT_PRIMARY: Color = Color::Rgb(220, 220, 240);
const TEXT_SECONDARY: Color = Color::Rgb(150, 150, 180);
const TEXT_MUTED: Color = Color::Rgb(100, 100, 130);
const ACCENT: Color = Color::Rgb(120, 140, 255);  // soft blue accent
const SUCCESS: Color = Color::Rgb(80, 200, 120);
const WARNING: Color = Color::Rgb(255, 200, 60);
const ERROR: Color = Color::Rgb(255, 80, 80);
const INFO: Color = Color::Rgb(100, 180, 255);

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Main vertical: top panel + bottom input
    let main = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(3),
    ])
    .split(area);

    // Top: sidebar + content
    let top = Layout::horizontal([
        Constraint::Length(26),
        Constraint::Min(40),
    ])
    .split(main[0]);

    draw_sidebar(f, top[0], app);
    draw_content(f, top[1], app);
    draw_input(f, main[1], app);
}

// ── Sidebar ────────────────────────────────────────────────────────────────

fn draw_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let color = domain_color(&app.stats.domain);
    let blocks_visible = area.height >= 14;

    let block = Block::default()
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if !blocks_visible { return; }

    // ── Header / Domain Badge ──
    let badge_bg = if app.connection_status == ConnectionState::Connected { color } else { Color::Rgb(60, 60, 80) };
    let badge = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(" {} ", app.stats.domain.to_uppercase()),
            Style::default().fg(Color::Black).bg(badge_bg).add_modifier(Modifier::BOLD),
        )),
    ];
    let badge_para = Paragraph::new(Text::from(badge));
    f.render_widget(badge_para, Rect { x: inner.x + 1, y: inner.y, width: inner.width.saturating_sub(2), height: 3 });

    // ── Connection Status ──
    let (status_dot, status_text, status_color) = match app.connection_status {
        ConnectionState::Connected => ("●", "Connected", SUCCESS),
        ConnectionState::Connecting => ("◐", "Connecting...", WARNING),
        ConnectionState::Disconnected => ("○", "Disconnected", ERROR),
    };
    let status = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!(" {} ", status_dot), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            Span::styled(status_text, Style::default().fg(TEXT_SECONDARY)),
        ]),
    ];
    let status_para = Paragraph::new(Text::from(status));
    f.render_widget(status_para, Rect { x: inner.x + 1, y: inner.y + 3, width: inner.width.saturating_sub(2), height: 2 });

    // ── Stats ──
    let stats = vec![
        Line::from(Span::styled("── Stats ──", Style::default().fg(TEXT_MUTED))),
        Line::from(Span::styled(
            format!(" ▸ Tokens  {}", fmt_tokens(app.stats.total_tokens)),
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            format!(" ▸ Cost    ${:.4}", app.stats.total_cost),
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            format!(" ▸ Agents  {}/{}", app.stats.completed_agents, app.stats.total_agents),
            Style::default().fg(if app.stats.total_agents > 0 && app.stats.completed_agents == app.stats.total_agents { SUCCESS } else { TEXT_PRIMARY }),
        )),
    ];
    let stats_para = Paragraph::new(Text::from(stats));
    f.render_widget(stats_para, Rect { x: inner.x + 1, y: inner.y + 5, width: inner.width.saturating_sub(2), height: 5 });

    // ── Agents List ──
    if !app.available_agents.is_empty() {
        let mut agent_lines = vec![
            Line::from(Span::styled("── Agents ──", Style::default().fg(TEXT_MUTED))),
        ];
        let max_agents = (inner.height.saturating_sub(15) as usize).min(6);
        for agent in app.available_agents.iter().take(max_agents) {
            agent_lines.push(Line::from(Span::styled(
                format!(" • {}", agent),
                Style::default().fg(TEXT_SECONDARY),
            )));
        }
        if app.available_agents.len() > 6 {
            agent_lines.push(Line::from(Span::styled(
                format!(" +{} more", app.available_agents.len() - 6),
                Style::default().fg(TEXT_MUTED),
            )));
        }
        let agents_para = Paragraph::new(Text::from(agent_lines));
        f.render_widget(agents_para, Rect { x: inner.x + 1, y: inner.y + 10, width: inner.width.saturating_sub(2), height: inner.height.saturating_sub(10).min(8) });
    }
}

// ── Content (Chat + Agent Panel) ───────────────────────────────────────────

fn draw_content(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Percentage(55),
        Constraint::Percentage(45),
    ])
    .split(area);

    draw_chat(f, chunks[0], app);
    draw_agent_panel(f, chunks[1], app);
}

fn draw_chat(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Chat ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    if app.chat_messages.is_empty() {
        match app.connection_status {
            ConnectionState::Connected => {
                lines.push(Line::from(Span::styled(
                    "  Welcome to EvAgent. Type a prompt to dispatch agents.",
                    Style::default().fg(TEXT_MUTED).italic(),
                )));
            }
            ConnectionState::Connecting => {
                lines.push(Line::from(Span::styled(
                    "  ⚡ Connecting to EvAgent core engine...",
                    Style::default().fg(WARNING),
                )));
                lines.push(Line::from(Span::styled(
                    "  Ensure evagent-core is running on ws://127.0.0.1:9753",
                    Style::default().fg(TEXT_MUTED).italic(),
                )));
            }
            ConnectionState::Disconnected => {
                lines.push(Line::from(Span::styled(
                    "  ⚠ Connection lost. Reconnecting...",
                    Style::default().fg(ERROR),
                )));
                lines.push(Line::from(Span::styled(
                    "  Check that evagent-core is still running.",
                    Style::default().fg(TEXT_MUTED).italic(),
                )));
            }
        }
    } else {
        for msg in app.chat_messages.iter().rev().take(100).rev() {
            let (prefix, style) = match msg.role.as_str() {
                "user" => ("┃ ", Style::default().fg(ACCENT)),
                "assistant" => ("┃ ", Style::default().fg(SUCCESS)),
                "system" => ("  ", Style::default().fg(TEXT_MUTED).italic()),
                _ => ("  ", Style::default().fg(TEXT_SECONDARY)),
            };

            for (i, line_str) in msg.content.lines().enumerate() {
                if i == 0 {
                    let p = prefix;
                    let s = style;
                    // Don't show raw error text — clean it up
                    let cleaned = if line_str.contains("⚠️") || line_str.contains("Error:") {
                        format!("Error occurred")
                    } else {
                        line_str.to_string()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(p, s),
                        Span::styled(cleaned, Style::default().fg(TEXT_PRIMARY)),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("   {}", line_str),
                        Style::default().fg(TEXT_SECONDARY),
                    )));
                }
            }
        }
    }

    // Fill remaining space
    let available = inner.height.saturating_sub(2);
    while lines.len() < available as usize {
        lines.push(Line::from(""));
    }

    let para = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

// ── Agent Panel ────────────────────────────────────────────────────────────

fn draw_agent_panel(f: &mut Frame, area: Rect, app: &App) {
    let running = app.active_agents.iter().filter(|a| a.status == AgentState::Running).count();
    let is_active = running > 0;

    let title = if is_active {
        format!(" Agents Running ({}) ", running)
    } else if !app.active_agents.is_empty() {
        format!(" Agents ({}/{}) ", app.stats.completed_agents, app.active_agents.len())
    } else {
        " Agents ".to_string()
    };

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(if is_active { WARNING } else { ACCENT }).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.active_agents.is_empty() {
        let msg = Paragraph::new(Text::from(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No agents dispatched yet.",
                Style::default().fg(TEXT_MUTED).italic(),
            )),
            Line::from(Span::styled(
                "  Type a prompt in the input bar below.",
                Style::default().fg(TEXT_MUTED).italic(),
            )),
        ]));
        f.render_widget(msg, inner);
        return;
    }

    // Render agent cards in a grid
    let max_visible = (inner.height / 3).max(1) as usize;
    let cols = 1.max(inner.width as usize / 30);
    let cards_per_row = cols;

    let mut y_offset = inner.y;

    for (idx, agent) in app.active_agents.iter().take(max_visible).enumerate() {
        if y_offset + 3 > inner.y + inner.height { break; }

        let card_width = (inner.width / cards_per_row as u16).max(24);
        let x_pos = inner.x + (idx as u16 % cards_per_row as u16) * card_width;
        let card_area = Rect { x: x_pos, y: y_offset, width: card_width, height: 3 };

        let (icon, status_color, bar_color) = match agent.status {
            AgentState::Running => ("▶", WARNING, ACCENT),
            AgentState::Completed => ("✔", SUCCESS, SUCCESS),
            AgentState::Failed => ("✘", ERROR, ERROR),
            AgentState::Timeout => ("⏱", ERROR, ERROR),
            AgentState::Idle => ("○", TEXT_MUTED, TEXT_MUTED),
        };

        // Card background
        let card = Block::default()
            .style(Style::default().bg(SURFACE));

        let card_inner = card.inner(card_area);
        f.render_widget(card, card_area);

        // Line 1: Icon + Name + Tokens
        let name_short = if agent.agent_name.len() > (card_width as usize).saturating_sub(12) {
        let max_len = (card_width as usize).saturating_sub(14);
        if max_len < 3 { agent.agent_name.clone() }
        else { format!("{}…", &agent.agent_name[..max_len]) }
        } else {
            agent.agent_name.clone()
        };

        let line1 = Line::from(vec![
            Span::styled(format!(" {} ", icon), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            Span::styled(name_short, Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!(" {}t", fmt_tokens(agent.tokens_used)),
                Style::default().fg(TEXT_MUTED),
            ),
        ]);
        f.render_widget(Paragraph::new(Text::from(vec![line1])), card_inner);

        // Line 2: Simple progress bar
        let bar_width = card_inner.width.saturating_sub(2) as usize;
        let filled = ((agent.progress / 100.0) * bar_width as f32) as usize;
        let filled_s = "━".repeat(filled.min(bar_width));
        let empty_s = "━".repeat(bar_width.saturating_sub(filled));
        let bar_line = Line::from(vec![
            Span::styled(filled_s, Style::default().fg(bar_color)),
            Span::styled(empty_s, Style::default().fg(BORDER)),
            Span::styled(format!(" {:3.0}%", agent.progress), Style::default().fg(TEXT_MUTED)),
        ]);

        // Clean progress text
        let progress_clean = if agent.progress_text.contains("⚠️") || agent.progress_text.contains("Error:") {
            "Processing...".to_string()
        } else if agent.progress_text.len() > bar_width.saturating_sub(4) {
            format!("{}…", &agent.progress_text[..bar_width.saturating_sub(6)])
        } else {
            agent.progress_text.clone()
        };

        let bar_info = vec![
            bar_line,
            Line::from(Span::styled(progress_clean, Style::default().fg(TEXT_MUTED).italic())),
        ];
        f.render_widget(Paragraph::new(Text::from(bar_info)), Rect { x: card_inner.x, y: card_inner.y + 1, width: card_inner.width, height: 2 });

        if (idx + 1) % cards_per_row == 0 {
            y_offset += 3;
        }
    }
}

// ── Input Bar ──────────────────────────────────────────────────────────────

fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let connected = app.connection_status == ConnectionState::Connected;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let prefix = Span::styled("> ", Style::default().fg(if connected { SUCCESS } else { ERROR }).add_modifier(Modifier::BOLD));

    let (text, style) = if !connected {
        ("Reconnecting...".to_string(), Style::default().fg(TEXT_MUTED).italic())
    } else if app.input.is_empty() {
        ("Dispatch agents across domains...".to_string(), Style::default().fg(TEXT_MUTED).italic())
    } else {
        (app.input.clone(), Style::default().fg(TEXT_PRIMARY))
    };

    let line = Line::from(vec![prefix, Span::styled(text, style)]);
    let para = Paragraph::new(Text::from(vec![line]));
    f.render_widget(para, inner);

    // Cursor
    if !app.input.is_empty() && connected {
        f.set_cursor_position((
            inner.x + 2 + (app.input_cursor as u16).min(app.input.len() as u16).min(inner.width.saturating_sub(3)),
            inner.y + 1,
        ));
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn fmt_tokens(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}
