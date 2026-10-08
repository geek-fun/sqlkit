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

/// macOS network configuration (SCDynamicStore) or the Windows registry.
/// Honors NO_PROXY / the OS exception list for the target host.
pub fn system_proxy_for(host: &str, port: u16) -> Option<String> {
    use hyper_util::client::proxy::matcher::Matcher;

    let matcher = Matcher::from_system();
    for scheme in ["https", "http"] {
        let uri: http::Uri = format!("{scheme}://{host}:{port}").parse().ok()?;
        if let Some(intercept) = matcher.intercept(&uri) {
            // hyper-util's macOS matcher ignores the OS ExceptionsList, so
            // honor it here: exempted targets bypass the proxy entirely.
            if macos_proxy_exempts(host) {
                return None;
            }
            return Some(intercept.uri().to_string());
        }
    }
    None
}

/// Match a host against one macOS ExceptionsList entry: exact hostname,
/// "*.suffix" wildcard, or IPv4 CIDR.
fn proxy_exempt_matches(host: &str, pattern: &str) -> bool {
    if let Some(rest) = pattern.strip_prefix("*.") {
        return host == rest || host.ends_with(&format!(".{}", rest));
    }
    if let Some((cidr, bits)) = pattern.split_once('/') {
        let (Ok(ip), Ok(bits)) = (host.parse::<std::net::Ipv4Addr>(), bits.parse::<u8>()) else {
            return false;
        };
        if bits > 32 {
            return false;
        }
        let Some(base) = cidr.parse::<std::net::Ipv4Addr>().ok() else {
            return false;
        };
        let mask = if bits == 0 {
            0
        } else {
            u32::MAX << (32 - bits)
        };
        let base_u = u32::from(base) & mask;
        return u32::from(ip) & mask == base_u;
    }
    host == pattern
}

#[cfg(target_os = "macos")]
fn macos_proxy_exempts(host: &str) -> bool {
    use system_configuration::core_foundation::array::CFArray;
    use system_configuration::core_foundation::base::TCFType;
    use system_configuration::core_foundation::string::CFString;
    use system_configuration::dynamic_store::SCDynamicStoreBuilder;
    use system_configuration::sys::schema_definitions::kSCPropNetProxiesExceptionsList;

    let Some(store) = SCDynamicStoreBuilder::new("dockit").build() else {
        return false;
    };
    let Some(proxies) = store.get_proxies() else {
        return false;
    };
    let Some(exceptions) = proxies.find(unsafe { kSCPropNetProxiesExceptionsList }) else {
        return false;
    };
    let Some(list) = exceptions.downcast::<CFArray<*const std::ffi::c_void>>() else {
        return false;
    };
    for i in 0..list.len() {
        let item = unsafe { list.get_unchecked(i) };
        let cfstring = unsafe { CFString::wrap_under_get_rule(*item as *const _) };
        if proxy_exempt_matches(host, &cfstring.to_string()) {
            return true;
        }
    }
    false
}

#[cfg(not(target_os = "macos"))]
fn macos_proxy_exempts(_host: &str) -> bool {
    false
}


/// Detect the system proxy for `host:port` when given, or for an arbitrary
/// probe target otherwise. Used by the UI to offer "use system proxy" on
/// SSH tunnels and to warn when an opted-in proxy no longer applies.
#[tauri::command]
pub async fn detect_system_proxy(
    host: Option<String>,
    port: Option<u16>,
) -> Result<Option<String>, String> {
    match (host, port) {
        (Some(h), Some(p)) => Ok(system_proxy_for(&h, p)),
        _ => Ok(system_proxy_for("proxy-check.invalid", 443)),
    }
}
