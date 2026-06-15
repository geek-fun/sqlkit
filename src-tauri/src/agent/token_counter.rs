#![allow(dead_code)]

use serde_json::Value;
use tiktoken_rs::{cl100k_base, o200k_base, CoreBPE};

use crate::agent::model_registry::{resolve_spec, ModelSpec, TokenizerFamily};

const PER_MESSAGE_OVERHEAD: usize = 4;
const PER_REPLY_OVERHEAD: usize = 3;

fn bpe_for(family: TokenizerFamily) -> Option<CoreBPE> {
    match family {
        TokenizerFamily::OpenAiO200k => o200k_base().ok(),
        TokenizerFamily::OpenAiCl100k => cl100k_base().ok(),
        _ => None,
    }
}

fn char_heuristic(text: &str, family: TokenizerFamily) -> usize {
    let chars = text.chars().count();
    let divisor: f64 = match family {
        TokenizerFamily::Anthropic => 3.5,
        TokenizerFamily::DeepSeek => 3.2,
        _ => 3.3,
    };
    ((chars as f64) / divisor).ceil() as usize
}

fn count_text(text: &str, family: TokenizerFamily) -> usize {
    if let Some(bpe) = bpe_for(family) {
        return bpe.encode_with_special_tokens(text).len();
    }
    char_heuristic(text, family)
}

pub fn count_chat_messages(messages: &[Value], spec: &ModelSpec) -> usize {
    let family = spec.tokenizer;
    let body_total: usize = messages
        .iter()
        .map(|m| count_single_message(m, family))
        .sum();
    body_total + PER_REPLY_OVERHEAD
}

fn count_single_message(message: &Value, family: TokenizerFamily) -> usize {
    let role = message.get("role").and_then(|v| v.as_str()).unwrap_or("");
    let mut total = PER_MESSAGE_OVERHEAD + count_text(role, family);

    if let Some(content) = message.get("content") {
        total += count_value_text(content, family);
    }
    if let Some(name) = message.get("name").and_then(|v| v.as_str()) {
        total += count_text(name, family);
    }
    if let Some(tcid) = message.get("tool_call_id").and_then(|v| v.as_str()) {
        total += count_text(tcid, family);
    }
    if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
        for tc in tool_calls {
            if let Some(func) = tc.get("function") {
                if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                    total += count_text(name, family);
                }
                if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                    total += count_text(args, family);
                }
            }
        }
    }
    total
}

fn count_value_text(value: &Value, family: TokenizerFamily) -> usize {
    match value {
        Value::String(s) => count_text(s, family),
        Value::Null => 0,
        Value::Array(items) => items.iter().map(|v| count_value_text(v, family)).sum(),
        Value::Object(_) => count_text(&value.to_string(), family),
        other => count_text(&other.to_string(), family),
    }
}

pub fn estimate_stored_message(role: &str, content: &str, spec: &ModelSpec) -> usize {
    let family = spec.tokenizer;
    PER_MESSAGE_OVERHEAD + count_text(role, family) + count_text(content, family)
}

pub fn count_tools_tokens(tools: &Value, spec: &ModelSpec) -> usize {
    if !tools.is_array() {
        return 0;
    }
    let family = spec.tokenizer;
    count_text(&tools.to_string(), family)
}

// ── Backward-compatible wrappers for existing callers ──────────────────

pub fn count_message_tokens(_role: &str, content: &str, model: &str) -> usize {
    let spec = resolve_spec_for_model(model);
    let family = spec.tokenizer;
    PER_MESSAGE_OVERHEAD + count_text(content, family)
}

pub fn count_tokens(text: &str, model: &str) -> usize {
    let spec = resolve_spec_for_model(model);
    count_text(text, spec.tokenizer)
}

fn resolve_spec_for_model(model: &str) -> ModelSpec {
    let lower = model.to_lowercase();
    if lower.contains("gpt-4")
        || lower.contains("gpt-3.5")
        || lower.contains("o1")
        || lower.contains("o3")
    {
        resolve_spec("OPEN_AI", model)
    } else if lower.contains("claude") {
        resolve_spec("anthropic", model)
    } else if lower.contains("deepseek") {
        resolve_spec("DEEP_SEEK", model)
    } else {
        ModelSpec {
            model_id: model.to_string(),
            context_window: 128_000,
            output_reserve: 4_096,
            tokenizer: TokenizerFamily::Generic,
        }
    }
}
