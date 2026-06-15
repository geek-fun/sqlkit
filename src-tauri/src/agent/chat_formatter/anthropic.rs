use serde_json::{json, Value};

use super::{ChatFormatter, ChunkResult, LlmMessage, LlmToolCall, ParseResult};

pub struct AnthropicChatFormatter;

impl ChatFormatter for AnthropicChatFormatter {
    fn format_messages(
        &self,
        messages: &[LlmMessage],
        _system_prompt: &str,
    ) -> Result<Value, String> {
        let formatted: Vec<Value> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();
        Ok(json!(formatted))
    }

    fn format_tools(&self, tools: &[Value]) -> Result<Value, String> {
        let anthropic_tools: Vec<Value> = tools
            .iter()
            .filter_map(|t| {
                let function = t.get("function")?;
                Some(json!({
                    "name": function.get("name")?.as_str()?,
                    "description": function.get("description")?.as_str()?,
                    "input_schema": function.get("parameters")?
                }))
            })
            .collect();
        Ok(json!(anthropic_tools))
    }

    fn parse_response(&self, response: &Value) -> Result<ParseResult, String> {
        let content = response.get("content").and_then(|c| c.as_array());
        if let Some(blocks) = content {
            for block in blocks {
                if block.get("type") == Some(&json!("tool_use")) {
                    let tool_calls = vec![LlmToolCall {
                        id: block
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        name: block
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        arguments: block.get("input").cloned().unwrap_or(json!({})),
                    }];
                    return Ok(ParseResult::ToolCalls(tool_calls));
                }
                if block.get("type") == Some(&json!("text")) {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        return Ok(ParseResult::Content(text.to_string()));
                    }
                }
            }
        }
        Ok(ParseResult::Content(String::new()))
    }

    fn parse_chunk(&self, chunk: &Value) -> Result<ChunkResult, String> {
        let event_type = chunk.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let mut content_delta: Option<String> = None;
        let mut thinking_delta: Option<String> = None;
        let mut tool_calls: Vec<LlmToolCall> = vec![];
        let mut finish_reason: Option<String> = None;

        match event_type {
            "content_block_start" => {
                let block = chunk.get("content_block");
                if let Some(b) = block {
                    match b.get("type").and_then(|v| v.as_str()) {
                        Some("tool_use") => {
                            let id = b
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let name = b
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let input = b.get("input").cloned().unwrap_or(json!({}));
                            tool_calls.push(LlmToolCall {
                                id,
                                name,
                                arguments: input,
                            });
                        }
                        _ => {
                            if let Some(text) = b.get("text").and_then(|v| v.as_str()) {
                                content_delta = Some(text.to_string());
                            }
                        }
                    }
                }
            }
            "content_block_delta" => {
                let delta = chunk.get("delta");
                if let Some(d) = delta {
                    match d.get("type").and_then(|v| v.as_str()) {
                        Some("input_json_delta") => {
                            // Tool call argument delta — handled via content_block_stop
                        }
                        Some("thinking_delta") => {
                            if let Some(thinking) = d.get("thinking").and_then(|v| v.as_str()) {
                                thinking_delta = Some(thinking.to_string());
                            }
                        }
                        _ => {
                            if let Some(text) = d.get("text").and_then(|v| v.as_str()) {
                                content_delta = Some(text.to_string());
                            }
                        }
                    }
                }
            }
            "content_block_stop" => {
                // Tool call finalization happens at message level
            }
            "message_delta" => {
                let delta = chunk.get("delta");
                if let Some(d) = delta {
                    // Check stop_reason for tool_use
                    if let Some(reason) = d.get("stop_reason").and_then(|v| v.as_str()) {
                        finish_reason = match reason {
                            "tool_use" => Some("tool_calls".to_string()),
                            "end_turn" => Some("stop".to_string()),
                            "max_tokens" => Some("length".to_string()),
                            _ => Some(reason.to_string()),
                        };
                    }
                    // Also check stop_sequence
                    if let Some(text) = d.get("text").and_then(|v| v.as_str()) {
                        content_delta = Some(text.to_string());
                    }
                }
            }
            "message_start" => {
                let msg = chunk.get("message");
                if let Some(m) = msg {
                    if let Some(content) = m.get("content").and_then(|c| c.as_array()) {
                        for block in content {
                            if block.get("type") == Some(&json!("tool_use")) {
                                let id = block
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let name = block
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let args = block.get("input").cloned().unwrap_or(json!({}));
                                tool_calls.push(LlmToolCall {
                                    id,
                                    name,
                                    arguments: args,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(ChunkResult {
            content_delta,
            thinking_delta,
            tool_calls,
            finish_reason,
        })
    }
}
