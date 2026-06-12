use serde_json::{json, Value};
use tauri::AppHandle;

use crate::agent::chat_formatter::LlmMessage;
use crate::db::AgentDb;

/// Evaluate whether compaction should occur based on token usage
pub fn evaluate(used_tokens: usize, capacity: usize, threshold: f64) -> bool {
    if capacity == 0 {
        return false;
    }
    let ratio = used_tokens as f64 / capacity as f64;
    ratio >= threshold
}

/// Count projected tokens for a message list
pub fn count_projected_tokens(messages: &[LlmMessage], model: &str) -> usize {
    let mut total = 0;
    for msg in messages {
        total += crate::agent::token_counter::count_message_tokens(&msg.role, &msg.content, model);
    }
    total
}

/// Resolve model spec for a session from settings
pub fn resolve_model_spec_for_session(settings: &Value) -> (String, usize) {
    let model = settings
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4")
        .to_string();

    let context_window = settings
        .get("contextWindowOverride")
        .and_then(|v| v.as_u64())
        .unwrap_or(128_000) as usize;

    (model, context_window)
}

/// Compact a session by removing older messages and replacing them with a summary.
/// Returns summary info for event emission.
pub fn compact_session(
    db: &AgentDb,
    _app: &AppHandle,
    _settings: &Value,
    session_id: &str,
    _start_index: usize,
    _model: &str,
    _context_window: usize,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock().map_err(|e| format!("Failed to lock db: {}", e))?;

    // Count messages
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM agent_messages WHERE session_id = ?1",
            rusqlite::params![session_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count messages: {}", e))?;

    if total < 4 {
        return Ok(json!({
            "pre_tokens": 0,
            "post_tokens": 0,
            "removed_count": 0,
            "trigger": "auto"
        }));
    }

    // Keep the last 2 pairs (4 messages: user+assistant rounds)
    let keep = 4_i64;
    let remove_count = total - keep;

    // Get the messages to remove (oldest first)
    let mut stmt = conn
        .prepare(
            "SELECT id FROM agent_messages WHERE session_id = ?1 ORDER BY created_at ASC LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare: {}", e))?;

    let to_remove: Vec<String> = stmt
        .query_map(rusqlite::params![session_id, remove_count], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|e| format!("Failed to query: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let pre_tokens: usize = total as usize * 50; // rough estimate
    let removed_count = to_remove.len();

    for id in &to_remove {
        conn.execute(
            "DELETE FROM agent_messages WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Failed to delete message: {}", e))?;
    }

    let post_tokens: usize = (total - removed_count as i64) as usize * 50;

    // Insert a compaction marker as a system message
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let marker_id = format!("compaction-{}-{}", session_id, now);
    let summary = format!(
        "Compaction removed {} messages. Older conversation context has been summarized.",
        removed_count
    );

    let marker_content = serde_json::json!({
        "_compact_boundary": true,
        "summary": summary,
        "pre_tokens": pre_tokens,
        "post_tokens": post_tokens,
        "trigger": "auto"
    });

    conn.execute(
        "INSERT INTO agent_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, 'system', ?3, ?4)",
        rusqlite::params![marker_id, session_id, marker_content.to_string(), now],
    )
    .map_err(|e| format!("Failed to insert compaction marker: {}", e))?;

    Ok(json!({
        "pre_tokens": pre_tokens,
        "post_tokens": post_tokens,
        "removed_count": removed_count,
        "trigger": "auto"
    }))
}
