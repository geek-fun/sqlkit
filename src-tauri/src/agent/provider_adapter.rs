use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Endpoint config table — one entry per apiCompatibility mode
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct ApiEndpointConfig {
    chat_path: &'static str,
    models_path: &'static str,
    auth_header_name: &'static str,
    auth_header_prefix: &'static str,
    models_extractor: fn(&Value) -> Vec<String>,
}

static OPENAI_ENDPOINT: ApiEndpointConfig = ApiEndpointConfig {
    chat_path: "/v1/chat/completions",
    models_path: "/v1/models",
    auth_header_name: "Authorization",
    auth_header_prefix: "Bearer ",
    models_extractor: openai_models_extractor,
};

static ANTHROPIC_ENDPOINT: ApiEndpointConfig = ApiEndpointConfig {
    chat_path: "/v1/messages",
    models_path: "/v1/models",
    auth_header_name: "x-api-key",
    auth_header_prefix: "",
    models_extractor: openai_models_extractor,
};

static LOCAL_ENDPOINT: ApiEndpointConfig = ApiEndpointConfig {
    chat_path: "/api/chat",
    models_path: "/api/tags",
    auth_header_name: "",
    auth_header_prefix: "",
    models_extractor: local_models_extractor,
};

fn endpoint_for(api_compatibility: &str) -> &'static ApiEndpointConfig {
    match api_compatibility {
        "local" => &LOCAL_ENDPOINT,
        "anthropic" => &ANTHROPIC_ENDPOINT,
        _ => &OPENAI_ENDPOINT,
    }
}

// ---------------------------------------------------------------------------
// Model ID extractors (dispatched via endpoint config function pointer)
// ---------------------------------------------------------------------------

