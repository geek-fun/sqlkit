use serde::{Deserialize, Serialize};

use crate::common::http_client::create_http_client;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TokenizerFamily {
    OpenAiCl100k,
    OpenAiO200k,
    Anthropic,
    DeepSeek,
    #[default]
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    pub model_id: String,
    pub context_window: usize,
    pub output_reserve: usize,
    #[serde(skip)]
    pub tokenizer: TokenizerFamily,
}

const DEFAULT_OPENAI_RESERVE: usize = 16_000;
const DEFAULT_ANTHROPIC_WINDOW: usize = 200_000;
const DEFAULT_ANTHROPIC_RESERVE: usize = 20_000;
const DEFAULT_DEEPSEEK_RESERVE: usize = 8_000;
const DEFAULT_OLLAMA_WINDOW: usize = 8_192;
const DEFAULT_GENERIC_WINDOW: usize = 32_768;
const DEFAULT_GENERIC_RESERVE: usize = 4_096;

const OPENAI_MODELS: &[(&str, usize, usize, TokenizerFamily)] = &[
    (
        "gpt-4o",
        128_000,
        DEFAULT_OPENAI_RESERVE,
        TokenizerFamily::OpenAiO200k,
    ),
    (
        "gpt-4o-mini",
        128_000,
        DEFAULT_OPENAI_RESERVE,
        TokenizerFamily::OpenAiO200k,
    ),
    ("gpt-4.1", 1_047_576, 32_000, TokenizerFamily::OpenAiO200k),
    (
        "gpt-4.1-mini",
        1_047_576,
        32_000,
        TokenizerFamily::OpenAiO200k,
    ),
    (
        "gpt-4.1-nano",
        1_047_576,
        32_000,
        TokenizerFamily::OpenAiO200k,
    ),
    (
        "gpt-4-turbo",
        128_000,
        DEFAULT_OPENAI_RESERVE,
        TokenizerFamily::OpenAiCl100k,
    ),
    ("gpt-4", 8_192, 4_096, TokenizerFamily::OpenAiCl100k),
    (
        "gpt-3.5-turbo",
        16_385,
        4_096,
        TokenizerFamily::OpenAiCl100k,
    ),
    ("o1", 200_000, 32_000, TokenizerFamily::OpenAiO200k),
    (
        "o1-mini",
        128_000,
        DEFAULT_OPENAI_RESERVE,
        TokenizerFamily::OpenAiO200k,
    ),
    ("o3", 200_000, 32_000, TokenizerFamily::OpenAiO200k),
    ("o3-mini", 200_000, 32_000, TokenizerFamily::OpenAiO200k),
];

const ANTHROPIC_MODELS: &[(&str, usize, usize)] = &[
    ("claude-sonnet-4-5", 200_000, DEFAULT_ANTHROPIC_RESERVE),
    ("claude-sonnet-4", 200_000, DEFAULT_ANTHROPIC_RESERVE),
    ("claude-opus-4", 200_000, DEFAULT_ANTHROPIC_RESERVE),
    ("claude-3-5-sonnet", 200_000, DEFAULT_ANTHROPIC_RESERVE),
    ("claude-3-5-haiku", 200_000, DEFAULT_ANTHROPIC_RESERVE),
    ("claude-3-opus", 200_000, DEFAULT_ANTHROPIC_RESERVE),
];

const DEEPSEEK_MODELS: &[(&str, usize, usize)] = &[
    ("deepseek-chat", 128_000, DEFAULT_DEEPSEEK_RESERVE),
    ("deepseek-reasoner", 128_000, DEFAULT_DEEPSEEK_RESERVE),
    ("deepseek-coder", 128_000, DEFAULT_DEEPSEEK_RESERVE),
];

fn matches_prefix(id: &str, prefix: &str) -> bool {
    id == prefix || id.starts_with(prefix)
}

