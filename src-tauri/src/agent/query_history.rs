use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::AgentDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub id: String,
    pub timestamp: i64,
    pub database_type: Option<String>,
    pub sql: Option<String>,
    pub connection_name: String,
    pub connection_id: String,
    pub duration_ms: Option<i64>,
    pub row_count: Option<i64>,
    pub starred: bool,
}

#[tauri::command]
pub fn load_query_history(agent_db: State<'_, AgentDb>) -> Result<Vec<QueryHistoryEntry>, String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, database_type, sql, connection_name, connection_id, \
             duration_ms, row_count, starred \
             FROM query_history ORDER BY timestamp DESC LIMIT 200",
        )
        .map_err(|e| format!("Failed to prepare: {}", e))?;

    let entries = stmt
        .query_map([], |row| {
            Ok(QueryHistoryEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                database_type: row.get(2)?,
                sql: row.get(3)?,
                connection_name: row.get(4)?,
                connection_id: row.get(5)?,
                duration_ms: row.get(6)?,
                row_count: row.get(7)?,
                starred: row.get::<_, i32>(8)? != 0,
            })
        })
        .map_err(|e| format!("Failed to query: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

#[tauri::command]
pub fn add_query_history_entry(
    agent_db: State<'_, AgentDb>,
    input: QueryHistoryEntry,
) -> Result<QueryHistoryEntry, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let id = uuid::Uuid::new_v4().to_string();

    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO query_history (id, timestamp, database_type, sql, connection_name, connection_id, duration_ms, row_count, starred) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
        rusqlite::params![
            id,
            now,
            input.database_type,
            input.sql.as_deref().unwrap_or(""),
            input.connection_name,
            input.connection_id,
            input.duration_ms,
            input.row_count,
        ],
    )
    .map_err(|e| format!("Failed to insert: {}", e))?;

    Ok(QueryHistoryEntry {
        id,
        timestamp: now,
        ..input
    })
}