fn openai_models_extractor(payload: &Value) -> Vec<String> {
    payload
        .get("data")
        .and_then(|data| data.as_array())
        .map(|models| {
            models
                .iter()
                .filter_map(|model| model.get("id").and_then(|v| v.as_str()))
                .map(|v| v.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn local_models_extractor(payload: &Value) -> Vec<String> {
    payload
        .get("models")
        .and_then(|models| models.as_array())
        .map(|models| {
            models
                .iter()
                .filter_map(|model| {
                    model
                        .get("name")
                        .or_else(|| model.get("model"))
                        .or_else(|| model.get("id"))
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Strip trailing slashes and ensure a `/v1` suffix.
pub fn normalize_base_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    let without_slashes = trimmed.trim_end_matches('/');
    if without_slashes.ends_with("/v1") {
        without_slashes.to_string()
    } else {
        format!("{}/v1", without_slashes)
    }
}

pub fn default_base_url(api_compatibility: &str) -> String {
    match api_compatibility {
        "local" => "http://127.0.0.1:11434/v1".to_string(),
        "anthropic" => "https://api.anthropic.com/v1".to_string(),
        _ => "https://api.openai.com/v1".to_string(),
    }
}

pub fn default_models(api_compat: &str) -> Vec<String> {
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
pub fn get_base_url(settings: &Value) -> String {
    let api_compatibility = settings
        .get("apiCompatibility")
        .and_then(|v| v.as_str())
        .unwrap_or("openai-compatible");
    let explicit = settings.get("baseUrl").and_then(|v| v.as_str());

    if let Some(url) = explicit {
        if !url.trim().is_empty() {
            return normalize_base_url(url);
        }
    }

    default_base_url(api_compatibility)
}

pub fn build_headers(settings: &Value) -> Result<HeaderMap, String> {
    let api_compatibility = settings
        .get("apiCompatibility")
        .and_then(|v| v.as_str())
        .unwrap_or("openai-compatible");
    let api_key = settings
        .get("apiKey")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let config = endpoint_for(api_compatibility);
    let mut headers = HeaderMap::new();

    if !api_key.is_empty() && !config.auth_header_name.is_empty() {
        let header_value = if config.auth_header_prefix.is_empty() {
            format!("{}", api_key)
        } else {
            format!("{}{}", config.auth_header_prefix, api_key)
        };
        let value = HeaderValue::from_str(&header_value)
            .map_err(|e| format!("Invalid header value: {}", e))?;
        let header_name = reqwest::header::HeaderName::try_from(config.auth_header_name)
            .map_err(|e| format!("Invalid header name '{}': {}", config.auth_header_name, e))?;
        headers.insert(header_name, value);
    }

    Ok(headers)
}

pub fn map_to_api_compatibility(provider: &str) -> &'static str {
    match provider {
        "OLLAMA" => "local",
        "openai-compatible" | "OPENAI" | "DEEP_SEEK" | "OPENROUTER" | "LM_STUDIO" => {
            "openai-compatible"
        }
        "custom-anthropic" | "anthropic" | "ANTHROPIC" => "anthropic",
        "local" => "local",
        _ => "openai-compatible",
    }
}

pub fn extract_model_ids(api_compatibility: &str, payload: &Value) -> Vec<String> {
    if api_compatibility == "openai-compatible" {
        let ids = openai_models_extractor(payload);
        if !ids.is_empty() {
            return ids;
        }
        return payload
            .get("models")
            .and_then(|models| models.as_array())
            .map(|models| {
                models
                    .iter()
                    .filter_map(|model| {
                        model
                            .get("key")
                            .and_then(|v| v.as_str())
                            .map(|v| v.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
    }

    let config = endpoint_for(api_compatibility);
    (config.models_extractor)(payload)
}

pub fn get_native_api_url(provider: &str, normalized_base_url: &str, endpoint: &str) -> String {
    let base_without_v1 = normalized_base_url.trim_end_matches("/v1");
    match provider {
        "OLLAMA" | "LM_STUDIO" => format!("{}/{}", base_without_v1, endpoint),
        _ => format!("{}/{}", normalized_base_url, endpoint),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_base_url() {
        assert_eq!(
            normalize_base_url("https://api.openai.com"),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            normalize_base_url("https://api.openai.com/"),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            normalize_base_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1"
        );
        assert_eq!(normalize_base_url(""), "");
        assert_eq!(normalize_base_url("  "), "");
    }

    #[test]
    fn test_get_base_url_openai() {
        let s = json!({"apiCompatibility": "openai-compatible"});
        assert_eq!(get_base_url(&s), "https://api.openai.com/v1");
    }

    #[test]
    fn test_get_base_url_local() {
        let s = json!({"apiCompatibility": "local"});
        assert_eq!(get_base_url(&s), "http://127.0.0.1:11434/v1");
    }

    #[test]
    fn test_get_base_url_anthropic() {
        let s = json!({"apiCompatibility": "anthropic"});
        assert_eq!(get_base_url(&s), "https://api.anthropic.com/v1");
    }

    #[test]
    fn test_get_base_url_explicit() {
        let s = json!({"apiCompatibility": "openai-compatible", "baseUrl": "https://custom.example.com"});
        assert_eq!(get_base_url(&s), "https://custom.example.com/v1");
    }

    #[test]
    fn test_extract_model_ids_openai() {
        let payload = json!({
            "data": [
                {"id": "gpt-4"},
                {"id": "gpt-3.5-turbo"}
            ]
        });
        let ids = extract_model_ids("openai-compatible", &payload);
        assert_eq!(ids, vec!["gpt-4", "gpt-3.5-turbo"]);
    }

    #[test]
    fn test_extract_model_ids_local() {
        let payload = json!({
            "models": [
                {"name": "llama3.1"},
                {"name": "mistral"}
            ]
        });
        let ids = extract_model_ids("local", &payload);
        assert_eq!(ids, vec!["llama3.1", "mistral"]);
    }

    #[test]
    fn test_extract_model_ids_lm_studio_fallback() {
        let payload = json!({
            "models": [
                {"key": "model-a"},
                {"key": "model-b"}
            ]
        });
        let ids = extract_model_ids("openai-compatible", &payload);
        assert_eq!(ids, vec!["model-a", "model-b"]);
    }

    #[test]
    fn test_map_to_api_compatibility() {
        assert_eq!(map_to_api_compatibility("OLLAMA"), "local");
        assert_eq!(map_to_api_compatibility("OPENAI"), "openai-compatible");
        assert_eq!(map_to_api_compatibility("DEEP_SEEK"), "openai-compatible");
        assert_eq!(map_to_api_compatibility("OPENROUTER"), "openai-compatible");
        assert_eq!(map_to_api_compatibility("LM_STUDIO"), "openai-compatible");
    }

    #[test]
    fn test_get_native_api_url() {
        let base = "http://127.0.0.1:11434/v1";
        assert_eq!(
            get_native_api_url("OLLAMA", base, "api/tags"),
            "http://127.0.0.1:11434/api/tags"
        );
        assert_eq!(
            get_native_api_url("LM_STUDIO", base, "api/v1/models"),
            "http://127.0.0.1:11434/api/v1/models"
        );
        assert_eq!(
            get_native_api_url("OPENAI", "https://api.openai.com/v1", "models"),
            "https://api.openai.com/v1/models"
        );
    }
}