pub fn resolve_spec(provider: &str, model_id: &str) -> ModelSpec {
    let lower = model_id.to_lowercase();
    if provider == "DEEP_SEEK" || lower.starts_with("deepseek") {
        for (id, ctx, reserve) in DEEPSEEK_MODELS {
            if matches_prefix(&lower, id) {
                return ModelSpec {
                    model_id: model_id.to_string(),
                    context_window: *ctx,
                    output_reserve: *reserve,
                    tokenizer: TokenizerFamily::DeepSeek,
                };
            }
        }
        return ModelSpec {
            model_id: model_id.to_string(),
            context_window: 128_000,
            output_reserve: DEFAULT_DEEPSEEK_RESERVE,
            tokenizer: TokenizerFamily::DeepSeek,
        };
    }

    if provider == "anthropic" || lower.starts_with("claude") {
        for (id, ctx, reserve) in ANTHROPIC_MODELS {
            if matches_prefix(&lower, id) {
                return ModelSpec {
                    model_id: model_id.to_string(),
                    context_window: *ctx,
                    output_reserve: *reserve,
                    tokenizer: TokenizerFamily::Anthropic,
                };
            }
        }
        return ModelSpec {
            model_id: model_id.to_string(),
            context_window: DEFAULT_ANTHROPIC_WINDOW,
            output_reserve: DEFAULT_ANTHROPIC_RESERVE,
            tokenizer: TokenizerFamily::Anthropic,
        };
    }

    if provider == "OLLAMA" || provider == "LM_STUDIO" {
        return ModelSpec {
            model_id: model_id.to_string(),
            context_window: DEFAULT_OLLAMA_WINDOW,
            output_reserve: 2_048,
            tokenizer: TokenizerFamily::Generic,
        };
    }

    for (id, ctx, reserve, tk) in OPENAI_MODELS {
        if matches_prefix(&lower, id) {
            return ModelSpec {
                model_id: model_id.to_string(),
                context_window: *ctx,
                output_reserve: *reserve,
                tokenizer: *tk,
            };
        }
    }

    ModelSpec {
        model_id: model_id.to_string(),
        context_window: DEFAULT_GENERIC_WINDOW,
        output_reserve: DEFAULT_GENERIC_RESERVE,
        tokenizer: TokenizerFamily::Generic,
    }
}

pub fn apply_overrides(spec: ModelSpec, context_window_override: Option<usize>) -> ModelSpec {
    match context_window_override {
        Some(w) if w >= 1_024 => ModelSpec {
            context_window: w,
            ..spec
        },
        _ => spec,
    }
}

pub fn usable_window(spec: &ModelSpec) -> usize {
    spec.context_window.saturating_sub(spec.output_reserve)
}

pub async fn get_provider_model_list(
    api_compat: &str,
    base_url: &str,
    api_key: &str,
    proxy_url: Option<String>,
    proxy_mode: &str,
) -> Result<Vec<serde_json::Value>, String> {
    if api_key.is_empty() {
        return Ok(default_models_for_api(api_compat)
            .into_iter()
            .map(|m| serde_json::json!({"id": m}))
            .collect());
    }

    let url = match api_compat {
        "anthropic" => {
            return Ok(default_models_for_api(api_compat)
                .into_iter()
                .map(|m| serde_json::json!({"id": m}))
                .collect())
        }
        _ => format!("{}/models", base_url.trim_end_matches('/')),
    };

    let client = create_http_client(proxy_mode, proxy_url, Some(true), None);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse models response: {}", e))?;

    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        Ok(data
            .iter()
            .filter_map(|m| m.get("id").map(|id| serde_json::json!({"id": id.clone()})))
            .collect())
    } else {
        Ok(default_models_for_api(api_compat)
            .into_iter()
            .map(|m| serde_json::json!({"id": m}))
            .collect())
    }
}

fn default_models_for_api(api_compat: &str) -> Vec<String> {
    match api_compat {
        "anthropic" => vec![
            "claude-sonnet-4-20250514".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        ],
        _ => vec![
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "gpt-4-turbo".to_string(),
        ],
    }
}
