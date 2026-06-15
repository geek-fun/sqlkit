use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::agent::chat_formatter::{ChatFormatter, LlmMessage, OpenAIChatFormatter};
use crate::agent::config::{build_headers, get_base_url};
use crate::agent::loop_runner_support::{
    load_all_messages, load_messages_for_compact, new_id, now_ms, post_chat_completions_compact,
    StoredMessage,
};
use crate::agent::model_registry::{
    apply_overrides, resolve_spec, usable_window, ModelSpec, TokenizerFamily,
};
use crate::agent::token_counter::{
    count_chat_messages, count_tools_tokens, estimate_stored_message,
};
use crate::common::http_client::create_http_client;
use crate::db::AgentDb;

pub const DEFAULT_COMPACT_RATIO: f64 = 0.75;
pub const SAFETY_BUFFER_TOKENS: usize = 13_000;
pub const KEEP_LAST_PAIRS: usize = 4;
#[allow(dead_code)]
pub const MAX_CONSECUTIVE_FAILURES: u32 = 3;

#[derive(Debug, Clone, Copy)]
pub struct CompactDecision {
    pub capacity: usize,
    pub trigger_at: usize,
    pub should_compact: bool,
}

#[derive(Debug, Clone)]
pub struct CompactionInfo {
    pub trigger: String,
    pub pre_tokens: usize,
    pub post_tokens: usize,
    pub removed_count: usize,
    pub fallback_keep_pairs: Option<usize>,
}

pub fn evaluate(messages: &[StoredMessage], spec: &ModelSpec) -> CompactDecision {
    let used: usize = messages
        .iter()
        .map(|m| estimate_stored_message(&m.role, &m.content, spec))
        .sum();
    let capacity = usable_window(spec);
    let trigger_at = compact_trigger_threshold(capacity);
    CompactDecision {
        capacity,
        trigger_at,
        should_compact: used >= trigger_at,
    }
}

pub fn count_projected_tokens(
    messages: &[StoredMessage],
    system_prompt: Option<&str>,
    tools: Option<&Value>,
    spec: &ModelSpec,
) -> usize {
    let mut chat_msgs: Vec<Value> = Vec::new();
    if let Some(sp) = system_prompt {
        chat_msgs.push(json!({"role": "system", "content": sp}));
    }
    for m in messages {
        chat_msgs.push(json!({"role": m.role, "content": m.content}));
    }
    let msg_tokens = count_chat_messages(&chat_msgs, spec);
    let tool_tokens = tools.map(|t| count_tools_tokens(t, spec)).unwrap_or(0);
    msg_tokens + tool_tokens
}

pub fn compact_trigger_threshold(capacity: usize) -> usize {
    let by_ratio = ((capacity as f64) * DEFAULT_COMPACT_RATIO) as usize;
    let by_buffer = capacity.saturating_sub(SAFETY_BUFFER_TOKENS);
    by_ratio.min(by_buffer).max(1)
}

pub fn resolve_model_spec(settings: &Value) -> ModelSpec {
    let provider = settings
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("OPEN_AI");
    let model = settings
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4o-mini");
    let override_window = settings
        .get("contextWindowOverride")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);
    apply_overrides(resolve_spec(provider, model), override_window)
}

static SESSION_TOKENIZER_CACHE: OnceLock<Mutex<HashMap<String, TokenizerFamily>>> = OnceLock::new();

pub fn resolve_model_spec_for_session(session_id: &str, settings: &Value) -> ModelSpec {
    let mut spec = resolve_model_spec(settings);
    let cache = SESSION_TOKENIZER_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().expect("tokenizer cache poisoned");
    match map.get(session_id) {
        Some(locked) => {
            spec.tokenizer = *locked;
        }
        None => {
            map.insert(session_id.to_string(), spec.tokenizer);
        }
    }
    spec
}

fn assistant_has_tool_calls(message: &StoredMessage) -> bool {
    message.role == "assistant"
        && serde_json::from_str::<Value>(&message.content)
            .ok()
            .and_then(|v| v.get("tool_calls").cloned())
            .and_then(|tc| tc.as_array().map(|a| !a.is_empty()))
            .unwrap_or(false)
}

fn is_safe_boundary(messages: &[StoredMessage], split: usize) -> bool {
    if split == 0 || split > messages.len() {
        return false;
    }
    let curr_role = messages.get(split).map(|m| m.role.as_str()).unwrap_or("");
    let prev = &messages[split - 1];
    curr_role != "tool" && !assistant_has_tool_calls(prev)
}

pub fn safe_split_index(messages: &[StoredMessage], proposed_split: usize) -> usize {
    let mut split = proposed_split.min(messages.len());
    while split > 0 && !is_safe_boundary(messages, split) {
        split -= 1;
    }
    split
}

