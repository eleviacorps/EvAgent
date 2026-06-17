//! Helper functions for extracting tool calls, file activities, and progress from text.

use crate::types::{FileActivity, ToolCall};

// ─── Tool Call Extraction ───

/// Try to extract a tool call from an agent's progress text.
pub fn extract_tool_call(agent: &str, text: &str, timestamp: &str, duration: u64) -> Option<ToolCall> {
    let lower = text.to_lowercase();

    let (matched, icon) = if lower.contains("search") || lower.contains("lookup") {
        (true, "🔍")
    } else if lower.contains("write") || lower.contains("save") {
        (true, "📝")
    } else if lower.contains("read") || lower.contains("open") {
        (true, "📖")
    } else if lower.contains("run") || lower.contains("execut") || lower.contains("call") {
        (true, "⚡")
    } else if lower.contains("http") || lower.contains("fetch") || lower.contains("api") {
        (true, "🌐")
    } else if lower.contains("fix") || lower.contains("patch") || lower.contains("edit") {
        (true, "🔧")
    } else {
        (false, "")
    };

    if !matched {
        return None;
    }

    let tool_name = if let Some(pos) = text.find(':') {
        text[..pos].trim().to_string()
    } else if let Some(pos) = text.find("...") {
        text[..pos].trim().to_string()
    } else {
        let parts: Vec<&str> = agent.split('.').collect();
        parts.last().unwrap_or(&agent).to_string()
    };

    let target = if let Some(start) = text.find('"') {
        if let Some(end) = text[start + 1..].find('"') {
            text[start + 1..start + 1 + end].to_string()
        } else {
            String::new()
        }
    } else if let Some(pos) = text.to_lowercase().find(" on ") {
        text[pos + 4..].trim().to_string()
    } else if let Some(pos) = text.to_lowercase().find(" in ") {
        text[pos + 4..].trim().to_string()
    } else {
        String::new()
    };

    Some(ToolCall {
        icon: icon.to_string(),
        tool_name: tool_name.chars().take(18).collect(),
        target: target.chars().take(24).collect(),
        timestamp: timestamp.to_string(),
        duration_ms: duration,
    })
}

// ─── File Activity Extraction ───

/// Try to extract a file activity from progress text.
pub fn extract_file_activity(text: &str, timestamp: &str) -> Option<FileActivity> {
    let lower = text.to_lowercase();

    let (matched, action) = if lower.contains("writing") || lower.contains("saving") {
        (true, "✚ created")
    } else if lower.contains("reading") || lower.contains("opening") {
        (true, "○ read")
    } else if lower.contains("modifying") || lower.contains("editing") || lower.contains("updating") || lower.contains("patching") {
        (true, "◈ modified")
    } else if lower.contains("deleting") || lower.contains("removing") {
        (true, "✕ deleted")
    } else {
        (false, "")
    };

    if !matched {
        return None;
    }

    let path = if let Some(start) = text.find('"') {
        if let Some(end) = text[start + 1..].find('"') {
            text[start + 1..start + 1 + end].to_string()
        } else {
            text[start + 1..].to_string()
        }
    } else if let Some(pos) = text.to_lowercase().find(" on ") {
        text[pos + 4..].trim().to_string()
    } else if let Some(pos) = text.to_lowercase().find(": ") {
        text[pos + 2..].trim().to_string()
    } else {
        text.chars().take(30).collect()
    };

    Some(FileActivity {
        path: path.chars().take(30).collect(),
        action: action.to_string(),
        timestamp: timestamp.to_string(),
    })
}

// ─── Progress Parsing ───

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
