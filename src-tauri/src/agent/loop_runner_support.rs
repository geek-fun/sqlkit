use serde_json::Value;

use crate::db::AgentDb;

/// A stored message from the DB that can be used for compaction
pub struct StoredMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

/// Load messages for compaction (excludes tool call internal messages)
pub fn load_messages_for_compact(db: &AgentDb, session_id: &str) -> Result<Vec<StoredMessage>, String> {
    let conn = db.0.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
    let mut stmt = conn
        .prepare("SELECT id, role, content, created_at FROM agent_messages WHERE session_id = ?1 ORDER BY created_at ASC")
        .map_err(|e| format!("Failed to prepare: {}", e))?;

    let messages = stmt
        .query_map(rusqlite::params![session_id], |row| {
            Ok(StoredMessage {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to query: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(messages)
}

/// Format a user message for the LLM
pub fn format_user_message(session_id: &str, content: &str) -> Value {
    serde_json::json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "session_id": session_id,
        "role": "user",
        "content": content,
    })
}