pub fn safe_split_index_forward(messages: &[StoredMessage], proposed_split: usize) -> usize {
    let start = proposed_split.min(messages.len());
    (start..=messages.len())
        .find(|split| is_safe_boundary(messages, *split))
        .unwrap_or(0)
}

pub fn target_split_keeping_pairs(messages: &[StoredMessage], keep_pairs: usize) -> usize {
    let mut pairs_seen = 0usize;
    for (idx, m) in messages.iter().enumerate().rev() {
        if m.role == "user" {
            pairs_seen += 1;
            if pairs_seen >= keep_pairs {
                return idx;
            }
        }
    }
    0
}

pub const COMPACT_SYSTEM_PROMPT: &str =
    "Summarize this conversation so it can continue without the full history.

Output exactly this Markdown structure, keeping all sections even if empty:

## What We Were Doing
- [The user's goal and current task — one or two sentences]

## What We Found / Did
- [Key results, queries run, data discovered, decisions made]

## Next Steps
- [What to do next, or \"(none)\"]

## Critical Details
- [Exact names to preserve: connections, indexes, fields, query strings, error messages]

Rules:
- Bullets only, no prose.
- Preserve exact identifiers verbatim.
- Do not mention this summary process.";

pub async fn summarize_with_llm(
    messages_to_summarize: &[StoredMessage],
    settings: &Value,
) -> Result<String, String> {
    let chat_msgs: Vec<LlmMessage> = messages_to_summarize
        .iter()
        .map(|m| LlmMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    let formatter = OpenAIChatFormatter;
    let body = formatter
        .format_messages(&chat_msgs, COMPACT_SYSTEM_PROMPT)
        .map_err(|e| format!("Failed to format compact request: {}", e))?;

    let base_url = get_base_url(settings);
    let headers = build_headers(settings)?;
    let http_proxy = settings
        .get("httpProxy")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let proxy_mode = settings
        .get("proxyMode")
        .and_then(|v| v.as_str())
        .unwrap_or("system");
    let http_client = create_http_client(proxy_mode, http_proxy, None, None);

    let resp = post_chat_completions_compact(&http_client, &base_url, headers, body).await?;
    let payload: Value = resp.json().await.map_err(|e| e.to_string())?;
    let summary = payload
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if summary.is_empty() {
        return Err("LLM returned empty summary".to_string());
    }
    Ok(summary)
}

pub fn build_boundary_payload(
    summary: &str,
    pre_tokens: usize,
    post_tokens: usize,
    trigger: &str,
) -> String {
    let compacted_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    json!({
        "_compact_boundary": true,
        "trigger": trigger,
        "summary": summary,
        "pre_tokens": pre_tokens,
        "post_tokens": post_tokens,
        "compacted_at": compacted_at,
    })
    .to_string()
}

pub async fn run_compact_with_events(
    session_id: &str,
    settings: &Value,
    db: &AgentDb,
    app: &AppHandle,
) -> Result<Option<CompactionInfo>, String> {
    run_compact_inner(session_id, settings, db, Some(app), "auto", false).await
}

pub async fn run_compact_manual(
    session_id: &str,
    settings: &Value,
    db: &AgentDb,
    app: &AppHandle,
) -> Result<Option<CompactionInfo>, String> {
    run_compact_inner(session_id, settings, db, Some(app), "manual", true).await
}

async fn run_compact_inner(
    session_id: &str,
    settings: &Value,
    db: &AgentDb,
    app: Option<&AppHandle>,
    trigger: &str,
    force: bool,
) -> Result<Option<CompactionInfo>, String> {
    let messages = if force {
        load_all_messages(db, session_id)?
    } else {
        load_messages_for_compact(db, session_id)?
    };
    let spec = resolve_model_spec_for_session(session_id, settings);
    let decision = evaluate(&messages, &spec);
    if !force && !decision.should_compact {
        return Ok(None);
    }

    let keep_candidates: [usize; 3] = [KEEP_LAST_PAIRS, 2, 1];
    let split_result = keep_candidates.iter().find_map(|keep_pairs| {
        let proposed = target_split_keeping_pairs(&messages, *keep_pairs);
        let split = safe_split_index(&messages, proposed);
        (split > 0).then_some((split, *keep_pairs, proposed))
    });

    let (split, fallback_keep_pairs) = if let Some((split, keep_pairs, _)) = split_result {
        let fallback = (keep_pairs != KEEP_LAST_PAIRS).then_some(keep_pairs);
        (split, fallback)
    } else {
        let fallback_keep = 1usize;
        let fallback_proposed = target_split_keeping_pairs(&messages, fallback_keep);
        let forward_split = safe_split_index_forward(&messages, fallback_proposed);
        if forward_split == 0 {
            let warning_message = "Context compaction failed: history has too many consecutive tool calls — consider clearing the session or asking a more focused question";
            if let Some(app) = app {
                let _ = app.emit(
                    "agent-loop-warning",
                    json!({
                        "session_id": session_id,
                        "warning": warning_message,
                    }),
                );
            }
            return Err(warning_message.to_string());
        }
        (forward_split, Some(fallback_keep))
    };

    if split == 0 {
        return Err("compact: cannot find safe split".to_string());
    }

    let to_summarize = &messages[..split];
    let pre_tokens: usize = to_summarize
        .iter()
        .map(|m| estimate_stored_message(&m.role, &m.content, &spec))
        .sum();
    let post_tokens: usize = messages[split..]
        .iter()
        .map(|m| estimate_stored_message(&m.role, &m.content, &spec))
        .sum();

    if let Some(app) = app {
        let _ = app.emit(
            "agent-loop-compacting",
            json!({
                "session_id": session_id,
                "phase": "start",
            }),
        );
    }
    let summary_result = summarize_with_llm(to_summarize, settings).await;
    if let Some(app) = app {
        let _ = app.emit(
            "agent-loop-compacting",
            json!({
                "session_id": session_id,
                "phase": "end",
            }),
        );
    }
    let summary = summary_result?;
    let payload = build_boundary_payload(&summary, pre_tokens, post_tokens, trigger);
    let ids_to_remove: Vec<String> = to_summarize.iter().map(|m| m.id.clone()).collect();
    let removed_count = ids_to_remove.len();
    insert_compact_boundary(db, session_id, &ids_to_remove, &payload)?;

    Ok(Some(CompactionInfo {
        trigger: trigger.to_string(),
        pre_tokens,
        post_tokens,
        removed_count,
        fallback_keep_pairs,
    }))
}

fn insert_compact_boundary(
    db: &AgentDb,
    session_id: &str,
    _removed_ids: &[String],
    boundary_payload: &str,
) -> Result<(), String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let boundary_ts: i64 = now_ms();

    tx.execute(
        "INSERT INTO agent_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![new_id(), session_id, "system", boundary_payload, boundary_ts],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ── Backward-compatible wrappers for existing callers ──────────────────

pub fn resolve_model_spec_for_session_old(settings: &Value) -> (String, usize) {
    let spec = resolve_model_spec_for_session("legacy", settings);
    (spec.model_id, spec.context_window)
}

pub fn evaluate_old(used_tokens: usize, capacity: usize, threshold: f64) -> bool {
    if capacity == 0 {
        return false;
    }
    let ratio = used_tokens as f64 / capacity as f64;
    ratio >= threshold
}

pub fn count_projected_tokens_old(messages: &[LlmMessage], model: &str) -> usize {
    let mut total = 0;
    for msg in messages {
        total += crate::agent::token_counter::count_message_tokens(&msg.role, &msg.content, model);
    }
    total
}

pub fn compact_session(
    db: &AgentDb,
    _app: &AppHandle,
    _settings: &Value,
    session_id: &str,
    _start_index: usize,
    _model: &str,
    _context_window: usize,
) -> Result<serde_json::Value, String> {
    let conn =
        db.0.lock()
            .map_err(|e| format!("Failed to lock db: {}", e))?;
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM agent_messages WHERE session_id = ?1",
            rusqlite::params![session_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count messages: {}", e))?;

    if total < 4 {
        return Ok(json!({
            "pre_tokens": 0, "post_tokens": 0, "removed_count": 0, "trigger": "auto"
        }));
    }

    let keep = 4_i64;
    let remove_count = total - keep;
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

    let pre_tokens: usize = total as usize * 50;
    let removed_count = to_remove.len();
    for id in &to_remove {
        conn.execute(
            "DELETE FROM agent_messages WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Failed to delete message: {}", e))?;
    }

    let now = now_ms();
    let marker_id = new_id();
    let marker_content = json!({
        "_compact_boundary": true,
        "summary": format!("Compaction removed {} messages.", removed_count),
        "pre_tokens": pre_tokens,
        "post_tokens": (total - removed_count as i64) as usize * 50,
        "trigger": "auto"
    });
    conn.execute(
        "INSERT INTO agent_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, 'system', ?3, ?4)",
        rusqlite::params![marker_id, session_id, marker_content.to_string(), now],
    )
    .map_err(|e| format!("Failed to insert compaction marker: {}", e))?;

    Ok(json!({
        "pre_tokens": pre_tokens,
        "post_tokens": (total - removed_count as i64) as usize * 50,
        "removed_count": removed_count,
        "trigger": "auto"
    }))
}
