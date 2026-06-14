use serde_json::Value;
use tauri::AppHandle;

use crate::db::AgentDb;

pub fn append(
    db: &AgentDb,
    _app: &AppHandle,
    _settings: &Value,
    id: &str,
    session_id: &str,
    role: &str,
    content: &str,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let conn =
        db.0.lock()
            .map_err(|e| format!("Failed to lock db: {}", e))?;
    conn.execute(
        "INSERT INTO agent_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, session_id, role, content, now],
    )
    .map_err(|e| format!("Failed to insert message: {}", e))?;

    // Also update session updated_at
    conn.execute(
        "UPDATE agent_sessions SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, session_id],
    )
    .map_err(|e| format!("Failed to update session: {}", e))?;

    Ok(())
}

pub fn get_session_messages(
    db: &AgentDb,
    session_id: &str,
) -> Result<Vec<(String, String, String, i64)>, String> {
    let conn =
        db.0.lock()
            .map_err(|e| format!("Failed to lock db: {}", e))?;
    let mut stmt = conn
        .prepare("SELECT id, role, content, created_at FROM agent_messages WHERE session_id = ?1 ORDER BY created_at ASC")
        .map_err(|e| format!("Failed to prepare: {}", e))?;

    let messages = stmt
        .query_map(rusqlite::params![session_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| format!("Failed to query: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(messages)
}

pub fn store_tool_call(
    db: &AgentDb,
    id: &str,
    message_id: &str,
    session_id: &str,
    tool_name: &str,
    arguments: &str,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let conn =
        db.0.lock()
            .map_err(|e| format!("Failed to lock db: {}", e))?;
    conn.execute(
        "INSERT INTO agent_tool_calls (id, message_id, session_id, tool_name, arguments, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
        rusqlite::params![id, message_id, session_id, tool_name, arguments, now],
    )
    .map_err(|e| format!("Failed to insert tool call: {}", e))?;

    Ok(())
}

pub fn store_tool_result(
    db: &AgentDb,
    id: &str,
    tool_call_id: &str,
    full_result: &str,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let conn =
        db.0.lock()
            .map_err(|e| format!("Failed to lock db: {}", e))?;
    conn.execute(
        "INSERT INTO tool_result_store (id, tool_call_id, full_result, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, tool_call_id, full_result, now],
    )
    .map_err(|e| format!("Failed to insert tool result: {}", e))?;

    Ok(())
}

pub fn get_tool_result(db: &AgentDb, tool_call_id: &str) -> Result<Option<String>, String> {
    let conn =
        db.0.lock()
            .map_err(|e| format!("Failed to lock db: {}", e))?;
    let result = conn
        .query_row(
            "SELECT full_result FROM tool_result_store WHERE tool_call_id = ?1 ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![tool_call_id],
            |row| row.get(0),
        )
        .ok();
    Ok(result)
}
