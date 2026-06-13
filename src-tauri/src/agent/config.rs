use serde_json::Value;

use crate::agent::provider_adapter;

pub fn build_headers(api_key: &str) -> reqwest::header::HeaderMap {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if !api_key.is_empty() {
        let bearer = format!("Bearer {}", api_key);
        if let Ok(val) = HeaderValue::from_str(&bearer) {
            headers.insert(AUTHORIZATION, val);
        }
    }
    headers
}

pub fn get_base_url(settings: &Value) -> String {
    let api_compat = settings
        .get("apiCompatibility")
        .and_then(|v| v.as_str())
        .unwrap_or("openai");

    if let Some(base_url) = settings.get("baseUrl").and_then(|v| v.as_str()) {
        if !base_url.is_empty() {
            return base_url.to_string();
        }
    }

    provider_adapter::default_base_url(api_compat)
}
