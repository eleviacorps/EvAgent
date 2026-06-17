//! Markdown-to-ratatui Line renderer.
//!
//! Handles: **bold**, *italic*, `code`, # headers, - lists,
//! ``` code blocks, --- horizontal rules, [links](url).

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

// ── Colors ──
const CYAN: Color = Color::Rgb(79, 195, 247);
const GREEN: Color = Color::Rgb(74, 222, 128);
const AMBER: Color = Color::Rgb(251, 191, 36);
const TEXT_PRIMARY: Color = Color::Rgb(215, 220, 229);
const TEXT_MUTED: Color = Color::Rgb(127, 136, 150);
const BG_CODE: Color = Color::Rgb(14, 17, 24);

/// Render markdown text into a vector of ratatui `Line`s.
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_block_content = String::new();

    for raw_line in text.lines() {
        // Handle code block fences
        if raw_line.trim_start().starts_with("```") {
            if in_code_block {
                // End code block
                lines.push(render_code_block(&code_block_content));
                code_block_content.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
                code_block_content.clear();
            }
            continue;
        }

        if in_code_block {
            if !code_block_content.is_empty() {
                code_block_content.push('\n');
            }
            code_block_content.push_str(raw_line);
            continue;
        }

        let trimmed = raw_line.trim();

        // Horizontal rule: --- or ___ or ***
        if trimmed == "---" || trimmed == "___" || trimmed == "***" {
            lines.push(render_horizontal_rule());
            continue;
        }

        // Headers: # ## ###
        if let Some(h_content) = trimmed.strip_prefix("# ") {
            lines.push(render_header(1, h_content));
            continue;
        }
        if let Some(h_content) = trimmed.strip_prefix("## ") {
            lines.push(render_header(2, h_content));
            continue;
        }
        if let Some(h_content) = trimmed.strip_prefix("### ") {
            lines.push(render_header(3, h_content));
            continue;
        }
        if let Some(h_content) = trimmed.strip_prefix("#### ") {
            lines.push(render_header(4, h_content));
            continue;
        }
        if let Some(h_content) = trimmed.strip_prefix("##### ") {
            lines.push(render_header(5, h_content));
            continue;
        }

        // List items: - or *
        if let Some(item) = trimmed.strip_prefix("- ") {
            lines.push(render_list_item(item));
            continue;
        }
        if let Some(item) = trimmed.strip_prefix("* ") {
            lines.push(render_list_item(item));
            continue;
        }

        // Regular paragraph (handle inline formatting)
        let spans = parse_inline(raw_line);
        if spans.is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(spans));
        }
    }

    // Close any unclosed code block
    if in_code_block && !code_block_content.is_empty() {
        lines.push(render_code_block(&code_block_content));
    }

    lines
}

/// Parse inline formatting within a line: **bold**, *italic*, `code`, [links](url).
fn parse_inline(text: &str) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Check for link [text](url)
        if i + 1 < len && chars[i] == '[' {
            if let Some(close_bracket) = text[i..].find(']') {
                let abs_close = i + close_bracket;
                if abs_close + 1 < len && chars[abs_close + 1] == '(' {
                    if let Some(close_paren) = text[abs_close + 1..].find(')') {
                        let abs_close_paren = abs_close + 1 + close_paren;
                        let link_text = &text[i + 1..abs_close];
                        let _link_url = &text[abs_close + 2..abs_close_paren];
                        spans.push(Span::styled(
                            link_text.to_string(),
                            Style::default()
                                .fg(CYAN)
                                .add_modifier(Modifier::UNDERLINED),
                        ));
                        i = abs_close_paren + 1;
                        continue;
                    }
                }
            }
        }

        // Check for **bold**
        if i + 1 < len && chars[i] == '*' && i + 1 < len && chars[i + 1] == '*' {
            let start = i + 2;
            if let Some(end) = text[start..].find("**") {
                let content = &text[start..start + end];
                spans.push(Span::styled(
                    content.to_string(),
                    Style::default()
                        .fg(TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ));
                i = start + end + 2;
                continue;
            }
        }

        // Check for *italic* (single asterisk, not followed by another *)
        if chars[i] == '*' && i + 1 < len && chars[i + 1] != '*' {
            let start = i + 1;
            if let Some(end) = text[start..].find('*') {
                // Make sure it's not **
                if start + end + 1 >= len || chars[start + end] != '*' {
                    let content = &text[start..start + end];
                    spans.push(Span::styled(
                        content.to_string(),
                        Style::default()
                            .fg(TEXT_PRIMARY)
                            .add_modifier(Modifier::ITALIC),
                    ));
                    i = start + end + 1;
                    continue;
                }
            }
        }

        // Check for `code`
        if chars[i] == '`' {
            let start = i + 1;
            if let Some(end) = text[start..].find('`') {
                let content = &text[start..start + end];
                spans.push(Span::styled(
                    content.to_string(),
                    Style::default().fg(CYAN).bg(BG_CODE),
                ));
                i = start + end + 1;
                continue;
            }
        }

        // Regular character
        // Collect consecutive non-special chars for efficiency
        let mut chunk = String::new();
        while i < len {
            let c = chars[i];
            if c == '*' || c == '`' || c == '[' {
                break;
            }
            chunk.push(c);
            i += 1;
        }
        if !chunk.is_empty() {
            // Check for emoji and other special rendering
            spans.push(Span::styled(chunk, Style::default().fg(TEXT_PRIMARY)));
        }
    }

    spans
}

/// Render a header line.
fn render_header(level: usize, content: &str) -> Line<'static> {
    let prefix = match level {
        1 => "█ ",
        2 => "▌ ",
        3 => "▪ ",
        _ => "  ",
    };
    let color = match level {
        1 => CYAN,
        2 => GREEN,
        3 => AMBER,
        _ => TEXT_PRIMARY,
    };

    let inner = parse_inline(content);
    let mut spans = vec![
        Span::styled(prefix, Style::default().fg(color).add_modifier(Modifier::BOLD)),
    ];
    spans.extend(inner);

    Line::from(spans)
}

/// Render a list item.
fn render_list_item(content: &str) -> Line<'static> {
    let mut spans = vec![
        Span::styled(" • ", Style::default().fg(TEXT_MUTED)),
    ];
    spans.extend(parse_inline(content));
    Line::from(spans)
}

/// Render a horizontal rule (─ line).
fn render_horizontal_rule() -> Line<'static> {
    Line::from(Span::styled(
        "─".repeat(60),
        Style::default().fg(Color::Rgb(35, 42, 54)),
    ))
}

/// Render a code block (multi-line, gray background).
fn render_code_block(content: &str) -> Line<'static> {
    // Code blocks are rendered as a single line in the conversation stream
    // For simplicity, we put a code marker and the content
    let display = if content.len() > 80 {
        format!("{}…", &content[..77])
    } else {
        content.to_string()
    };
    Line::from(Span::styled(
        format!(" ▌ {}", display),
        Style::default().fg(CYAN).bg(BG_CODE),
    ))
}

/// Format agent progress as a visual bar string like "████░░░░ 68%"
pub fn render_progress_bar(pct: f32, width: u16) -> String {
    if width < 6 {
        return format!("{}%", pct as u8);
    }
    let bar_w = (width as usize).saturating_sub(5).max(2);
    let filled = ((pct / 100.0) * bar_w as f32).round() as usize;
    let empty = bar_w.saturating_sub(filled);

    let bar: String = std::iter::repeat('█')
        .take(filled)
        .chain(std::iter::repeat('░').take(empty))
        .collect();

    format!("{} {:3.0}%", bar, pct)
}
