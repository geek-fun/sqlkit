use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use tauri::Emitter;

use crate::agent::chat_formatter::{ChatFormatter, OpenAIChatFormatter};
use crate::agent::model_registry;
use crate::agent::provider_adapter;
use crate::common::http_client::create_http_client;

fn sanitize_error(msg: &str) -> String {
    let mut result = msg.to_string();
    let bearer_prefix = "Bearer ";
    while let Some(pos) = result.find(bearer_prefix) {
        let start = pos + bearer_prefix.len();
        if start < result.len() {
            let tail = result[start..].to_string();
            let end = tail
                .find(|c: char| {
                    c.is_whitespace()
                        || c == '"'
                        || c == '\''
                        || c == ','
                        || c == '}'
                        || c == ']'
                        || c == ')'
                })
                .map(|e| start + e)
                .unwrap_or(result.len());
            result.replace_range(start..end, "[REDACTED]");
        } else {
            break;
        }
    }
    let sk_prefix = "sk-";
    while let Some(pos) = result.find(sk_prefix) {
        let tail = result[pos..].to_string();
        let end = tail
            .find(|c: char| {
                c.is_whitespace()
                    || c == '"'
                    || c == '\''
                    || c == ','
                    || c == '}'
                    || c == ']'
                    || c == ')'
            })
            .map(|e| pos + e)
            .unwrap_or(result.len());
        result.replace_range(pos..end, "sk-[REDACTED]");
    }
    result
}

fn to_openai_messages(
    messages: &[Value],
) -> Result<Vec<async_openai::types::ChatCompletionRequestMessage>, String> {
    use async_openai::types::{
        ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent, ChatCompletionRequestToolMessage,
        ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent,
    };
    let mut out = Vec::with_capacity(messages.len());
    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
        match role {
            "system" => {
                let c = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                out.push(ChatCompletionRequestMessage::System(
                    ChatCompletionRequestSystemMessage {
                        content: ChatCompletionRequestSystemMessageContent::Text(c.to_string()),
                        name: None,
                    },
                ));
            }
            "user" => {
                let c = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                out.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Text(c.to_string()),
                        name: None,
                    },
                ));
            }
            "assistant" => {
                let c = msg.get("content").and_then(|c| c.as_str()).and_then(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                });
                out.push(ChatCompletionRequestMessage::Assistant(
                    ChatCompletionRequestAssistantMessage {
                        content: c.map(ChatCompletionRequestAssistantMessageContent::Text),
                        name: None,
                        tool_calls: None,
                        refusal: None,
                        #[allow(deprecated)]
                        function_call: None,
                    },
                ));
            }
            "tool" => {
                let c = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                let id = msg
                    .get("tool_call_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                out.push(ChatCompletionRequestMessage::Tool(
                    ChatCompletionRequestToolMessage {
                        content: ChatCompletionRequestToolMessageContent::Text(c.to_string()),
                        tool_call_id: id.to_string(),
                    },
                ));
            }
            _ => {
                let c = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                out.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Text(c.to_string()),
                        name: None,
                    },
                ));
            }
        }
    }
    Ok(out)
}

fn to_openai_tools(tools: &[Value]) -> Vec<async_openai::types::ChatCompletionTool> {
    use async_openai::types::{ChatCompletionTool, ChatCompletionToolType, FunctionObject};
    tools
        .iter()
        .filter_map(|t| {
            let f = t.get("function")?;
            Some(ChatCompletionTool {
                r#type: ChatCompletionToolType::Function,
                function: FunctionObject {
                    name: f.get("name")?.as_str()?.to_string(),
                    description: f
                        .get("description")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string()),
                    parameters: f.get("parameters").cloned(),
                    strict: None,
                },
            })
        })
        .collect()
}

#[tauri::command]
pub async fn run_agent_step(
    window: tauri::Window,
    request_id: String,
    provider: String,
    model: String,
    messages: Vec<Value>,
    tools: Vec<Value>,
    http_proxy: Option<String>,
    proxy_mode: Option<String>,
    api_key: String,
    base_url: Option<String>,
) -> Result<String, String> {
    let api_compat = provider_adapter::map_to_api_compatibility(&provider);
    let resolved_base = base_url
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| provider_adapter::default_base_url(api_compat));
    let result = match api_compat {
        "anthropic" => {
            run_anthropic(
                &window,
                &request_id,
                &model,
                &messages,
                &tools,
                &resolved_base,
                &api_key,
            )
            .await
        }
        _ => {
            run_openai(
                &window,
                &request_id,
                &model,
                &messages,
                &tools,
                &resolved_base,
                &api_key,
                http_proxy.as_deref(),
                proxy_mode.as_deref(),
            )
            .await
        }
    }?;
    serde_json::to_string(&result)
        .map_err(|e| format!("Serialize failed: {}", sanitize_error(&e.to_string())))
}

