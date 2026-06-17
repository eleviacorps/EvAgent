//! Simplified extraction helpers.
//! No longer extracts ToolCall/FileActivity as separate types — those are
//! embedded inline in AgentStatus during update_from_ws in app.rs.

use crate::types::ToolInfo;

/// Extract a percentage (0.0–100.0) from a progress string like "70%" or "searching... 55%".
pub fn parse_progress(text: &str) -> f32 {
    if let Some(percent_str) = text.split('%').next() {
        if let Some(last_num) = percent_str.split_whitespace().last() {
            if let Ok(val) = last_num.parse::<f32>() {
                return val.clamp(0.0, 100.0);
            }
        }
    }
    if let Some(pos) = text.find('%') {
        let before = &text[..pos];
        let num_part = before.split_whitespace().last().unwrap_or("");
        if let Ok(val) = num_part.parse::<f32>() {
            return val.clamp(0.0, 100.0);
        }
    }
    let lower = text.to_lowercase();
    if lower.contains("complete") || lower.contains("done") || lower.contains("finished") {
        100.0
    } else if lower.contains("search") || lower.contains("research") {
        50.0
    } else if lower.contains("start") || lower.contains("begin") {
        10.0
    } else {
        0.0
    }
}

/// Try to extract a tool name from progress text.
/// Returns `ToolInfo` if a tool-like pattern is found, None otherwise.
pub fn extract_tool_info(text: &str) -> Option<ToolInfo> {
    let lower = text.to_lowercase();

    let name = if lower.contains("search") || lower.contains("lookup") {
        "Search"
    } else if lower.contains("write") || lower.contains("save") {
        "WriteFile"
    } else if lower.contains("read") || lower.contains("open") {
        "ReadFile"
    } else if lower.contains("run") || lower.contains("execut") || lower.contains("call") {
        "RunCommand"
    } else if lower.contains("http") || lower.contains("fetch") || lower.contains("api") {
        "HttpRequest"
    } else if lower.contains("fix") || lower.contains("patch") || lower.contains("edit") {
        "EditFile"
    } else if lower.contains("think") || lower.contains("analyze") || lower.contains("reason") {
        "Think"
    } else if lower.contains("plan") || lower.contains("design") {
        "Plan"
    } else {
        return None;
    };

    let target = if let Some(start) = text.find('"') {
        if let Some(end) = text[start + 1..].find('"') {
            text[start + 1..start + 1 + end].to_string()
        } else {
            text[start + 1..].trim().to_string()
        }
    } else if let Some(pos) = text.to_lowercase().find(" on ") {
        text[pos + 4..].trim().to_string()
    } else if let Some(pos) = text.to_lowercase().find(" in ") {
        text[pos + 4..].trim().to_string()
    } else if let Some(pos) = text.find(':') {
        text[pos + 1..].trim().to_string()
    } else {
        String::new()
    };

    Some(ToolInfo {
        name: name.to_string(),
        target: target.chars().take(24).collect(),
    })
}

/// Extract a diff summary like "+28 -6" from text.
pub fn extract_diff_summary(text: &str) -> String {
    // Look for patterns like "+N -M" or "+N lines -M lines"
    let mut result = String::new();
    let mut has_plus = false;
    let mut has_minus = false;

    for word in text.split_whitespace() {
        if word.starts_with('+') && word[1..].chars().all(|c| c.is_ascii_digit()) {
            if !has_plus {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(word);
                has_plus = true;
            }
        } else if word.starts_with('-') && word[1..].chars().all(|c| c.is_ascii_digit()) {
            if !has_minus {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(word);
                has_minus = true;
            }
        }
    }

    // If no explicit diff pattern, check for "additions"/"deletions"
    if result.is_empty() && (text.contains("diff") || text.contains("+") && text.contains("-")) {
        for word in text.split_whitespace() {
            if word.starts_with('+') || word.starts_with('-') {
                let clean: String = word.chars().take(6).collect();
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(&clean);
            }
        }
    }

    result
}
