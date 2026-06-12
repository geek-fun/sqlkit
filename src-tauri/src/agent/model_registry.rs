use serde_json::{json, Value};

pub async fn get_provider_model_list(api_compat: &str, base_url: &str, api_key: &str) -> Result<Vec<Value>, String> {
    if api_key.is_empty() {
        return Ok(crate::agent::provider_adapter::default_models(api_compat)
            .into_iter()
            .map(|m| json!({"id": m}))
            .collect());
    }

    let url = match api_compat {
        "anthropic" => return Ok(crate::agent::provider_adapter::default_models(api_compat)
            .into_iter()
            .map(|m| json!({"id": m}))
            .collect()),
        _ => format!("{}/models", base_url.trim_end_matches('/')),
    };

    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse models response: {}", e))?;

    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        Ok(data
            .iter()
            .filter_map(|m| m.get("id").map(|id| json!({"id": id.clone()})))
            .collect())
    } else {
        Ok(crate::agent::provider_adapter::default_models(api_compat)
            .into_iter()
            .map(|m| json!({"id": m}))
            .collect())
    }
}