async fn run_openai(
    window: &tauri::Window,
    request_id: &str,
    model: &str,
    messages: &[Value],
    tools: &[Value],
    base_url: &str,
    api_key: &str,
    http_proxy: Option<&str>,
    proxy_mode: Option<&str>,
) -> Result<Value, String> {
    use async_openai::types::CreateChatCompletionRequest;
    use async_openai::Client;

    let http_client = create_http_client(
        proxy_mode.unwrap_or("none"),
        http_proxy.map(|s| s.to_string()),
        None,
        None,
    );
    let config = async_openai::config::OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(base_url);
    let client = Client::with_config(config).with_http_client(http_client);

    let oai_msgs = to_openai_messages(messages)?;
    let oai_tools = to_openai_tools(tools);

    let request = CreateChatCompletionRequest {
        model: model.to_string(),
        messages: oai_msgs,
        stream: Some(true),
        tools: if oai_tools.is_empty() {
            None
        } else {
            Some(oai_tools)
        },
        ..Default::default()
    };

    let mut stream = client
        .chat()
        .create_stream(request)
        .await
        .map_err(|e| format!("Stream error: {}", e))?;
    let formatter = OpenAIChatFormatter;
    let mut full = String::new();
    let mut calls: Vec<Value> = Vec::new();
    let mut finish: Option<String> = None;

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("Chunk error: {}", e))?;
        let cv = serde_json::to_value(&chunk).map_err(|e| format!("Ser error: {}", e))?;
        let cr = formatter
            .parse_chunk(&cv)
            .map_err(|e| format!("Parse error: {}", e))?;
        if let Some(ref c) = cr.content_delta {
            if !c.is_empty() {
                let _ = window.emit(
                    "agent-delta",
                    json!({"requestId": request_id, "content": c}),
                );
                full.push_str(c);
            }
        }
        for tc in &cr.tool_calls {
            let p = json!({"index": calls.len(), "id": tc.id, "type": "function", "function": {"name": tc.name, "arguments": tc.arguments.to_string()}});
            let _ = window.emit(
                "agent-tool-call",
                json!({"requestId": request_id, "toolCall": p}),
            );
            calls.push(p);
        }
        if let Some(ref r) = cr.finish_reason {
            finish = Some(r.clone());
        }
    }

    let result = json!({"requestId": request_id, "content": full, "toolCalls": calls, "finishReason": finish, "type": "step_done"});
    let _ = window.emit("agent-step-done", result.clone());
    Ok(result)
}

fn parse_sse_data(line: &str) -> Option<Value> {
    let line = line.trim();
    if line.is_empty() || !line.starts_with("data: ") {
        return None;
    }
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return None;
    }
    serde_json::from_str(data).ok()
}

