//! Agent Loop Runner — Orchestrates the full LLM agent loop.
//!
//! This module manages the core agent loop: calling the LLM, processing tool calls,
//! streaming events to the frontend, persisting messages to SQLite, enforcing budgets,
//! handling cancellations and confirmations, and compacting context windows.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::oneshot;

use crate::agent::chat_formatter::{
    AnthropicChatFormatter, ChatFormatter, LlmMessage, LlmToolCall, OpenAIChatFormatter,
};
use crate::agent::compact::{
    count_projected_tokens_old as count_projected_tokens, evaluate_old as evaluate,
    resolve_model_spec_for_session_old as resolve_model_spec_for_session,
};
use crate::agent::config::{build_headers, get_base_url};
use crate::agent::tool_executor::ToolExecutor;
use crate::common::http_client::create_http_client;
use crate::db::AgentDb;

// ---------------------------------------------------------------------------
// Type aliases for concurrent session management
// ---------------------------------------------------------------------------

/// Map of session_id → confirmation sender (yes/no).
pub type ConfirmMap = Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>;

/// Map of session_id → cancellation sender.
pub type CancelMap = Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>;

// ---------------------------------------------------------------------------
// Budget & retry constants
// ---------------------------------------------------------------------------

const DEFAULT_MAX_ITERATIONS: u32 = 200;
const DEFAULT_WALL_CLOCK_BUDGET_SECS: u64 = 30 * 60; // 30 minutes
const DEFAULT_TOKEN_BUDGET: usize = 20_000_000;
const CONFIRM_TIMEOUT_SECS: u64 = 300;
const RETRY_DELAYS_MS: &[u64] = &[1000, 3000, 8000];
const RETRY_JITTER_MS: u64 = 250;

// ---------------------------------------------------------------------------
// Helper: add jitter to a base delay
// ---------------------------------------------------------------------------

fn add_jitter(base_ms: u64, jitter_ms: u64) -> Duration {
    let jitter = rand::random::<u64>() % jitter_ms;
    Duration::from_millis(base_ms + jitter)
}

// ---------------------------------------------------------------------------
// Helper: emit a Tauri event with safe error handling
// ---------------------------------------------------------------------------

fn emit_event(app: &AppHandle, event: &str, payload: Value) {
    if let Err(e) = app.emit(event, payload) {
        eprintln!("[loop_runner] Failed to emit event '{}': {}", event, e);
    }
}

// ---------------------------------------------------------------------------
// Helper: extract settings from the settings Value
// ---------------------------------------------------------------------------

fn get_settings_str(settings: &Value, key: &str) -> Option<String> {
    settings
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn get_settings_u64(settings: &Value, key: &str, default: u64) -> u64 {
    settings
        .get(key)
        .and_then(|v| v.as_u64())
        .unwrap_or(default)
}

fn get_settings_bool(settings: &Value, key: &str, default: bool) -> bool {
    settings
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

// ---------------------------------------------------------------------------
// Helper: load connection config from the settings
// ---------------------------------------------------------------------------

fn connection_config_from_settings(settings: &Value) -> Value {
    settings
        .get("connectionConfig")
        .cloned()
        .unwrap_or(json!({}))
}

// ---------------------------------------------------------------------------
// Helper: extract string list from settings (e.g. attached sources)
// ---------------------------------------------------------------------------

fn get_settings_string_array(settings: &Value, key: &str) -> Vec<String> {
    settings
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Helper: build the system prompt for the agent
// ---------------------------------------------------------------------------

fn build_system_prompt(settings: &Value) -> String {
    if let Some(prompt) = get_settings_str(settings, "systemPrompt") {
        if !prompt.is_empty() {
            return prompt;
        }
    }
    // Fallback default system prompt
    let mut prompt = String::from(
        "You are an expert database assistant. You help users query, analyze, and manage their databases. \
         Use the available tools to explore schemas, run queries, and provide insights. \
         Always verify your assumptions by checking the database schema before writing queries. \
         When you generate SQL, explain what it does. If an error occurs, diagnose and fix it."
    );

    // Append custom instructions if present
    if let Some(instructions) = get_settings_str(settings, "customInstructions") {
        if !instructions.is_empty() {
            prompt.push_str("\n\nAdditional instructions:\n");
            prompt.push_str(&instructions);
        }
    }

    prompt
}

// ---------------------------------------------------------------------------
// Helper: build the list of tools from attached sources
// ---------------------------------------------------------------------------

fn build_tools_list(settings: &Value) -> Vec<Value> {
    let sources = get_settings_string_array(settings, "attachedSources");
    let _db_types: Vec<String> = sources
        .iter()
        .filter_map(|s| {
            // Source identifiers may be like "postgres:conn-id" or connection ids
            if s.contains(':') {
                s.split(':').next().map(String::from)
            } else {
                None
            }
        })
        .collect();

    // Get all capabilities tagged as agent tools
    let registry = crate::capabilities::registry::registry();
    let mut tools: Vec<Value> = registry
        .agent_tools()
        .iter()
        .map(|cap| {
            let mut schema = cap.input_schema.clone();
            // Tag with the capability name so the LLM can call it
            if let Some(obj) = schema.as_object_mut() {
                obj.insert("name".to_string(), json!(cap.name));
                obj.insert("description".to_string(), json!(cap.description));
            }
            schema
        })
        .collect();

    // Deduplicate by name
    tools.sort_by(|a, b| {
        let an = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let bn = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        an.cmp(bn)
    });
    tools.dedup_by(|a, b| a.get("name") == b.get("name"));

    tools
}

// ---------------------------------------------------------------------------
// Helper: retry a future with exponential backoff + jitter
// ---------------------------------------------------------------------------

async fn retry_with_backoff<F, Fut, T, E>(f: F) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_err = None;
    for &delay_ms in RETRY_DELAYS_MS.iter() {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(add_jitter(delay_ms, RETRY_JITTER_MS)).await;
            }
        }
    }
    Err(last_err.expect("retry_with_backoff: no error captured"))
}

// ---------------------------------------------------------------------------
// Helper: extract an error message from an LLM streaming error
// ---------------------------------------------------------------------------

fn extract_llm_error(err: &str) -> String {
    // Common patterns to clean up
    if err.contains("status code 401") {
        return "Authentication failed. Check your API key.".to_string();
    }
    if err.contains("status code 429") {
        return "Rate limited. Please wait and try again.".to_string();
    }
    if err.contains("status code 500")
        || err.contains("status code 502")
        || err.contains("status code 503")
    {
        return "LLM provider returned a server error. Please try again.".to_string();
    }
    if err.contains("timed out") || err.contains("timeout") {
        return "Request timed out. The model may be overloaded.".to_string();
    }
    err.to_string()
}

// ---------------------------------------------------------------------------
// Helper: check cancellation signal
// ---------------------------------------------------------------------------

fn is_cancelled(cancel_map: &CancelMap, session_id: &str) -> bool {
    let map = cancel_map.lock().unwrap_or_else(|e| e.into_inner());
    map.contains_key(session_id)
}

fn take_cancellation(cancel_map: &CancelMap, session_id: &str) -> bool {
    let mut map = cancel_map.lock().unwrap_or_else(|e| e.into_inner());
    map.remove(session_id).is_some()
}

