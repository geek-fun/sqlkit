use serde_json::{json, Value};

use super::{ChatFormatter, ChunkResult, LlmMessage, LlmToolCall, ParseResult};

pub struct OpenAIChatFormatter;

impl ChatFormatter for OpenAIChatFormatter {
    fn format_messages(
        &self,
        messages: &[LlmMessage],
        system_prompt: &str,
    ) -> Result<Value, String> {
        let mut formatted = vec![];

        if !system_prompt.is_empty() {
            formatted.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }

        for msg in messages {
            formatted.push(json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        Ok(json!(formatted))
    }

    fn format_tools(&self, tools: &[Value]) -> Result<Value, String> {
        Ok(json!(tools))
    }

    fn parse_response(&self, response: &Value) -> Result<ParseResult, String> {
        let choice = response
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .ok_or_else(|| "No choices in response".to_string())?;

        let delta = choice.get("delta").or_else(|| choice.get("message"));
        let finish_reason = choice.get("finish_reason").and_then(|f| f.as_str());

        if let Some(msg) = delta {
            // Check for tool calls
            if let Some(tool_calls) = msg.get("tool_calls").and_then(|t| t.as_array()) {
                let calls: Vec<LlmToolCall> = tool_calls
                    .iter()
                    .filter_map(|tc| {
                        let id = tc.get("id")?.as_str()?.to_string();
                        let function = tc.get("function")?;
                        let name = function.get("name")?.as_str()?.to_string();
                        let args_str = function.get("arguments")?.as_str().unwrap_or("{}");
                        let arguments: Value =
                            serde_json::from_str(args_str).unwrap_or(json!({}));
                        Some(LlmToolCall {
                            id,
                            name,
                            arguments,
                        })
                    })
                    .collect();

                if !calls.is_empty() {
                    return Ok(ParseResult::ToolCalls(calls));
                }
            }

            let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
            return Ok(ParseResult::Content(content.to_string()));
        }

        if finish_reason == Some("stop") || finish_reason == Some("length") {
            let content = delta
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("");
            return Ok(ParseResult::Content(content.to_string()));
        }

        Ok(ParseResult::Content(String::new()))
    }

    fn parse_chunk(&self, chunk: &Value) -> Result<ChunkResult, String> {
        let choices = chunk
            .get("choices")
            .and_then(|c| c.as_array())
            .ok_or_else(|| "No choices in chunk".to_string())?;

        let mut content_delta: Option<String> = None;
        let mut tool_calls: Vec<LlmToolCall> = vec![];
        let mut finish_reason: Option<String> = None;

        for choice in choices {
            let delta = choice.get("delta");
            let fr = choice.get("finish_reason").and_then(|f| f.as_str());
            if let Some(reason) = fr {
                if !reason.is_empty() && reason != "null" {
                    finish_reason = Some(reason.to_string());
                }
            }

            if let Some(d) = delta {
                if let Some(content) = d.get("content").and_then(|c| c.as_str()) {
                    if !content.is_empty() {
                        content_delta = Some(content.to_string());
                    }
                }

                if let Some(tcs) = d.get("tool_calls").and_then(|t| t.as_array()) {
                    for tc in tcs {
                        let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let function = tc.get("function");
                        let name = function
                            .and_then(|f| f.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let args_str = function
                            .and_then(|f| f.get("arguments"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("{}");
                        let arguments: Value =
                            serde_json::from_str(args_str).unwrap_or(json!({}));
                        tool_calls.push(LlmToolCall {
                            id: id.to_string(),
                            name: name.to_string(),
                            arguments,
                        });
                    }
                }
            }
        }

        Ok(ChunkResult {
            content_delta,
            thinking_delta: None,
            tool_calls,
            finish_reason,
        })
    }
}
