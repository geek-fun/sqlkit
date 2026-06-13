use std::env;
use std::time::Duration;

fn get_proxy(http_proxy: Option<String>) -> Option<String> {
    if let Some(proxy) = http_proxy {
        if !proxy.is_empty() {
            return Some(proxy);
        }
    }
    env::var("HTTPS_PROXY")
        .ok()
        .or_else(|| env::var("https_proxy").ok())
        .or_else(|| env::var("HTTP_PROXY").ok())
        .or_else(|| env::var("http_proxy").ok())
        .or_else(|| {
            env::var("all_proxy")
                .ok()
                .filter(|p| p.starts_with("http://"))
        })
}

const CONNECT_TIMEOUT_SECS: u64 = 15;

pub fn create_http_client(
    proxy_mode: &str,
    proxy_url: Option<String>,
    ssl: Option<bool>,
    request_timeout: Option<Duration>,
) -> reqwest::Client {
    let mut builder = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(!ssl.unwrap_or(true))
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .no_proxy();

    if let Some(duration) = request_timeout {
        builder = builder.timeout(duration);
    }

    match proxy_mode {
        "manual" => {
            if let Some(proxy_url) = get_proxy(proxy_url) {
                match reqwest::Proxy::all(&proxy_url) {
                    Ok(proxy) => {
                        builder = builder.proxy(proxy);
                    }
                    Err(e) => {
                        eprintln!("[sqlkit] Failed to configure proxy '{}': {}", proxy_url, e);
                    }
                };
            }
        }
        "none" => {
            // no proxy — already set via .no_proxy()
        }
        _ => {
            // "system" or default — let reqwest use system env vars
            builder = builder.no_proxy();
            if let Some(proxy_url) = get_proxy(proxy_url) {
                match reqwest::Proxy::all(&proxy_url) {
                    Ok(proxy) => {
                        builder = builder.proxy(proxy);
                    }
                    Err(e) => {
                        eprintln!("[sqlkit] Failed to configure proxy '{}': {}", proxy_url, e);
                    }
                };
            }
        }
    }

    builder.build().unwrap_or_else(|e| {
        eprintln!("[sqlkit] Failed to build HTTP client: {}", e);
        reqwest::Client::new()
    })
}