async fn run_anthropic(
    window: &tauri::Window,
    request_id: &str,
    model: &str,
    messages: &[Value],
    tools: &[Value],
    base_url: &str,
    api_key: &str,
) -> Result<Value, String> {
    let client = create_http_client(
        "none",
        None,
        None,
        Some(std::time::Duration::from_secs(300)),
    );
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key).map_err(|e| format!("Bad key: {}", e))?,
    );
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

    let sys = messages
        .iter()
        .find(|m| m.get("role").and_then(|v| v.as_str()) == Some("system"))
        .and_then(|m| m.get("content").and_then(|v| v.as_str()))
        .unwrap_or("");

    let msgs: Vec<Value> = messages
        .iter()
        .filter_map(|m| {
            let role = m.get("role").and_then(|v| v.as_str())?;
            if role == "system" {
                return None;
            }
            let c = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
            Some(json!({"role": role, "content": c}))
        })
        .collect();

    let anthropic_tools: Vec<Value> = tools.iter().filter_map(|t| {
        let f = t.get("function")?;
        Some(json!({
            "name": f.get("name")?.as_str()?,
            "description": f.get("description").and_then(|d| d.as_str()).unwrap_or(""),
            "input_schema": f.get("parameters").cloned().unwrap_or(json!({"type": "object", "properties": {}})),
        }))
    }).collect();

    let mut body = json!({"model": model, "messages": msgs, "max_tokens": 4096, "stream": true});
    if !sys.is_empty() {
        body["system"] = json!(sys);
    }
    if !anthropic_tools.is_empty() {
        body["tools"] = json!(anthropic_tools);
    }

    let url = format!(
        "{}/v1/messages",
        base_url.trim_end_matches('/').trim_end_matches("/v1")
    );
    let resp = client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Req failed: {}", e))?;
    if !resp.status().is_success() {
        let s = resp.status();
        let t = resp.text().await.unwrap_or_default();
        return Err(format!("Anthropic {}: {}", s, t));
    }

    let mut full = String::new();
    let mut calls: Vec<Value> = Vec::new();
    let mut tool_use: Option<(String, String, String)> = None;
    let mut finish: Option<String> = None;
    let mut buf = String::new();
    let mut stream = resp.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("Read error: {}", e))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        loop {
            let nl = match buf.find('\n') {
                Some(p) => p,
                None => break,
            };
            let line = buf[..nl].to_string();
            buf = buf[nl + 1..].to_string();
            if line.trim().is_empty() || line.starts_with("event: ") {
                continue;
            }
            let Some(data) = parse_sse_data(&line) else {
                continue;
            };
            match data.get("type").and_then(|t| t.as_str()) {
                Some("content_block_start") => {
                    if data
                        .get("content_block")
                        .and_then(|b| b.get("type"))
                        .and_then(|t| t.as_str())
                        == Some("tool_use")
                    {
                        let id = data
                            .get("content_block")
                            .and_then(|b| b.get("id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let nm = data
                            .get("content_block")
                            .and_then(|b| b.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        tool_use = Some((id, nm, String::new()));
                    }
                }
                Some("content_block_delta") => {
                    if let Some(text) = data
                        .get("delta")
                        .and_then(|d| d.get("text"))
                        .and_then(|v| v.as_str())
                    {
                        if !text.is_empty() {
                            let _ = window.emit(
                                "agent-delta",
                                json!({"requestId": request_id, "content": text}),
                            );
                            full.push_str(text);
                        }
                    }
                    if data
                        .get("delta")
                        .and_then(|d| d.get("type"))
                        .and_then(|t| t.as_str())
                        == Some("input_json_delta")
                    {
                        let partial = data
                            .get("delta")
                            .and_then(|d| d.get("partial_json"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if let Some((_, _, ref mut acc)) = tool_use {
                            acc.push_str(partial);
                        }
                    }
                }
                Some("content_block_stop") => {
                    if let Some((id, nm, json_input)) = tool_use.take() {
                        let args: Value = serde_json::from_str(&json_input).unwrap_or(json!({}));
                        let p = json!({"index": calls.len(), "id": id, "type": "function", "function": {"name": nm, "arguments": args.to_string()}});
                        let _ = window.emit(
                            "agent-tool-call",
                            json!({"requestId": request_id, "toolCall": p}),
                        );
                        calls.push(p);
                    }
                }
                Some("message_delta") => {
                    if let Some(r) = data.get("delta").and_then(|d| d.get("stop_reason")) {
                        finish = r.as_str().map(|s| s.to_string());
                    }
                }
                Some("error") => {
                    let msg = data
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    return Err(msg.to_string());
                }
                _ => {}
            }
        }
    }
    if let Some((id, nm, json_input)) = tool_use.take() {
        let args: Value = serde_json::from_str(&json_input).unwrap_or(json!({}));
        let p = json!({"index": calls.len(), "id": id, "type": "function", "function": {"name": nm, "arguments": args.to_string()}});
        let _ = window.emit(
            "agent-tool-call",
            json!({"requestId": request_id, "toolCall": p}),
        );
        calls.push(p);
    }

    let result = json!({"requestId": request_id, "content": full, "toolCalls": calls, "finishReason": finish, "type": "step_done"});
    let _ = window.emit("agent-step-done", result.clone());
    Ok(result)
}

#[tauri::command]
pub async fn validate_llm_config(
    provider: String,
    api_key: String,
    model: String,
    http_proxy: Option<String>,
    proxy_mode: Option<String>,
    base_url: Option<String>,
) -> Result<bool, String> {
    let api_compat = provider_adapter::map_to_api_compatibility(&provider);
    let resolved_base = base_url
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| provider_adapter::default_base_url(api_compat));
    let client = create_http_client(&proxy_mode.unwrap_or_default(), http_proxy, None, None);
    match api_compat {
        "anthropic" => {
            let url = format!(
                "{}/v1/messages",
                resolved_base.trim_end_matches('/').trim_end_matches("/v1")
            );
            let body = json!({"model": model, "max_tokens": 10, "messages": [{"role": "user", "content": "ping"}]});
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            headers.insert(
                "x-api-key",
                HeaderValue::from_str(&api_key).map_err(|e| sanitize_error(&e.to_string()))?,
            );
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
            let resp = client
                .post(&url)
                .headers(headers)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Req failed: {}", sanitize_error(&e.to_string())))?;
            Ok(resp.status().is_success())
        }
        _ => {
            let url = format!("{}/models", resolved_base.trim_end_matches('/'));
            let resp = client
                .get(&url)
                .header(AUTHORIZATION, format!("Bearer {}", api_key))
                .send()
                .await
                .map_err(|e| format!("Req failed: {}", sanitize_error(&e.to_string())))?;
            Ok(resp.status().is_success())
        }
    }
}

#[tauri::command]
pub async fn list_llm_models(
    provider: String,
    api_key: String,
    base_url: Option<String>,
) -> Result<Vec<String>, String> {
    let api_compat = provider_adapter::map_to_api_compatibility(&provider);
    let resolved_base = base_url
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| provider_adapter::default_base_url(api_compat));
    let models = model_registry::get_provider_model_list(api_compat, &resolved_base, &api_key)
        .await
        .map_err(|e| sanitize_error(&e))?;
    Ok(models
        .into_iter()
        .filter_map(|m| {
            m.get("id")
                .and_then(|id| id.as_str().map(|s| s.to_string()))
        })
        .collect())
}
