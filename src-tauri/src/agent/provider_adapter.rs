pub fn map_to_api_compatibility(provider: &str) -> &str {
    match provider.to_lowercase().as_str() {
        "anthropic" | "claude" => "anthropic",
        "azure" | "azure_openai" => "azure",
        _ => "openai",
    }
}

pub fn default_base_url(api_compat: &str) -> String {
    match api_compat {
        "anthropic" => "https://api.anthropic.com".to_string(),
        "azure" => "https://api.openai.com".to_string(),
        _ => "https://api.openai.com/v1".to_string(),
    }
}

pub fn default_models(api_compat: &str) -> Vec<String> {
    match api_compat {
        "anthropic" => vec![
            "claude-sonnet-4-20250514".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
        ],
        _ => vec![
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-3.5-turbo".to_string(),
        ],
    }
}