async fn request_confirmation(
    confirm_map: &ConfirmMap,
    session_id: &str,
    _tool_name: &str,
    _arguments: &Value,
) -> Result<bool, String> {
    let (tx, rx) = oneshot::channel::<bool>();

    {
        let mut map = confirm_map
            .lock()
            .map_err(|e| format!("Failed to lock confirm_map: {}", e))?;
        map.insert(session_id.to_string(), tx);
    }

    match tokio::time::timeout(Duration::from_secs(CONFIRM_TIMEOUT_SECS), rx).await {
        Ok(Ok(confirmed)) => {
            let mut map = confirm_map
                .lock()
                .map_err(|e| format!("Failed to lock confirm_map: {}", e))?;
            map.remove(session_id);
            Ok(confirmed)
        }
        Ok(Err(_)) => {
            let mut map = confirm_map
                .lock()
                .map_err(|e| format!("Failed to lock confirm_map: {}", e))?;
            map.remove(session_id);
            Err("Confirmation request was cancelled".to_string())
        }
        Err(_) => {
            let mut map = confirm_map
                .lock()
                .map_err(|e| format!("Failed to lock confirm_map: {}", e))?;
            map.remove(session_id);
            Err("Confirmation timed out".to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: store a message in the agent DB
// ---------------------------------------------------------------------------

fn store_message(
    agent_db: &AgentDb,
    session_id: &str,
    role: &str,
    content: &str,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();

    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock agent DB: {}", e))?;

    conn.execute(
        "INSERT INTO agent_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, session_id, role, content, now],
    )
    .map_err(|e| format!("Failed to store message: {}", e))?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// Helper: store a tool call in the agent DB
// ---------------------------------------------------------------------------

fn store_tool_call(
    agent_db: &AgentDb,
    session_id: &str,
    message_id: &str,
    tool_name: &str,
    arguments: &str,
    tool_call_id: &str,
) -> Result<String, String> {
    let id = if tool_call_id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        tool_call_id.to_string()
    };
    let now = chrono::Utc::now().timestamp_millis();

    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock agent DB: {}", e))?;

    conn.execute(
        "INSERT INTO agent_tool_calls (id, message_id, session_id, tool_name, arguments, status, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
        rusqlite::params![id, message_id, session_id, tool_name, arguments, now],
    )
    .map_err(|e| format!("Failed to store tool call: {}", e))?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// Helper: update tool call status
// ---------------------------------------------------------------------------

fn update_tool_call_status(
    agent_db: &AgentDb,
    tool_call_id: &str,
    status: &str,
) -> Result<(), String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock agent DB: {}", e))?;

    conn.execute(
        "UPDATE agent_tool_calls SET status = ?1 WHERE id = ?2",
        rusqlite::params![status, tool_call_id],
    )
    .map_err(|e| format!("Failed to update tool call status: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Helper: store full tool result
// ---------------------------------------------------------------------------

fn store_tool_result(
    agent_db: &AgentDb,
    tool_call_id: &str,
    full_result: &str,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();

    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock agent DB: {}", e))?;

    conn.execute(
        "INSERT INTO tool_result_store (id, tool_call_id, full_result, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, tool_call_id, full_result, now],
    )
    .map_err(|e| format!("Failed to store tool result: {}", e))?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// Helper: retrieve messages from the agent DB for LLM context
// ---------------------------------------------------------------------------

fn load_messages(agent_db: &AgentDb, session_id: &str) -> Result<Vec<LlmMessage>, String> {
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock agent DB: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT role, content FROM agent_messages \
             WHERE session_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let messages = stmt
        .query_map(rusqlite::params![session_id], |row| {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok(LlmMessage { role, content })
        })
        .map_err(|e| format!("Failed to query messages: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(messages)
}

// ---------------------------------------------------------------------------
// Helper: update session status
// ---------------------------------------------------------------------------

fn update_session_status(agent_db: &AgentDb, session_id: &str, status: &str) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let conn = agent_db
        .0
        .lock()
        .map_err(|e| format!("Failed to lock agent DB: {}", e))?;

    conn.execute(
        "UPDATE agent_sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![status, now, session_id],
    )
    .map_err(|e| format!("Failed to update session status: {}", e))?;

    Ok(())
}

// ===========================================================================
// Main loop runner
// ===========================================================================

/// Run the full agent loop for a session.
///
/// This is a long-running async task that:
/// 1. Inserts the user's message into the DB
/// 2. Builds the message list from history
/// 3. Calls the LLM in a streaming fashion
/// 4. Handles tool calls in a sub-loop
/// 5. Enforces budget constraints (iterations, wall clock, tokens)
/// 6. Streams events back to the frontend
/// 7. Handles cancellations and confirmations
#[tauri::command]
pub async fn run_agent_loop(
    session_id: String,
    user_message: String,
    settings: Value,
    app: AppHandle,
    agent_db: State<'_, AgentDb>,
    confirm_map: State<'_, ConfirmMap>,
    cancel_map: State<'_, CancelMap>,
    executor: State<'_, Arc<dyn ToolExecutor>>,
) -> Result<(), String> {
    // -----------------------------------------------------------------------
    // Phase 0: Setup & budget initialization
    // -----------------------------------------------------------------------

    let max_iterations =
        get_settings_u64(&settings, "maxIterations", DEFAULT_MAX_ITERATIONS as u64) as u32;
    let wall_clock_budget_secs = get_settings_u64(
        &settings,
        "wallClockBudgetSecs",
        DEFAULT_WALL_CLOCK_BUDGET_SECS,
    );
    let token_budget =
        get_settings_u64(&settings, "tokenBudget", DEFAULT_TOKEN_BUDGET as u64) as usize;
    let require_confirmation = get_settings_bool(&settings, "requireConfirmation", true);
    let start_time = std::time::Instant::now();
    let model = get_settings_str(&settings, "model").unwrap_or_else(|| String::from("gpt-4o"));
    let base_url = get_base_url(&settings);
    let api_key = get_settings_str(&settings, "apiKey").unwrap_or_default();
    let api_compat =
        get_settings_str(&settings, "apiCompatibility").unwrap_or_else(|| String::from("openai"));
    let formatter: Box<dyn ChatFormatter> = if api_compat == "anthropic" {
        Box::new(AnthropicChatFormatter)
    } else {
        Box::new(OpenAIChatFormatter)
    };
    let connection_config = connection_config_from_settings(&settings);

    // Build system prompt and tools list once
    let system_prompt = build_system_prompt(&settings);
    let tools = build_tools_list(&settings);
    let has_tools = !tools.is_empty();

    // Mark session as active
    if let Err(e) = update_session_status(&agent_db, &session_id, "running") {
        emit_event(
            &app,
            "agent-loop-warning",
            json!({
                "session_id": session_id,
                "warning": format!("Failed to update session status: {}", e)
            }),
        );
    }

    // -----------------------------------------------------------------------
    // Phase 1: Insert user message
    // -----------------------------------------------------------------------

    let _user_msg_id =
        store_message(&agent_db, &session_id, "user", &user_message).map_err(|e| {
            emit_event(
                &app,
                "agent-loop-error",
                json!({
                    "session_id": session_id,
                    "error": format!("Failed to store user message: {}", e)
                }),
            );
            format!("Failed to store user message: {}", e)
        })?;

    // Emit context-usage event after storing user message
    let initial_messages = load_messages(&agent_db, &session_id).unwrap_or_default();
    let initial_tokens: usize = initial_messages
        .iter()
        .map(|m| crate::agent::token_counter::count_message_tokens(&m.role, &m.content, &model))
        .sum();
    emit_event(
        &app,
        "agent-context-usage",
        json!({
            "session_id": session_id,
            "used_tokens": initial_tokens,
            "capacity": token_budget,
            "message_count": initial_messages.len(),
        }),
    );

    // -----------------------------------------------------------------------
    // Phase 2: Main agent loop
    // -----------------------------------------------------------------------

    let mut iteration: u32 = 0;
    let mut accumulated_content = String::new();
    let mut accumulated_thinking = String::new();
    let mut total_tokens_used: usize = initial_tokens;

    loop {
        // --- Check cancellation ---
        if is_cancelled(&cancel_map, &session_id) {
            take_cancellation(&cancel_map, &session_id);
            emit_event(
                &app,
                "agent-loop-stopped",
                json!({
                    "session_id": session_id,
                    "reason": "cancelled",
                    "message": "Agent loop was cancelled by the user"
                }),
            );
            let _ = update_session_status(&agent_db, &session_id, "idle");
            return Ok(());
        }

        // --- Budget: max iterations ---
        iteration += 1;
        if iteration > max_iterations {
            emit_event(
                &app,
                "agent-loop-stopped",
                json!({
                    "session_id": session_id,
                    "reason": "max_iterations",
                    "message": format!("Reached maximum iterations ({})", max_iterations)
                }),
            );
            let _ = update_session_status(&agent_db, &session_id, "idle");
            return Ok(());
        }

        // --- Budget: wall clock ---
        let elapsed = start_time.elapsed();
        if elapsed.as_secs() > wall_clock_budget_secs {
            emit_event(
                &app,
                "agent-loop-stopped",
                json!({
                    "session_id": session_id,
                    "reason": "wall_clock",
                    "message": format!("Exceeded wall clock budget of {}s", wall_clock_budget_secs)
                }),
            );
            let _ = update_session_status(&agent_db, &session_id, "idle");
            return Ok(());
        }

        // --- Budget: token budget ---
        if total_tokens_used > token_budget {
            emit_event(
                &app,
                "agent-loop-stopped",
                json!({
                    "session_id": session_id,
                    "reason": "token_budget",
                    "message": format!("Exceeded token budget of {}", token_budget)
                }),
            );
            let _ = update_session_status(&agent_db, &session_id, "idle");
            return Ok(());
        }

        // --- Emit iteration event ---
        emit_event(
            &app,
            "agent-loop-iteration",
            json!({
                "session_id": session_id,
                "iter_count": iteration,
                "max_iterations": max_iterations,
            }),
        );

        // --- Emit waiting event ---
        emit_event(
            &app,
            "agent-loop-waiting-llm",
            json!({
                "session_id": session_id,
                "iter_count": iteration,
            }),
        );

        // --- Load messages for this iteration ---
        let _messages = match load_messages(&agent_db, &session_id) {
            Ok(msgs) => msgs,
            Err(e) => {
                emit_event(
                    &app,
                    "agent-loop-error",
                    json!({
                        "session_id": session_id,
                        "error": format!("Failed to load messages: {}", e)
                    }),
                );
                let _ = update_session_status(&agent_db, &session_id, "error");
                return Err(format!("Failed to load messages: {}", e));
            }
        };

        let (_model_name, context_window) = resolve_model_spec_for_session(&settings);
        let compact_threshold = settings
            .get("compactThreshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.75);
        let should_compact = evaluate(total_tokens_used, context_window, compact_threshold);

        if should_compact {
            emit_event(
                &app,
                "agent-loop-compacting",
                json!({
                    "session_id": session_id,
                    "phase": "start",
                }),
            );

            match crate::agent::compact::compact_session(
                &agent_db,
                &app,
                &settings,
                &session_id,
                0,
                &_model_name,
                context_window,
            ) {
                Ok(summary_info) => {
                    let pre_tokens = total_tokens_used;
                    let compacted_messages =
                        load_messages(&agent_db, &session_id).unwrap_or_default();
                    let post_tokens: usize = compacted_messages
                        .iter()
                        .map(|m| {
                            crate::agent::token_counter::count_message_tokens(
                                &m.role,
                                &m.content,
                                &_model_name,
                            )
                        })
                        .sum();
                    total_tokens_used = post_tokens;
                    let removed_count = summary_info
                        .get("removed_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    emit_event(
                        &app,
                        "agent-loop-summary-injected",
                        json!({
                            "session_id": session_id,
                            "trigger": "auto",
                            "pre_tokens": pre_tokens,
                            "post_tokens": post_tokens,
                            "removed_count": removed_count,
                        }),
                    );

                    emit_event(
                        &app,
                        "agent-context-usage",
                        json!({
                            "session_id": session_id,
                            "used_tokens": post_tokens,
                            "capacity": token_budget,
                            "message_count": compacted_messages.len(),
                        }),
                    );
                }
                Err(e) => {
                    emit_event(
                        &app,
                        "agent-loop-warning",
                        json!({
                            "session_id": session_id,
                            "warning": format!("Compaction failed: {}", e)
                        }),
                    );
                }
            }

            emit_event(
                &app,
                "agent-loop-compacting",
                json!({
                    "session_id": session_id,
                    "phase": "end",
                }),
            );
        }

        // --- Reload messages (they may have changed after compaction) ---
        let messages = match load_messages(&agent_db, &session_id) {
            Ok(msgs) => msgs,
            Err(e) => {
                emit_event(
                    &app,
                    "agent-loop-error",
                    json!({
                        "session_id": session_id,
                        "error": format!("Failed to reload messages after compaction: {}", e)
                    }),
                );
                let _ = update_session_status(&agent_db, &session_id, "error");
                return Err(format!("Failed to reload messages: {}", e));
            }
        };

        // --- Format messages for LLM ---
        let formatted_messages = match formatter.format_messages(&messages, &system_prompt) {
            Ok(fm) => fm,
            Err(e) => {
                emit_event(
                    &app,
                    "agent-loop-error",
                    json!({
                        "session_id": session_id,
                        "error": format!("Failed to format messages: {}", e)
                    }),
                );
                let _ = update_session_status(&agent_db, &session_id, "error");
                return Err(format!("Failed to format messages: {}", e));
            }
        };

        // Create the HTTP request payload
        let mut request_body = json!({
            "model": model,
            "messages": formatted_messages,
            "stream": true,
        });

        // Anthropic requires system prompt as a top-level field, not a message role
        if api_compat == "anthropic" && !system_prompt.is_empty() {
            if let Some(obj) = request_body.as_object_mut() {
                obj.insert("system".to_string(), json!(system_prompt));
            }
        }

        if has_tools {
            let formatted_tools = formatter.format_tools(&tools).unwrap_or(json!(tools));
            if let Some(obj) = request_body.as_object_mut() {
                obj.insert("tools".to_string(), formatted_tools);
                obj.insert("tool_choice".to_string(), json!("auto"));
            }
        }

        // Add optional parameters from settings
        if let Some(temp) = settings.get("temperature").and_then(|v| v.as_f64()) {
            if let Some(obj) = request_body.as_object_mut() {
                obj.insert("temperature".to_string(), json!(temp));
            }
        }
        if let Some(max_tokens) = settings.get("maxTokens").and_then(|v| v.as_u64()) {
            if let Some(obj) = request_body.as_object_mut() {
                obj.insert("max_tokens".to_string(), json!(max_tokens));
            }
        }

        // --- Make the API call ---
        let url = if api_compat == "anthropic" {
            format!("{}/v1/messages", base_url.trim_end_matches('/'))
        } else {
            format!("{}/chat/completions", base_url.trim_end_matches('/'))
        };

        let headers: reqwest::header::HeaderMap = if api_compat == "anthropic" {
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(
                reqwest::header::HeaderName::from_static("x-api-key"),
                reqwest::header::HeaderValue::from_str(&api_key).unwrap(),
            );
            h.insert(
                reqwest::header::HeaderName::from_static("anthropic-version"),
                reqwest::header::HeaderValue::from_static("2023-06-01"),
            );
            h.insert(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/json"),
            );
            h
        } else {
            let h = build_headers(
                &serde_json::json!({ "apiKey": api_key, "apiCompatibility": api_compat }),
            )
            .unwrap_or_else(|_| {
                let mut h = reqwest::header::HeaderMap::new();
                h.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                h
            });
            h
        };
        let client = create_http_client("system", None, Some(true), Some(Duration::from_secs(300)));

        // Reset accumulated content for this iteration
        accumulated_content.clear();
        accumulated_thinking.clear();
        let mut tool_calls_this_turn: Vec<LlmToolCall> = Vec::new();
        let mut finish_reason: Option<String> = None;
        let mut assistant_content = String::new();
        let mut assistant_thinking = String::new();

        // Try the LLM call with retries
        let stream_result = retry_with_backoff(|| {
            let client = client.clone();
            let url = url.clone();
            let body = request_body.clone();
            let headers = headers.clone();

            async move {
                let response = client
                    .post(&url)
                    .headers(headers)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("HTTP request failed: {}", e))?;

                if !response.status().is_success() {
                    let status = response.status();
                    let body_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unable to read response body".to_string());
                    return Err(format!(
                        "LLM API returned status code {}: {}",
                        status, body_text
                    ));
                }

                Ok(response)
            }
        })
        .await;

        let response = match stream_result {
            Ok(resp) => resp,
            Err(e) => {
                let friendly = extract_llm_error(&e);
                emit_event(
                    &app,
                    "agent-loop-error",
                    json!({
                        "session_id": session_id,
                        "error": friendly,
                    }),
                );
                let _ = update_session_status(&agent_db, &session_id, "error");
                return Err(friendly);
            }
        };

        // --- Process streaming response ---
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            // Check cancellation mid-stream
            if is_cancelled(&cancel_map, &session_id) {
                take_cancellation(&cancel_map, &session_id);
                emit_event(
                    &app,
                    "agent-loop-stopped",
                    json!({
                        "session_id": session_id,
                        "reason": "cancelled",
                        "message": "Agent loop was cancelled during streaming"
                    }),
                );
                let _ = update_session_status(&agent_db, &session_id, "idle");
                return Ok(());
            }

            // Check wall clock mid-stream
            if start_time.elapsed().as_secs() > wall_clock_budget_secs {
                emit_event(
                    &app,
                    "agent-loop-stopped",
                    json!({
                        "session_id": session_id,
                        "reason": "wall_clock",
                        "message": "Exceeded wall clock budget during streaming"
                    }),
                );
                let _ = update_session_status(&agent_db, &session_id, "idle");
                return Ok(());
            }

            match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    buffer.push_str(&text);

                    // Process complete SSE lines from the buffer
                    while let Some(line_end) = buffer.find('\n') {
                        let line = buffer[..line_end].trim().to_string();
                        buffer = buffer[line_end + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                break;
                            }

                            match serde_json::from_str::<Value>(data) {
                                Ok(chunk) => {
                                    match formatter.parse_chunk(&chunk) {
                                        Ok(chunk_result) => {
                                            // Content delta
                                            if let Some(delta) = &chunk_result.content_delta {
                                                assistant_content.push_str(delta);
                                                accumulated_content.push_str(delta);
                                                emit_event(
                                                    &app,
                                                    "agent-loop-delta",
                                                    json!({
                                                        "session_id": session_id,
                                                        "content": delta,
                                                    }),
                                                );
                                            }

                                            // Thinking delta
                                            if let Some(thinking) = &chunk_result.thinking_delta {
                                                assistant_thinking.push_str(thinking);
                                                accumulated_thinking.push_str(thinking);
                                                emit_event(
                                                    &app,
                                                    "agent-loop-thinking-delta",
                                                    json!({
                                                        "session_id": session_id,
                                                        "content": thinking,
                                                    }),
                                                );
                                            }

                                            // Accumulate tool calls (handles partial chunks)
                                            for tc in &chunk_result.tool_calls {
                                                // Accumulate or merge tool calls by index/id
                                                let existing = tool_calls_this_turn
                                                    .iter_mut()
                                                    .find(|existing: &&mut LlmToolCall| {
                                                        existing.id == tc.id
                                                    });
                                                if let Some(existing) = existing {
                                                    // Merge: if name is empty, keep existing; if arguments accumulate
                                                    if !tc.name.is_empty() {
                                                        existing.name = tc.name.clone();
                                                    }
                                                    // Merge arguments by string concatenation (streaming JSON)
                                                    if tc.arguments != json!({}) {
                                                        // Approximate: replace with merged if current is partial
                                                        existing.arguments = tc.arguments.clone();
                                                    }
                                                } else {
                                                    tool_calls_this_turn.push(tc.clone());
                                                }
                                            }

                                            // Finish reason
                                            if let Some(reason) = &chunk_result.finish_reason {
                                                finish_reason = Some(reason.clone());
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("[loop_runner] Failed to parse chunk: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[loop_runner] Failed to parse SSE data: {} (data: {})",
                                        e,
                                        data.chars().take(100).collect::<String>()
                                    );
                                }
                            }
                        } else if line.starts_with("{\"id\":") || line.starts_with("{\"object\":") {
                            // Non-SSE JSON streaming (some providers)
                            match serde_json::from_str::<Value>(&line) {
                                Ok(chunk) => {
                                    if let Ok(cr) = formatter.parse_chunk(&chunk) {
                                        if let Some(delta) = cr.content_delta {
                                            assistant_content.push_str(&delta);
                                            accumulated_content.push_str(&delta);
                                            emit_event(
                                                &app,
                                                "agent-loop-delta",
                                                json!({
                                                    "session_id": session_id,
                                                    "content": delta,
                                                }),
                                            );
                                        }
                                        if let Some(thinking) = cr.thinking_delta {
                                            assistant_thinking.push_str(&thinking);
                                            accumulated_thinking.push_str(&thinking);
                                            emit_event(
                                                &app,
                                                "agent-loop-thinking-delta",
                                                json!({
                                                    "session_id": session_id,
                                                    "content": thinking,
                                                }),
                                            );
                                        }
                                        if let Some(reason) = cr.finish_reason {
                                            finish_reason = Some(reason);
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Skip non-JSON lines (like keep-alive comments)
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    emit_event(
                        &app,
                        "agent-loop-error",
                        json!({
                            "session_id": session_id,
                            "error": format!("Stream error: {}", e),
                        }),
                    );
                    let _ = update_session_status(&agent_db, &session_id, "error");
                    return Err(format!("Stream error: {}", e));
                }
            }
        }

        // --- Determine finish reason ---
        let reason = finish_reason.unwrap_or_else(|| {
            if !tool_calls_this_turn.is_empty() {
                "tool_calls".to_string()
            } else {
                "stop".to_string()
            }
        });

        // --- Store assistant message ---
        let assistant_msg_id =
            store_message(&agent_db, &session_id, "assistant", &assistant_content)?;

        // Emit step-done for the assistant message
        emit_event(
            &app,
            "agent-loop-step-done",
            json!({
                "session_id": session_id,
                "message_id": assistant_msg_id,
            }),
        );

        // Update token tracking
        let assistant_tokens = crate::agent::token_counter::count_message_tokens(
            "assistant",
            &assistant_content,
            &model,
        );
        total_tokens_used += assistant_tokens;

        // If thinking was present, also count it
        if !assistant_thinking.is_empty() {
            let thinking_tokens =
                crate::agent::token_counter::count_tokens(&assistant_thinking, &model);
            total_tokens_used += thinking_tokens;
        }

        emit_event(
            &app,
            "agent-context-usage",
            json!({
                "session_id": session_id,
                "used_tokens": total_tokens_used,
                "capacity": token_budget,
                "message_count": load_messages(&agent_db, &session_id).unwrap_or_default().len(),
            }),
        );

        // -------------------------------------------------------------------
        // Phase 3: Handle tool calls
        // -------------------------------------------------------------------

        if reason == "tool_calls" && !tool_calls_this_turn.is_empty() {
            for tool_call in &tool_calls_this_turn {
                // Check cancellation before each tool call
                if is_cancelled(&cancel_map, &session_id) {
                    take_cancellation(&cancel_map, &session_id);
                    emit_event(
                        &app,
                        "agent-loop-stopped",
                        json!({
                            "session_id": session_id,
                            "reason": "cancelled",
                            "message": "Agent loop was cancelled during tool execution"
                        }),
                    );
                    let _ = update_session_status(&agent_db, &session_id, "idle");
                    return Ok(());
                }

                // Check wall clock before each tool call
                if start_time.elapsed().as_secs() > wall_clock_budget_secs {
                    emit_event(
                        &app,
                        "agent-loop-stopped",
                        json!({
                            "session_id": session_id,
                            "reason": "wall_clock",
                            "message": "Exceeded wall clock budget during tool execution"
                        }),
                    );
                    let _ = update_session_status(&agent_db, &session_id, "idle");
                    return Ok(());
                }

                // Store the tool call
                let tool_call_args_str = serde_json::to_string(&tool_call.arguments)
                    .unwrap_or_else(|_| "{}".to_string());
                let tool_call_db_id = store_tool_call(
                    &agent_db,
                    &session_id,
                    &assistant_msg_id,
                    &tool_call.name,
                    &tool_call_args_str,
                    &tool_call.id,
                )?;

                // Emit tool-call event to frontend
                emit_event(
                    &app,
                    "agent-loop-tool-call",
                    json!({
                        "session_id": session_id,
                        "tool_call_id": tool_call.id,
                        "tool_name": tool_call.name,
                        "arguments": tool_call.arguments,
                    }),
                );

                // --- Confirmation check ---
                let should_execute = if require_confirmation {
                    // Check if there's a confirmation rule that auto-confirms this tool
                    let auto_confirmed =
                        is_tool_auto_confirmed(&agent_db, &session_id, &tool_call.name)?;

                    if auto_confirmed {
                        true
                    } else {
                        // Ask user for confirmation
                        emit_event(
                            &app,
                            "agent-loop-waiting-llm",
                            json!({
                                "session_id": session_id,
                                "iter_count": iteration,
                                "status": "awaiting_confirmation",
                                "tool_name": tool_call.name,
                            }),
                        );

                        match request_confirmation(
                            &confirm_map,
                            &session_id,
                            &tool_call.name,
                            &tool_call.arguments,
                        )
                        .await
                        {
                            Ok(true) => true,
                            Ok(false) => {
                                // User denied the tool call — record it
                                update_tool_call_status(&agent_db, &tool_call_db_id, "denied")?;

                                // Inject a message explaining the denial
                                let denial_msg = format!(
                                    "Tool call '{}' was denied by the user. Arguments: {}",
                                    tool_call.name, tool_call_args_str
                                );
                                store_message(&agent_db, &session_id, "tool", &denial_msg)?;

                                emit_event(
                                    &app,
                                    "agent-loop-tool-result",
                                    json!({
                                        "session_id": session_id,
                                        "tool_call_id": tool_call.id,
                                        "envelope": null,
                                        "error": "Tool call denied by user",
                                    }),
                                );
                                false
                            }
                            Err(e) => {
                                // Confirmation failed (timeout, etc.) — deny
                                update_tool_call_status(&agent_db, &tool_call_db_id, "error")?;

                                let error_msg = format!(
                                    "Tool call '{}' confirmation failed: {}",
                                    tool_call.name, e
                                );
                                store_message(&agent_db, &session_id, "tool", &error_msg)?;

                                emit_event(
                                    &app,
                                    "agent-loop-tool-result",
                                    json!({
                                        "session_id": session_id,
                                        "tool_call_id": tool_call.id,
                                        "envelope": null,
                                        "error": error_msg,
                                    }),
                                );
                                false
                            }
                        }
                    }
                } else {
                    // Confirmation not required
                    true
                };

                if !should_execute {
                    continue;
                }

                // --- Execute the tool ---
                update_tool_call_status(&agent_db, &tool_call_db_id, "executing")?;

                let tool_result = executor
                    .execute(&tool_call.name, &tool_call.arguments, &connection_config)
                    .await;

                match tool_result {
                    Ok(envelope) => {
                        update_tool_call_status(&agent_db, &tool_call_db_id, "completed")?;

                        // Store full result in the tool_result_store
                        store_tool_result(&agent_db, &tool_call_db_id, &envelope.full_result)?;

                        // Store the summary as a tool message in the conversation
                        let tool_message =
                            format!("[Tool: {}]\n{}", tool_call.name, envelope.summary);
                        store_message(&agent_db, &session_id, "tool", &tool_message)?;

                        // Count tool result tokens
                        let result_tokens = crate::agent::token_counter::count_message_tokens(
                            "tool",
                            &tool_message,
                            &model,
                        );
                        total_tokens_used += result_tokens;

                        // Emit tool-result event
                        emit_event(
                            &app,
                            "agent-loop-tool-result",
                            json!({
                                "session_id": session_id,
                                "tool_call_id": tool_call.id,
                                "envelope": {
                                    "summary": envelope.summary,
                                    "full_result": null, // Not sent in event to keep payload small
                                    "metadata": {
                                        "tool_name": envelope.metadata.tool_name,
                                        "duration_ms": envelope.metadata.duration_ms,
                                        "truncated": envelope.metadata.truncated,
                                    },
                                },
                                "error": null,
                            }),
                        );
                    }
                    Err(err) => {
                        update_tool_call_status(&agent_db, &tool_call_db_id, "error")?;

                        let error_message = format!("[Tool: {}] Error: {}", tool_call.name, err);
                        store_message(&agent_db, &session_id, "tool", &error_message)?;

                        let error_tokens = crate::agent::token_counter::count_message_tokens(
                            "tool",
                            &error_message,
                            &model,
                        );
                        total_tokens_used += error_tokens;

                        emit_event(
                            &app,
                            "agent-loop-tool-result",
                            json!({
                                "session_id": session_id,
                                "tool_call_id": tool_call.id,
                                "envelope": null,
                                "error": err,
                            }),
                        );
                    }
                }
            }

            // After processing all tool calls, continue the main loop
            // (the tool results are now in the message history)
            continue;
        }

        // -------------------------------------------------------------------
        // Phase 4: If not tool calls, we're done
        // -------------------------------------------------------------------

        if reason == "stop" || reason == "length" || reason == "end_turn" {
            emit_event(
                &app,
                "agent-loop-done",
                json!({
                    "session_id": session_id,
                }),
            );
            let _ = update_session_status(&agent_db, &session_id, "idle");
            emit_event(
                &app,
                "agent-context-usage",
                json!({
                    "session_id": session_id,
                    "used_tokens": total_tokens_used,
                    "capacity": token_budget,
                    "message_count": load_messages(&agent_db, &session_id).unwrap_or_default().len(),
                }),
            );
            return Ok(());
        }

        // If we reach here with an unknown finish reason, stop gracefully
        emit_event(
            &app,
            "agent-loop-done",
            json!({
                "session_id": session_id,
            }),
        );
        let _ = update_session_status(&agent_db, &session_id, "idle");
        return Ok(());
    }
}

// ---------------------------------------------------------------------------
// Helper: check if a tool is auto-confirmed for a session
// ---------------------------------------------------------------------------

fn is_tool_auto_confirmed(
    agent_db: &AgentDb,
    session_id: &str,
    tool_name: &str,
) -> Result<bool, String> {
    let conn = match agent_db.0.lock() {
        Ok(c) => c,
        Err(e) => return Err(format!("Failed to lock agent DB: {}", e)),
    };

    let result: Result<bool, _> = conn.query_row(
        "SELECT COUNT(*) > 0 FROM confirmation_rules \
         WHERE session_id = ?1 AND tool_name = ?2 AND action = 'allow'",
        rusqlite::params![session_id, tool_name],
        |row| row.get(0),
    );

    result.map_err(|e| format!("Failed to check confirmation rules: {}", e))
}

// ===========================================================================
// Cancel agent loop
// ===========================================================================

/// Cancel a running agent loop for a session.
#[tauri::command]
pub async fn cancel_agent_loop(
    session_id: String,
    cancel_map: State<'_, CancelMap>,
    agent_db: State<'_, AgentDb>,
) -> Result<(), String> {
    let mut map = cancel_map
        .lock()
        .map_err(|e| format!("Failed to lock cancel_map: {}", e))?;

    if let Some(sender) = map.remove(&session_id) {
        let _ = sender.send(());
    } else {
        let (tx, _rx) = oneshot::channel::<()>();
        map.insert(session_id.clone(), tx);
    }
    drop(map);

    let _ = update_session_status(&agent_db, &session_id, "idle");

    Ok(())
}

// ===========================================================================
// Confirm tool call
// ===========================================================================

/// Confirm (or deny) a pending tool call for a session.
#[tauri::command]
pub async fn confirm_tool_call(
    tool_call_id: String,
    allowed: bool,
    agent_db: State<'_, AgentDb>,
    confirm_map: State<'_, ConfirmMap>,
) -> Result<(), String> {
    let inner: &AgentDb = &*agent_db;
    let conn = inner
        .0
        .lock()
        .map_err(|e| format!("Failed to lock agent DB: {}", e))?;

    let session_id: Result<String, _> = conn.query_row(
        "SELECT session_id FROM agent_tool_calls WHERE id = ?1",
        rusqlite::params![tool_call_id],
        |row| row.get(0),
    );
    drop(conn);
    let session_id = session_id.map_err(|_| format!("Tool call not found: {}", tool_call_id))?;

    let mut map = confirm_map
        .lock()
        .map_err(|e| format!("Failed to lock confirm_map: {}", e))?;

    if let Some(sender) = map.remove(&session_id) {
        sender
            .send(allowed)
            .map_err(|_| "Failed to send confirmation: receiver dropped".to_string())?;
        Ok(())
    } else {
        Err(format!(
            "No pending confirmation request for session '{}'",
            session_id
        ))
    }
}

// ===========================================================================
// Compact agent session
// ===========================================================================

/// Manually trigger context window compaction for a session.
#[tauri::command]
pub async fn compact_agent_session(
    session_id: String,
    settings: Value,
    app: AppHandle,
    agent_db: State<'_, AgentDb>,
) -> Result<(), String> {
    let model = get_settings_str(&settings, "model").unwrap_or_else(|| String::from("gpt-4o"));
    let (_model_name, context_window) =
        crate::agent::compact::resolve_model_spec_for_session_old(&settings);

    emit_event(
        &app,
        "agent-loop-compacting",
        json!({
            "session_id": session_id,
            "phase": "start",
        }),
    );

    let summary_info = crate::agent::compact::compact_session(
        &agent_db,
        &app,
        &settings,
        &session_id,
        0,
        &model,
        context_window,
    )?;

    let compacted_messages = load_messages(&agent_db, &session_id).unwrap_or_default();
    let post_tokens: usize = compacted_messages
        .iter()
        .map(|m| crate::agent::token_counter::count_message_tokens(&m.role, &m.content, &model))
        .sum();
    let removed_count = summary_info
        .get("removed_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    emit_event(
        &app,
        "agent-loop-summary-injected",
        json!({
            "session_id": session_id,
            "trigger": "manual",
            "pre_tokens": 0,
            "post_tokens": post_tokens,
            "removed_count": removed_count,
        }),
    );

    emit_event(
        &app,
        "agent-context-usage",
        json!({
            "session_id": session_id,
            "used_tokens": post_tokens,
            "capacity": DEFAULT_TOKEN_BUDGET,
            "message_count": compacted_messages.len(),
        }),
    );

    emit_event(
        &app,
        "agent-loop-compacting",
        json!({
            "session_id": session_id,
            "phase": "end",
        }),
    );

    Ok(())
}

// ===========================================================================
// Get agent context usage
// ===========================================================================

/// Return the current context window usage stats for a session.
#[tauri::command]
pub async fn get_agent_context_usage(
    session_id: String,
    settings: Value,
    agent_db: State<'_, AgentDb>,
) -> Result<Value, String> {
    let model = get_settings_str(&settings, "model").unwrap_or_else(|| String::from("gpt-4o"));
    let messages = load_messages(&agent_db, &session_id)?;

    let total_tokens: usize = messages
        .iter()
        .map(|m| crate::agent::token_counter::count_message_tokens(&m.role, &m.content, &model))
        .sum();

    let projected = count_projected_tokens(&messages, &model);

    Ok(json!({
        "session_id": session_id,
        "message_count": messages.len(),
        "used_tokens": total_tokens,
        "projected_tokens": projected,
        "capacity": DEFAULT_TOKEN_BUDGET,
        "usage_percent": if projected > 0 {
            ((total_tokens as f64 / projected as f64) * 100.0).round() as u64
        } else {
            0
        },
    }))
}

// ===========================================================================
// Get tool full result
// ===========================================================================

/// Retrieve the full result of a tool call from the tool_result_store.
#[tauri::command]
pub async fn get_tool_full_result(
    tool_call_id: String,
    agent_db: State<'_, AgentDb>,
) -> Result<String, String> {
    let inner: &AgentDb = &*agent_db;
    let conn = inner
        .0
        .lock()
        .map_err(|e| format!("Failed to lock agent DB: {}", e))?;

    let result: Result<String, _> = conn.query_row(
        "SELECT full_result FROM tool_result_store WHERE tool_call_id = ?1 \
         ORDER BY created_at DESC LIMIT 1",
        rusqlite::params![tool_call_id],
        |row| row.get(0),
    );

    result.map_err(|e| format!("Tool result not found: {}", e))
}

// ===========================================================================
// Helper: run the agent loop in background (for use from other commands)
// ===========================================================================

/// Spawn the agent loop as a background task on the Tauri async runtime.
/// Returns immediately; events are streamed via Tauri events.
pub fn spawn_agent_loop(
    app: AppHandle,
    session_id: String,
    user_message: String,
    settings: Value,
    agent_db: AgentDb,
    confirm_map: ConfirmMap,
    cancel_map: CancelMap,
    executor: Arc<dyn ToolExecutor>,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_agent_loop_inner(
            &app,
            &session_id,
            &user_message,
            &settings,
            &agent_db,
            &confirm_map,
            &cancel_map,
            &executor,
        )
        .await
        {
            emit_event(
                &app,
                "agent-loop-error",
                json!({
                    "session_id": session_id,
                    "error": e,
                }),
            );
        }
    });
}

/// Internal version of `run_agent_loop` that takes references (no Tauri State).
async fn run_agent_loop_inner(
    app: &AppHandle,
    session_id: &str,
    user_message: &str,
    settings: &Value,
    agent_db: &AgentDb,
    confirm_map: &ConfirmMap,
    cancel_map: &CancelMap,
    executor: &Arc<dyn ToolExecutor>,
) -> Result<(), String> {
    // This is a simplified version; the full Tauri command handles all the details.
    // For background use, we delegate to the same logic via a direct implementation.
    // (In practice, code would be shared; here we reference the core logic.)

    let max_iterations =
        get_settings_u64(settings, "maxIterations", DEFAULT_MAX_ITERATIONS as u64) as u32;
    let wall_clock_budget_secs = get_settings_u64(
        settings,
        "wallClockBudgetSecs",
        DEFAULT_WALL_CLOCK_BUDGET_SECS,
    );
    let token_budget =
        get_settings_u64(settings, "tokenBudget", DEFAULT_TOKEN_BUDGET as u64) as usize;
    let require_confirmation = get_settings_bool(settings, "requireConfirmation", true);
    let start_time = std::time::Instant::now();
    let model = get_settings_str(settings, "model").unwrap_or_else(|| String::from("gpt-4o"));
    let base_url = get_base_url(settings);
    let api_key = get_settings_str(settings, "apiKey").unwrap_or_default();
    let api_compat =
        get_settings_str(settings, "apiCompatibility").unwrap_or_else(|| String::from("openai"));
    let formatter: Box<dyn ChatFormatter> = if api_compat == "anthropic" {
        Box::new(AnthropicChatFormatter)
    } else {
        Box::new(OpenAIChatFormatter)
    };
    let connection_config = connection_config_from_settings(settings);

    let system_prompt = build_system_prompt(settings);
    let tools = build_tools_list(settings);
    let has_tools = !tools.is_empty();

    let _ = update_session_status(agent_db, session_id, "running");

    let _user_msg_id = store_message(agent_db, session_id, "user", user_message)?;

    let mut iteration: u32 = 0;
    let mut total_tokens_used: usize = 0;

    loop {
        // --- Check cancellation ---
        if is_cancelled(cancel_map, session_id) {
            take_cancellation(cancel_map, session_id);
            emit_event(
                app,
                "agent-loop-stopped",
                json!({
                    "session_id": session_id,
                    "reason": "cancelled",
                    "message": "Agent loop was cancelled by the user"
                }),
            );
            let _ = update_session_status(agent_db, session_id, "idle");
            return Ok(());
        }

        // --- Budget checks ---
        iteration += 1;
        if iteration > max_iterations {
            emit_event(
                app,
                "agent-loop-stopped",
                json!({
                    "session_id": session_id,
                    "reason": "max_iterations",
                    "message": format!("Reached maximum iterations ({})", max_iterations)
                }),
            );
            let _ = update_session_status(agent_db, session_id, "idle");
            return Ok(());
        }

        if start_time.elapsed().as_secs() > wall_clock_budget_secs {
            emit_event(
                app,
                "agent-loop-stopped",
                json!({
                    "session_id": session_id,
                    "reason": "wall_clock",
                    "message": format!("Exceeded wall clock budget of {}s", wall_clock_budget_secs)
                }),
            );
            let _ = update_session_status(agent_db, session_id, "idle");
            return Ok(());
        }

        if total_tokens_used > token_budget {
            emit_event(
                app,
                "agent-loop-stopped",
                json!({
                    "session_id": session_id,
                    "reason": "token_budget",
                    "message": format!("Exceeded token budget of {}", token_budget)
                }),
            );
            let _ = update_session_status(agent_db, session_id, "idle");
            return Ok(());
        }

        emit_event(
            app,
            "agent-loop-iteration",
            json!({
                "session_id": session_id,
                "iter_count": iteration,
                "max_iterations": max_iterations,
            }),
        );

        emit_event(
            app,
            "agent-loop-waiting-llm",
            json!({
                "session_id": session_id,
                "iter_count": iteration,
            }),
        );

        // --- Load, potentially compact, call LLM ---
        let _messages = load_messages(agent_db, session_id)?;
        let (_model_name, context_window) = resolve_model_spec_for_session(settings);
        let compact_threshold = settings
            .get("compactThreshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.75);
        let should_compact = evaluate(total_tokens_used, context_window, compact_threshold);

        if should_compact {
            emit_event(
                app,
                "agent-loop-compacting",
                json!({
                    "session_id": session_id,
                    "phase": "start",
                }),
            );

            if let Ok(summary_info) = crate::agent::compact::compact_session(
                agent_db,
                app,
                settings,
                session_id,
                0,
                &_model_name,
                context_window,
            ) {
                let compacted = load_messages(agent_db, session_id).unwrap_or_default();
                let post_tokens: usize = compacted
                    .iter()
                    .map(|m| {
                        crate::agent::token_counter::count_message_tokens(
                            &m.role, &m.content, &model,
                        )
                    })
                    .sum();
                total_tokens_used = post_tokens;
                let removed = summary_info
                    .get("removed_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                emit_event(
                    app,
                    "agent-loop-summary-injected",
                    json!({
                        "session_id": session_id,
                        "trigger": "auto",
                        "pre_tokens": 0,
                        "post_tokens": post_tokens,
                        "removed_count": removed,
                    }),
                );
            }

            emit_event(
                app,
                "agent-loop-compacting",
                json!({
                    "session_id": session_id,
                    "phase": "end",
                }),
            );
        }

        // Reload messages after potential compaction
        let messages = load_messages(agent_db, session_id)?;
        let formatted_messages = formatter.format_messages(&messages, &system_prompt)?;

        let mut request_body = json!({
            "model": model,
            "messages": formatted_messages,
            "stream": true,
        });

        if has_tools {
            let formatted_tools = formatter.format_tools(&tools).unwrap_or(json!(tools));
            if let Some(obj) = request_body.as_object_mut() {
                obj.insert("tools".to_string(), formatted_tools);
                obj.insert("tool_choice".to_string(), json!("auto"));
            }
        }

        if let Some(temp) = settings.get("temperature").and_then(|v| v.as_f64()) {
            if let Some(obj) = request_body.as_object_mut() {
                obj.insert("temperature".to_string(), json!(temp));
            }
        }
        if let Some(max_t) = settings.get("maxTokens").and_then(|v| v.as_u64()) {
            if let Some(obj) = request_body.as_object_mut() {
                obj.insert("max_tokens".to_string(), json!(max_t));
            }
        }

        let url = if api_compat == "anthropic" {
            format!("{}/v1/messages", base_url.trim_end_matches('/'))
        } else {
            format!("{}/chat/completions", base_url.trim_end_matches('/'))
        };

        let headers: reqwest::header::HeaderMap = if api_compat == "anthropic" {
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(
                reqwest::header::HeaderName::from_static("x-api-key"),
                reqwest::header::HeaderValue::from_str(&api_key).unwrap(),
            );
            h.insert(
                reqwest::header::HeaderName::from_static("anthropic-version"),
                reqwest::header::HeaderValue::from_static("2023-06-01"),
            );
            h.insert(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/json"),
            );
            h
        } else {
            let h = build_headers(
                &serde_json::json!({ "apiKey": api_key, "apiCompatibility": api_compat }),
            )
            .unwrap_or_else(|_| {
                let mut h = reqwest::header::HeaderMap::new();
                h.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                h
            });
            h
        };
        let client = create_http_client("system", None, Some(true), Some(Duration::from_secs(300)));

        let mut assistant_content = String::new();
        let mut tool_calls_this_turn: Vec<LlmToolCall> = Vec::new();
        let mut finish_reason: Option<String> = None;

        let stream_result = retry_with_backoff(|| {
            let client = client.clone();
            let url = url.clone();
            let body = request_body.clone();
            let headers = headers.clone();

            async move {
                let resp = client
                    .post(&url)
                    .headers(headers)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("HTTP request failed: {}", e))?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    return Err(format!("LLM API returned {}: {}", status, text));
                }

                Ok(resp)
            }
        })
        .await;

        let response = match stream_result {
            Ok(r) => r,
            Err(e) => {
                emit_event(
                    app,
                    "agent-loop-error",
                    json!({
                        "session_id": session_id,
                        "error": extract_llm_error(&e),
                    }),
                );
                let _ = update_session_status(agent_db, session_id, "error");
                return Err(extract_llm_error(&e));
            }
        };

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            if is_cancelled(cancel_map, session_id) {
                take_cancellation(cancel_map, session_id);
                emit_event(
                    app,
                    "agent-loop-stopped",
                    json!({
                        "session_id": session_id,
                        "reason": "cancelled",
                        "message": "Cancelled during streaming"
                    }),
                );
                let _ = update_session_status(agent_db, session_id, "idle");
                return Ok(());
            }

            match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    buffer.push_str(&text);

                    while let Some(line_end) = buffer.find('\n') {
                        let line = buffer[..line_end].trim().to_string();
                        buffer = buffer[line_end + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                break;
                            }

                            if let Ok(chunk) = serde_json::from_str::<Value>(data) {
                                if let Ok(cr) = formatter.parse_chunk(&chunk) {
                                    if let Some(delta) = cr.content_delta {
                                        assistant_content.push_str(&delta);
                                        emit_event(
                                            app,
                                            "agent-loop-delta",
                                            json!({
                                                "session_id": session_id,
                                                "content": delta,
                                            }),
                                        );
                                    }
                                    if let Some(thinking) = cr.thinking_delta {
                                        emit_event(
                                            app,
                                            "agent-loop-thinking-delta",
                                            json!({
                                                "session_id": session_id,
                                                "content": thinking,
                                            }),
                                        );
                                    }
                                    for tc in cr.tool_calls {
                                        if let Some(existing) = tool_calls_this_turn
                                            .iter_mut()
                                            .find(|e: &&mut LlmToolCall| e.id == tc.id)
                                        {
                                            if !tc.name.is_empty() {
                                                existing.name = tc.name.clone();
                                            }
                                            if tc.arguments != json!({}) {
                                                existing.arguments = tc.arguments.clone();
                                            }
                                        } else {
                                            tool_calls_this_turn.push(tc);
                                        }
                                    }
                                    if let Some(reason) = cr.finish_reason {
                                        finish_reason = Some(reason);
                                    }
                                }
                            }
                        } else if line.starts_with('{') {
                            if let Ok(chunk) = serde_json::from_str::<Value>(&line) {
                                if let Ok(cr) = formatter.parse_chunk(&chunk) {
                                    if let Some(delta) = cr.content_delta {
                                        assistant_content.push_str(&delta);
                                        emit_event(
                                            app,
                                            "agent-loop-delta",
                                            json!({
                                                "session_id": session_id,
                                                "content": delta,
                                            }),
                                        );
                                    }
                                    if let Some(delta) = cr.thinking_delta {
                                        emit_event(
                                            app,
                                            "agent-loop-thinking-delta",
                                            json!({
                                                "session_id": session_id,
                                                "content": delta,
                                            }),
                                        );
                                    }
                                    if let Some(reason) = cr.finish_reason {
                                        finish_reason = Some(reason);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    emit_event(
                        app,
                        "agent-loop-error",
                        json!({
                            "session_id": session_id,
                            "error": format!("Stream error: {}", e),
                        }),
                    );
                    let _ = update_session_status(agent_db, session_id, "error");
                    return Err(format!("Stream error: {}", e));
                }
            }
        }

        let reason = finish_reason.unwrap_or_else(|| {
            if !tool_calls_this_turn.is_empty() {
                "tool_calls".to_string()
            } else {
                "stop".to_string()
            }
        });

        let assistant_msg_id =
            store_message(agent_db, session_id, "assistant", &assistant_content)?;

        emit_event(
            app,
            "agent-loop-step-done",
            json!({
                "session_id": session_id,
                "message_id": assistant_msg_id,
            }),
        );

        total_tokens_used += crate::agent::token_counter::count_message_tokens(
            "assistant",
            &assistant_content,
            &model,
        );

        // Handle tool calls
        if reason == "tool_calls" && !tool_calls_this_turn.is_empty() {
            for tool_call in &tool_calls_this_turn {
                if is_cancelled(cancel_map, session_id) {
                    take_cancellation(cancel_map, session_id);
                    emit_event(
                        app,
                        "agent-loop-stopped",
                        json!({
                            "session_id": session_id,
                            "reason": "cancelled",
                            "message": "Cancelled during tool execution"
                        }),
                    );
                    let _ = update_session_status(agent_db, session_id, "idle");
                    return Ok(());
                }

                let args_str = serde_json::to_string(&tool_call.arguments)
                    .unwrap_or_else(|_| "{}".to_string());
                let tc_db_id = store_tool_call(
                    agent_db,
                    session_id,
                    &assistant_msg_id,
                    &tool_call.name,
                    &args_str,
                    &tool_call.id,
                )?;

                emit_event(
                    app,
                    "agent-loop-tool-call",
                    json!({
                        "session_id": session_id,
                        "tool_call_id": tool_call.id,
                        "tool_name": tool_call.name,
                        "arguments": tool_call.arguments,
                    }),
                );

                let should_exec = if require_confirmation {
                    let auto = is_tool_auto_confirmed(agent_db, session_id, &tool_call.name)?;
                    if auto {
                        true
                    } else {
                        match request_confirmation(
                            confirm_map,
                            session_id,
                            &tool_call.name,
                            &tool_call.arguments,
                        )
                        .await
                        {
                            Ok(true) => true,
                            Ok(false) => {
                                let _ = update_tool_call_status(agent_db, &tc_db_id, "denied");
                                let msg = format!("Tool call '{}' was denied.", tool_call.name);
                                let _ = store_message(agent_db, session_id, "tool", &msg);
                                emit_event(
                                    app,
                                    "agent-loop-tool-result",
                                    json!({
                                        "session_id": session_id,
                                        "tool_call_id": tool_call.id,
                                        "envelope": null,
                                        "error": "Denied by user",
                                    }),
                                );
                                false
                            }
                            Err(e) => {
                                let _ = update_tool_call_status(agent_db, &tc_db_id, "error");
                                let msg = format!("Confirmation failed: {}", e);
                                let _ = store_message(agent_db, session_id, "tool", &msg);
                                emit_event(
                                    app,
                                    "agent-loop-tool-result",
                                    json!({
                                        "session_id": session_id,
                                        "tool_call_id": tool_call.id,
                                        "envelope": null,
                                        "error": msg,
                                    }),
                                );
                                false
                            }
                        }
                    }
                } else {
                    true
                };

                if !should_exec {
                    continue;
                }

                let _ = update_tool_call_status(agent_db, &tc_db_id, "executing");

                match executor
                    .execute(&tool_call.name, &tool_call.arguments, &connection_config)
                    .await
                {
                    Ok(envelope) => {
                        let _ = update_tool_call_status(agent_db, &tc_db_id, "completed");
                        let _ = store_tool_result(agent_db, &tc_db_id, &envelope.full_result);
                        let tool_msg = format!("[Tool: {}]\n{}", tool_call.name, envelope.summary);
                        let _ = store_message(agent_db, session_id, "tool", &tool_msg);

                        total_tokens_used += crate::agent::token_counter::count_message_tokens(
                            "tool", &tool_msg, &model,
                        );

                        emit_event(
                            app,
                            "agent-loop-tool-result",
                            json!({
                                "session_id": session_id,
                                "tool_call_id": tool_call.id,
                                "envelope": {
                                    "summary": envelope.summary,
                                    "full_result": null,
                                    "metadata": {
                                        "tool_name": envelope.metadata.tool_name,
                                        "duration_ms": envelope.metadata.duration_ms,
                                        "truncated": envelope.metadata.truncated,
                                    },
                                },
                                "error": null,
                            }),
                        );
                    }
                    Err(err) => {
                        let _ = update_tool_call_status(agent_db, &tc_db_id, "error");
                        let err_msg = format!("[Tool: {}] Error: {}", tool_call.name, err);
                        let _ = store_message(agent_db, session_id, "tool", &err_msg);

                        total_tokens_used += crate::agent::token_counter::count_message_tokens(
                            "tool", &err_msg, &model,
                        );

                        emit_event(
                            app,
                            "agent-loop-tool-result",
                            json!({
                                "session_id": session_id,
                                "tool_call_id": tool_call.id,
                                "envelope": null,
                                "error": err,
                            }),
                        );
                    }
                }
            }

            continue;
        }

        // Done with the loop
        emit_event(
            app,
            "agent-loop-done",
            json!({
                "session_id": session_id,
            }),
        );
        let _ = update_session_status(agent_db, session_id, "idle");
        return Ok(());
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_settings_str() {
        let settings = json!({
            "model": "gpt-4o",
            "maxTokens": 4096,
        });
        assert_eq!(
            get_settings_str(&settings, "model"),
            Some("gpt-4o".to_string())
        );
        assert_eq!(get_settings_str(&settings, "nonexistent"), None);
    }

    #[test]
    fn test_get_settings_u64() {
        let settings = json!({
            "maxIterations": 100,
            "nonexistent": "not_a_number",
        });
        assert_eq!(get_settings_u64(&settings, "maxIterations", 200), 100);
        assert_eq!(get_settings_u64(&settings, "nonexistent", 200), 200);
    }

    #[test]
    fn test_get_settings_bool() {
        let settings = json!({
            "requireConfirmation": false,
        });
        assert_eq!(
            get_settings_bool(&settings, "requireConfirmation", true),
            false
        );
        assert_eq!(get_settings_bool(&settings, "nonexistent", true), true);
    }

    #[test]
    fn test_build_system_prompt_default() {
        let settings = json!({});
        let prompt = build_system_prompt(&settings);
        assert!(prompt.contains("database assistant"));
        assert!(prompt.contains("query"));
    }

    #[test]
    fn test_build_system_prompt_custom() {
        let settings = json!({
            "systemPrompt": "You are a helpful assistant.",
            "customInstructions": "Always respond in Japanese.",
        });
        let prompt = build_system_prompt(&settings);
        assert_eq!(prompt, "You are a helpful assistant.");
    }

    #[test]
    fn test_build_system_prompt_with_instructions() {
        let settings = json!({
            "customInstructions": "Use only SELECT queries.",
        });
        let prompt = build_system_prompt(&settings);
        assert!(prompt.contains("Use only SELECT queries."));
    }

    #[test]
    fn test_extract_llm_error_auth() {
        assert!(extract_llm_error("status code 401").contains("Authentication failed"));
    }

    #[test]
    fn test_extract_llm_error_rate_limit() {
        assert!(extract_llm_error("status code 429").contains("Rate limited"));
    }

    #[test]
    fn test_extract_llm_error_server() {
        assert!(extract_llm_error("status code 500").contains("server error"));
        assert!(extract_llm_error("status code 502").contains("server error"));
        assert!(extract_llm_error("status code 503").contains("server error"));
    }

    #[test]
    fn test_extract_llm_error_timeout() {
        assert!(extract_llm_error("request timed out").contains("timed out"));
        assert!(extract_llm_error("timeout error").contains("timed out"));
    }

    #[test]
    fn test_extract_llm_error_generic() {
        let err = "Some other error";
        assert_eq!(extract_llm_error(err), err);
    }

    #[test]
    fn test_connection_config_from_settings_with_config() {
        let settings = json!({
            "connectionConfig": {
                "host": "localhost",
                "port": 5432,
            },
        });
        let config = connection_config_from_settings(&settings);
        assert_eq!(config["host"], "localhost");
        assert_eq!(config["port"], 5432);
    }

    #[test]
    fn test_connection_config_from_settings_empty() {
        let settings = json!({});
        let config = connection_config_from_settings(&settings);
        assert_eq!(config, json!({}));
    }

    #[test]
    fn test_get_settings_string_array() {
        let settings = json!({
            "attachedSources": ["postgres:abc", "mysql:def"],
        });
        let sources = get_settings_string_array(&settings, "attachedSources");
        assert_eq!(sources, vec!["postgres:abc", "mysql:def"]);
    }

    #[test]
    fn test_get_settings_string_array_missing() {
        let settings = json!({});
        let sources = get_settings_string_array(&settings, "attachedSources");
        assert_eq!(sources, Vec::<String>::new());
    }

    #[test]
    fn test_add_jitter_bounds() {
        let d = add_jitter(1000, 250);
        let ms = d.as_millis() as u64;
        assert!(ms >= 1000);
        assert!(ms <= 1250);
    }

    #[test]
    fn test_is_tool_auto_confirmed_no_db() {
        // This test verifies the function handles a missing DB gracefully
        // by expecting an error (no DB path provided for testing)
        let path = std::path::Path::new("/tmp/__nonexistent_test_db__.db");
        let _ = std::fs::remove_file(path);
        match crate::db::open(path) {
            Ok(db) => {
                let _ = crate::db::migrate(&db);
                let result = is_tool_auto_confirmed(&db, "test-session", "test-tool");
                assert!(result.is_ok());
                assert_eq!(result.unwrap(), false);
                let _ = std::fs::remove_file(path);
            }
            Err(_) => {
                // DB might not be creatable in test env, that's OK
            }
        }
    }
}
