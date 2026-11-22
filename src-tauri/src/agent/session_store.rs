use serde::{Deserialize, Serialize};
use tauri::State;

use data_studio_agent::storage::db::AgentDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub sources: String,
    pub permissions_mode: String,
    pub model_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessageRow {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<AgentToolCallRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCallRow {
    pub id: String,
    pub tool_name: String,
    pub arguments: String,
    pub status: String,
    pub result: Option<String>,
    pub denial: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationRuleRow {
    pub id: String,
    pub session_id: String,
    pub tool_name: String,
    pub action: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachedSourceRow {
    pub id: String,
    pub kind: String,
    pub alias: Option<String>,
    pub name: Option<String>,
    pub database_type: Option<String>,
    pub file_type: Option<String>,
    pub file_path: Option<String>,
    pub connection_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn load_agent_sessions(agent_db: State<'_, AgentDb>) -> Result<Vec<AgentSessionRow>, String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    let mut stmt = conn
        .prepare("SELECT id, title, status, sources, permissions_mode, model_id, created_at, updated_at FROM agent_sessions ORDER BY updated_at DESC")
        .map_err(|e| format!("Failed to prepare: {}", e))?;

    let sessions = stmt
        .query_map([], |row| {
            Ok(AgentSessionRow {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                sources: row.get(3)?,
                permissions_mode: row.get(4)?,
                model_id: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| format!("Failed to query: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

#[tauri::command]
pub fn create_agent_session(
    agent_db: State<'_, AgentDb>,
    title: String,
    sources: Option<String>,
    permissions_mode: Option<String>,
    model_id: Option<String>,
) -> Result<AgentSessionRow, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_ms();
    let sources_str = sources.unwrap_or_else(|| "[]".to_string());
    let perm_mode = permissions_mode.unwrap_or_else(|| "Ask".to_string());

    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    conn.execute(
        "INSERT INTO agent_sessions (id, title, status, sources, permissions_mode, model_id, created_at, updated_at) VALUES (?1, ?2, 'idle', ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, title, sources_str, perm_mode, model_id, now, now],
    )
    .map_err(|e| format!("Failed to insert: {}", e))?;

    Ok(AgentSessionRow {
        id,
        title,
        status: "idle".to_string(),
        sources: sources_str,
        permissions_mode: perm_mode,
        model_id,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub fn update_session_status(
    agent_db: State<'_, AgentDb>,
    session_id: String,
    status: String,
) -> Result<(), String> {
    let now = now_ms();
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    conn.execute(
        "UPDATE agent_sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![status, now, session_id],
    )
    .map_err(|e| format!("Failed to update: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn update_session_meta(
    agent_db: State<'_, AgentDb>,
    session_id: String,
    sources: Option<String>,
    permissions_mode: Option<String>,
    model_id: Option<String>,
) -> Result<(), String> {
    let now = now_ms();
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    if let Some(ref s) = sources {
        conn.execute(
            "UPDATE agent_sessions SET sources = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![s, now, session_id],
        )
        .map_err(|e| format!("Failed to update sources: {}", e))?;
    }
    if let Some(ref pm) = permissions_mode {
        conn.execute(
            "UPDATE agent_sessions SET permissions_mode = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![pm, now, session_id],
        )
        .map_err(|e| format!("Failed to update permissions_mode: {}", e))?;
    }
    if let Some(ref m) = model_id {
        conn.execute(
            "UPDATE agent_sessions SET model_id = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![m, now, session_id],
        )
        .map_err(|e| format!("Failed to update model_id: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn delete_agent_session(
    agent_db: State<'_, AgentDb>,
    session_id: String,
) -> Result<(), String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    conn.execute(
        "DELETE FROM agent_sessions WHERE id = ?1",
        rusqlite::params![session_id],
    )
    .map_err(|e| format!("Failed to delete: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn clear_agent_session_messages(
    agent_db: State<'_, AgentDb>,
    session_id: String,
) -> Result<(), String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    // CASCADE will handle tool_calls and tool_result_store
    conn.execute(
        "DELETE FROM agent_messages WHERE session_id = ?1",
        rusqlite::params![session_id],
    )
    .map_err(|e| format!("Failed to clear messages: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn load_session_messages(
    agent_db: State<'_, AgentDb>,
    session_id: String,
) -> Result<Vec<AgentMessageRow>, String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    load_session_history(&conn, &session_id)
}

fn load_session_history(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<AgentMessageRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, session_id, role, content, created_at FROM agent_messages WHERE session_id = ?1 ORDER BY rowid ASC")
        .map_err(|e| format!("Failed to prepare: {}", e))?;

    let mut messages: Vec<AgentMessageRow> = stmt
        .query_map(rusqlite::params![session_id], |row| {
            Ok(AgentMessageRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
                tool_calls: Vec::new(),
            })
        })
        .map_err(|e| format!("Failed to query: {}", e))?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    let results: std::collections::HashMap<String, String> = messages
        .iter()
        .filter(|message| message.role == "tool")
        .filter_map(|message| {
            let value: serde_json::Value = serde_json::from_str(&message.content).ok()?;
            Some((
                value["tool_call_id"].as_str()?.to_owned(),
                value["content"].as_str()?.to_owned(),
            ))
        })
        .collect();
    let mut stmt = conn.prepare("SELECT t.message_id, t.id, t.tool_name, t.arguments, t.status, (SELECT full_result FROM tool_result_store r WHERE r.tool_call_id=t.id ORDER BY r.rowid DESC LIMIT 1) FROM agent_tool_calls t WHERE t.session_id=?1 ORDER BY t.rowid ASC").map_err(|e| e.to_string())?;
    let calls = stmt
        .query_map([session_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                AgentToolCallRow {
                    id: row.get(1)?,
                    tool_name: row.get(2)?,
                    arguments: row.get(3)?,
                    status: row.get(4)?,
                    result: row.get(5)?,
                    denial: None,
                },
            ))
        })
        .map_err(|e| e.to_string())?;
    for call in calls {
        let (message_id, mut call) = call.map_err(|e| e.to_string())?;
        call.result = results.get(&call.id).cloned().or(call.result);
        if call.status == "failed" && call.result.as_deref() == Some("Tool call was cancelled.") {
            call.status = "interrupted".into();
        }
        if call.status == "denied" {
            let denial = crate::ai_permissions::AccessDenial::from_message(
                call.result.as_deref().unwrap_or("Access denied"),
            );
            call.result = Some(denial.message.clone());
            call.denial = Some(serde_json::to_value(denial).map_err(|e| e.to_string())?);
        }
        if let Some(message) = messages.iter_mut().find(|message| message.id == message_id) {
            message.tool_calls.push(call);
        }
    }

    Ok(messages)
}

#[tauri::command]
pub fn load_confirmation_rules(
    agent_db: State<'_, AgentDb>,
    session_id: String,
) -> Result<Vec<ConfirmationRuleRow>, String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    let mut stmt = conn
        .prepare("SELECT id, session_id, tool_name, action, created_at FROM confirmation_rules WHERE session_id = ?1 ORDER BY created_at")
        .map_err(|e| format!("Failed to prepare: {}", e))?;

    let rules = stmt
        .query_map(rusqlite::params![session_id], |row| {
            Ok(ConfirmationRuleRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                tool_name: row.get(2)?,
                action: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| format!("Failed to query: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rules)
}

#[tauri::command]
pub fn save_confirmation_rule(
    agent_db: State<'_, AgentDb>,
    session_id: String,
    tool_name: String,
    action: String,
) -> Result<ConfirmationRuleRow, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_ms();
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO confirmation_rules (id, session_id, tool_name, action, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, session_id, tool_name, action, now],
    )
    .map_err(|e| format!("Failed to save rule: {}", e))?;

    Ok(ConfirmationRuleRow {
        id,
        session_id,
        tool_name,
        action,
        created_at: now,
    })
}

#[tauri::command]
pub fn delete_confirmation_rule(agent_db: State<'_, AgentDb>, id: String) -> Result<(), String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    conn.execute(
        "DELETE FROM confirmation_rules WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| format!("Failed to delete: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn clear_session_confirmation_rules(
    agent_db: State<'_, AgentDb>,
    session_id: String,
) -> Result<(), String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    conn.execute(
        "DELETE FROM confirmation_rules WHERE session_id = ?1",
        rusqlite::params![session_id],
    )
    .map_err(|e| format!("Failed to clear: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn load_attached_sources(
    agent_db: State<'_, AgentDb>,
) -> Result<Vec<AttachedSourceRow>, String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    let mut stmt = conn
        .prepare("SELECT id, kind, alias, name, database_type, file_type, file_path, connection_id, created_at, updated_at FROM attached_sources ORDER BY updated_at DESC")
        .map_err(|e| format!("Failed to prepare: {}", e))?;

    let sources = stmt
        .query_map([], |row| {
            Ok(AttachedSourceRow {
                id: row.get(0)?,
                kind: row.get(1)?,
                alias: row.get(2)?,
                name: row.get(3)?,
                database_type: row.get(4)?,
                file_type: row.get(5)?,
                file_path: row.get(6)?,
                connection_id: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| format!("Failed to query: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sources)
}

#[tauri::command]
pub fn save_attached_source(
    agent_db: State<'_, AgentDb>,
    id: String,
    kind: String,
    alias: Option<String>,
    name: Option<String>,
    database_type: Option<String>,
    file_type: Option<String>,
    file_path: Option<String>,
    connection_id: Option<String>,
) -> Result<AttachedSourceRow, String> {
    let now = now_ms();
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO attached_sources (id, kind, alias, name, database_type, file_type, file_path, connection_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, COALESCE((SELECT created_at FROM attached_sources WHERE id = ?1), ?9), ?10)",
        rusqlite::params![id, kind, alias, name, database_type, file_type, file_path, connection_id, now, now],
    )
    .map_err(|e| format!("Failed to save: {}", e))?;

    Ok(AttachedSourceRow {
        id,
        kind,
        alias,
        name,
        database_type,
        file_type,
        file_path,
        connection_id,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub fn delete_attached_source(agent_db: State<'_, AgentDb>, id: String) -> Result<(), String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    conn.execute(
        "DELETE FROM attached_sources WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| format!("Failed to delete: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn migrate_session_metadata(
    agent_db: State<'_, AgentDb>,
    session_meta: String,
    confirmation_rules: String,
    _attached_sources: String,
) -> Result<(), String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    // Parse and migrate session metadata from localStorage format
    if let Ok(meta) =
        serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(&session_meta)
    {
        for (session_id, data) in &meta {
            let title = data
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Migrated Session");
            let sources = data
                .get("sources")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string());
            let now = now_ms();
            let _ = conn.execute(
                "INSERT OR IGNORE INTO agent_sessions (id, title, status, sources, created_at, updated_at) VALUES (?1, ?2, 'idle', ?3, ?4, ?5)",
                rusqlite::params![session_id, title, sources, now, now],
            );
        }
    }

    // Parse and migrate confirmation rules
    if let Ok(rules) = serde_json::from_str::<Vec<serde_json::Value>>(&confirmation_rules) {
        for rule in &rules {
            let session_id = rule.get("sessionId").and_then(|v| v.as_str()).unwrap_or("");
            let tool_name = rule.get("toolName").and_then(|v| v.as_str()).unwrap_or("");
            let action = rule
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("deny_always");
            let now = now_ms();
            let id = uuid::Uuid::new_v4().to_string();
            let _ = conn.execute(
                "INSERT OR IGNORE INTO confirmation_rules (id, session_id, tool_name, action, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, session_id, tool_name, action, now],
            );
        }
    }

    Ok(())
}

#[tauri::command]
pub fn recover_stuck_sessions(agent_db: State<'_, AgentDb>) -> Result<(), String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;
    conn.execute(
        "UPDATE agent_sessions SET status = 'idle' WHERE status IN ('running', 'waiting_confirmation')",
        [],
    )
    .map_err(|e| format!("Failed to recover: {}", e))?;
    conn.execute(
        "UPDATE agent_tool_calls SET status = 'interrupted' WHERE status IN ('pending', 'approved')",
        [],
    )
    .map_err(|e| format!("Failed to recover tool calls: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod history_tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[test]
    fn history_restores_outcomes_results_and_denial_context_without_promoting_pending_calls() {
        let db = AgentDb(Arc::new(Mutex::new(
            rusqlite::Connection::open_in_memory().unwrap(),
        )));
        data_studio_agent::storage::db::migrate(&db).unwrap();
        let conn = db.0.lock().unwrap();
        conn.execute("INSERT INTO agent_sessions (id,title,created_at,updated_at) VALUES ('chat','Chat',0,0)", []).unwrap();
        conn.execute("INSERT INTO agent_messages (id,session_id,role,content,created_at) VALUES ('assistant','chat','assistant',?1,0)", [json!({"content":"","tool_calls":[]}).to_string()]).unwrap();
        let denied = crate::ai_permissions::AccessDenial::permission(
            crate::ai_permissions::Permission::Manage,
            "db",
            "connection",
        );
        let outcomes = [
            ("completed", "completed", "Created example".to_owned()),
            ("failed", "failed", "Database execution failed".to_owned()),
            ("blocked", "denied", denied.encode()),
            (
                "rejected",
                "denied",
                "Tool call 'ddl' was denied by the user. Try an alternative approach.".to_owned(),
            ),
        ];
        for (id, status, result) in outcomes {
            conn.execute("INSERT INTO agent_tool_calls (id,message_id,session_id,tool_name,arguments,status,created_at) VALUES (?1,'assistant','chat','ddl','{}',?2,0)", rusqlite::params![id,status]).unwrap();
            conn.execute("INSERT INTO agent_messages (id,session_id,role,content,created_at) VALUES (?1,'chat','tool',?2,0)", rusqlite::params![format!("result-{id}"), json!({"tool_call_id":id,"content":result}).to_string()]).unwrap();
        }
        conn.execute("INSERT INTO agent_tool_calls (id,message_id,session_id,tool_name,arguments,status,created_at) VALUES ('unfinished','assistant','chat','ddl','{}','approved',0)", []).unwrap();
        let messages = load_session_history(&conn, "chat").unwrap();
        let calls = &messages[0].tool_calls;
        assert_eq!(calls.len(), 5);
        assert_eq!(calls[0].status, "completed");
        assert_eq!(calls[0].result.as_deref(), Some("Created example"));
        assert_eq!(calls[1].status, "failed");
        assert_eq!(calls[2].status, "denied");
        assert_eq!(calls[2].denial.as_ref().unwrap()["source"], "connection");
        assert_eq!(calls[2].denial.as_ref().unwrap()["action"], "manage");
        assert_eq!(calls[2].result.as_deref(), Some(denied.message.as_str()));
        assert_eq!(calls[3].denial.as_ref().unwrap()["kind"], "user");
        assert_eq!(calls[4].status, "approved");
        assert!(calls[4].result.is_none());
        assert_eq!(messages[1].id, "result-completed");
    }
}
