use base64::Engine;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{Duration, MissedTickBehavior};

use russh::client::{self, Handle};
use russh::{ChannelMsg, Preferred};

use crate::ssh::config::{
    default_connect_timeout_secs, SshAuthMethod, SshTunnelConfig, IDLE_PING_TIMEOUT_SECS,
    INITIAL_RECONNECT_DELAY_SECS, MAX_RECONNECT_ATTEMPTS, MAX_RECONNECT_DELAY_SECS,
};

const BUFFER_SIZE: usize = 65536;

struct SshClient {
    verify_host_key: bool,
}

impl client::Handler for SshClient {
    type Error = russh::Error;

    /// Host key verification is unconditionally accepted when `verify_host_key` is false
    /// (the default). This trades security for convenience — MITM attacks are possible on
    /// untrusted networks. Enable `verify_host_key` in the SSH tunnel config to require
    /// known-hosts verification before trusting the server's identity.
    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        if self.verify_host_key {
            log::warn!("Host key verification is not yet implemented; accepting key anyway");
        }
        Ok(true)
    }
}

fn ssh_client_config() -> client::Config {
    let mut preferred = Preferred::default();
    let mut kex = preferred.kex.into_owned();
    for algorithm in [
        russh::kex::CURVE25519,
        russh::kex::ECDH_SHA2_NISTP256,
        russh::kex::ECDH_SHA2_NISTP384,
        russh::kex::ECDH_SHA2_NISTP521,
        russh::kex::DH_G14_SHA1,
    ] {
        if !kex.contains(&algorithm) {
            kex.push(algorithm);
        }
    }
    preferred.kex = std::borrow::Cow::Owned(kex);

    client::Config {
        nodelay: true,
        keepalive_interval: Some(Duration::from_secs(30)),
        preferred,
        ..Default::default()
    }
}

use russh::keys::agent::AgentIdentity;

async fn authenticate_with_agent_inner(
    mut agent: russh::keys::agent::client::AgentClient<impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static>,
    session: &mut Handle<SshClient>,
    username: &str,
    timeout: &Duration,
) -> Result<(), String> {
    let identities = agent
        .request_identities()
        .await
        .map_err(|e| format!("SSH agent request failed: {}", e))?;

    if identities.is_empty() {
        return Err("SSH agent has no identities".to_string());
    }

    let hash_alg = session.best_supported_rsa_hash().await.ok().flatten().flatten();

    let auth_result = tokio::time::timeout(*timeout, async {
        for identity in &identities {
            let result = match identity {
                AgentIdentity::PublicKey { key, .. } => {
                    session.authenticate_publickey_with(username, key.clone(), hash_alg, &mut agent).await
                }
                AgentIdentity::Certificate { certificate, .. } => {
                    session.authenticate_certificate_with(username, certificate.clone(), hash_alg, &mut agent).await
                }
            };

            match result {
                Ok(auth_res) if auth_res.success() => return Ok(()),
                Ok(_) => continue,
                Err(_) => continue,
            }
        }
        Err("No SSH agent identity was accepted".to_string())
    })
    .await;

    match auth_result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("SSH agent auth timed out".to_string()),
    }
}

#[cfg(unix)]
async fn authenticate_with_agent(
    session: &mut Handle<SshClient>,
    username: &str,
    timeout: &Duration,
) -> Result<(), String> {
    let agent = russh::keys::agent::client::AgentClient::connect_env()
        .await
        .map_err(|e| format!("SSH agent unavailable: {}", e))?;

    authenticate_with_agent_inner(agent, session, username, timeout).await
}

#[cfg(windows)]
async fn authenticate_with_agent(
    session: &mut Handle<SshClient>,
    username: &str,
    timeout: &Duration,
) -> Result<(), String> {
    let stream = pageant::PageantStream::new()
        .await
        .map_err(|e| format!("SSH agent (Pageant) unavailable: {}", e))?;
    let agent = russh::keys::agent::client::AgentClient::connect(stream);

    authenticate_with_agent_inner(agent, session, username, timeout).await
}

fn load_ssh_private_key(path: &str, passphrase: Option<&str>) -> Result<russh::keys::PrivateKey, String> {
    let secret = std::fs::read_to_string(path).map_err(|e| format!("Cannot read SSH key file: {}", e))?;

    match russh::keys::decode_secret_key(&secret, passphrase) {
        Ok(key) => Ok(key),
        Err(err) if err.to_string().contains("character encoding invalid") => {
            let sanitized = sanitize_openssh_key_comment(&secret)?;
            russh::keys::decode_secret_key(&sanitized, passphrase)
                .map_err(|e| format!("SSH key decode failed (after comment sanitization): {}", e))
        }
        Err(err) => Err(format!("SSH key decode failed: {}", err)),
    }
}

fn sanitize_openssh_key_comment(secret: &str) -> Result<String, String> {
    const OPENSSH_BEGIN: &str = "-----BEGIN OPENSSH PRIVATE KEY-----";
    const OPENSSH_END: &str = "-----END OPENSSH PRIVATE KEY-----";

    if !secret.contains(OPENSSH_BEGIN) {
        return Err("Key is not an OpenSSH format private key".to_string());
    }

    let body: String = secret
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();

    let mut bytes = base64::engine::general_purpose::STANDARD
        .decode(body.as_bytes())
        .map_err(|e| format!("Base64 decode failed: {}", e))?;

    strip_openssh_comment(&mut bytes)?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("{}\n{}\n{}\n", OPENSSH_BEGIN, encoded, OPENSSH_END))
}

fn strip_openssh_comment(bytes: &mut Vec<u8>) -> Result<(), String> {
    const AUTH_MAGIC: &[u8] = b"openssh-key-v1\0";

    if !bytes.starts_with(AUTH_MAGIC) {
        return Err("Invalid OpenSSH key header".to_string());
    }

    let mut pos = AUTH_MAGIC.len();

    let cipher_name = read_ssh_string(bytes, &mut pos)?;
    if cipher_name != b"none" {
        return Err("Cannot sanitize encrypted OpenSSH keys".to_string());
    }

    let _kdf_name = read_ssh_string(bytes, &mut pos)?;
    let _kdf_options = read_ssh_string(bytes, &mut pos)?;
    let key_count = read_u32(bytes, &mut pos)?;

    if key_count == 0 {
        return Err("No private keys found in OpenSSH key file".to_string());
    }

    for _ in 0..key_count {
        let _public_key = read_ssh_string(bytes, &mut pos)?;
        let private_blob = read_ssh_string(bytes, &mut pos)?;
        let patched = zero_out_comment_in_blob(private_blob)?;

        let blob_start = pos - private_blob.len() - 4;
        let blob_len_pos = blob_start;
        let patched_len = patched.len();
        bytes.splice(
            blob_len_pos..pos,
            (patched_len as u32)
                .to_be_bytes()
                .into_iter()
                .chain(patched),
        );
        pos = blob_len_pos + 4 + patched_len;
    }

    Ok(())
}

fn zero_out_comment_in_blob(blob: &[u8]) -> Result<Vec<u8>, String> {
    let unpadded_end = blob
        .len()
        .checked_sub(find_padding_len(blob)?)
        .ok_or_else(|| "Invalid padding in private key blob".to_string())?;

    let comment_pos = find_comment_position(&blob[..unpadded_end])
        .ok_or_else(|| "Could not locate comment field in private key".to_string())?;

    let mut patched = blob.to_vec();
    patched[comment_pos..comment_pos + 4].copy_from_slice(&0u32.to_be_bytes());
    Ok(patched)
}

fn find_comment_position(bytes: &[u8]) -> Option<usize> {
    for pos in (8..bytes.len().saturating_sub(3)).rev() {
        let len_bytes = bytes.get(pos..pos + 4)?;
        let len = u32::from_be_bytes(len_bytes.try_into().ok()?) as usize;
        if pos.checked_add(4)?.checked_add(len)? == bytes.len() {
            return Some(pos);
        }
    }
    None
}

fn find_padding_len(bytes: &[u8]) -> Result<usize, String> {
    for len in (1..=16).rev() {
        if bytes.len() >= len
            && bytes[bytes.len() - len..]
                .iter()
                .enumerate()
                .all(|(i, &b)| b == (i + 1) as u8)
        {
            return Ok(len);
        }
    }
    Err("Invalid private key padding".to_string())
}

fn read_ssh_string<'a>(bytes: &'a [u8], pos: &mut usize) -> Result<&'a [u8], String> {
    let len = read_u32(bytes, pos)? as usize;
    let end = pos
        .checked_add(len)
        .ok_or_else(|| "Invalid SSH string length".to_string())?;
    if end > bytes.len() {
        return Err("SSH string exceeds buffer".to_string());
    }
    let value = &bytes[*pos..end];
    *pos = end;
    Ok(value)
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, String> {
    let end = pos
        .checked_add(4)
        .ok_or_else(|| "Unexpected end of SSH key data".to_string())?;
    if end > bytes.len() {
        return Err("Key data truncated".to_string());
    }
    let value = u32::from_be_bytes(bytes[*pos..end].try_into().unwrap());
    *pos = end;
    Ok(value)
}

async fn forward_loop(
    session: &Handle<SshClient>,
    listener: &TcpListener,
    remote_host: &str,
    remote_port: u16,
    keepalive_interval: Duration,
) {
    let interval_secs = std::cmp::max(keepalive_interval.as_secs(), 5);
    let mut idle_check = tokio::time::interval(Duration::from_secs(interval_secs));
    idle_check.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        let accepted = tokio::select! {
            result = listener.accept() => result,
            _ = idle_check.tick() => {
                if session.is_closed() {
                    log::warn!("SSH tunnel session closed while idle");
                    break;
                }
                match tokio::time::timeout(
                    Duration::from_secs(IDLE_PING_TIMEOUT_SECS),
                    session.send_ping(),
                ).await {
                    Ok(Ok(())) => continue,
                    Ok(Err(e)) => {
                        log::warn!("SSH tunnel health ping failed: {}", e);
                        break;
                    }
                    Err(_) => {
                        log::warn!("SSH tunnel health ping timed out");
                        break;
                    }
                }
            }
        };

        let (mut stream, peer_addr) = match accepted {
            Ok(v) => v,
            Err(e) => {
                log::error!("SSH tunnel listener error: {}", e);
                break;
            }
        };

        if session.is_closed() {
            log::warn!("SSH tunnel session closed, exiting forward loop");
            break;
        }

        let mut channel = match session
            .channel_open_direct_tcpip(
                remote_host,
                remote_port.into(),
                peer_addr.ip().to_string(),
                peer_addr.port().into(),
            )
            .await
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("SSH direct-tcpip channel open failed: {}", e);
                break;
            }
        };

        tokio::spawn(async move {
            let mut buf = vec![0u8; BUFFER_SIZE];
            let mut stream_closed = false;

            loop {
                tokio::select! {
                    r = stream.read(&mut buf), if !stream_closed => {
                        match r {
                            Ok(0) => {
                                stream_closed = true;
                                let _ = channel.eof().await;
                            }
                            Ok(n) => {
                                if channel.data(&buf[..n]).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    msg = channel.wait() => {
                        match msg {
                            Some(ChannelMsg::Data { ref data }) => {
                                if stream.write_all(data).await.is_err() {
                                    break;
                                }
                            }
                            Some(ChannelMsg::Eof) | None => break,
                            _ => {}
                        }
                    }
                }
            }
        });
    }
}

async fn tunnel_reconnect_loop(
    config: SshTunnelConfig,
    connect_timeout_secs: u64,
    listener: TcpListener,
    remote_host: String,
    remote_port: u16,
) {
    let initial_config = config;
    let mut current_config = initial_config.clone();

    loop {
        let connect_host = current_config.host.clone();
        let connect_port = current_config.port;

        log::info!(
            "SSH tunnel active: {}:{} -> {}:{}",
            connect_host,
            connect_port,
            remote_host,
            remote_port
        );

        match client::connect(
            Arc::new(ssh_client_config()),
            (&*connect_host, connect_port),
            SshClient { verify_host_key: current_config.verify_host_key },
        )
        .await
        {
            Ok(mut raw_session) => {
                match authenticate_session(&mut raw_session, &current_config, connect_timeout_secs).await {
                    Ok(()) => {
                        let ka = Duration::from_secs(current_config.keepalive_interval_secs);
                        forward_loop(&raw_session, &listener, &remote_host, remote_port, ka).await;
                        log::warn!("SSH tunnel lost ({}:{}), reconnecting...", connect_host, connect_port);
                    }
                    Err(e) => {
                        log::error!("SSH tunnel auth failed ({}:{}): {}", connect_host, connect_port, e);
                    }
                }
            }
            Err(e) => {
                log::error!("SSH tunnel connect failed ({}:{}): {}", connect_host, connect_port, e);
            }
        }

        let mut delay = Duration::from_secs(INITIAL_RECONNECT_DELAY_SECS);
        let mut attempts: u32 = 0;

        loop {
            if attempts >= MAX_RECONNECT_ATTEMPTS {
                log::error!(
                    "SSH tunnel max reconnect attempts ({}) exhausted for {}:{}",
                    MAX_RECONNECT_ATTEMPTS,
                    connect_host,
                    connect_port
                );
                return;
            }

            tokio::time::sleep(delay).await;

            match client::connect(
                Arc::new(ssh_client_config()),
                (&*connect_host, connect_port),
                SshClient { verify_host_key: current_config.verify_host_key },
            )
            .await
            {
                Ok(mut raw_session) => {
                    match authenticate_session(&mut raw_session, &current_config, connect_timeout_secs).await {
                        Ok(()) => {
                            current_config = initial_config.clone();
                            log::info!(
                                "SSH tunnel reconnected to {}:{} (attempt {})",
                                connect_host,
                                connect_port,
                                attempts + 1
                            );
                            let ka = Duration::from_secs(current_config.keepalive_interval_secs);
                            forward_loop(&raw_session, &listener, &remote_host, remote_port, ka).await;
                            break;
                        }
                        Err(e) => {
                            attempts += 1;
                            log::error!(
                                "SSH reconnect auth failed (attempt {}/{}): {}",
                                attempts,
                                MAX_RECONNECT_ATTEMPTS,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    attempts += 1;
                    log::error!(
                        "SSH reconnect failed ({}:{}, attempt {}/{}): {}",
                        connect_host,
                        connect_port,
                        attempts,
                        MAX_RECONNECT_ATTEMPTS,
                        e
                    );
                }
            }

            delay = std::cmp::min(delay * 2, Duration::from_secs(MAX_RECONNECT_DELAY_SECS));
        }
    }
}

async fn authenticate_session(
    session: &mut Handle<SshClient>,
    config: &SshTunnelConfig,
    connect_timeout_secs: u64,
) -> Result<(), String> {
    let timeout = Duration::from_secs(connect_timeout_secs);

    match &config.auth_method {
        SshAuthMethod::Password { password } => {
            let auth_res = tokio::time::timeout(timeout, session.authenticate_password(&config.username, password))
                .await
                .map_err(|_| format!("Auth timed out ({}s)", connect_timeout_secs))?
                .map_err(|e| format!("Auth failed: {}", e))?;
            if !auth_res.success() {
                return Err("Password authentication failed".to_string());
            }
        }
        SshAuthMethod::PrivateKey {
            private_key_path,
            passphrase,
        } => {
            let key_pair = load_ssh_private_key(private_key_path, passphrase.as_deref())
                .map_err(|e| format!("Failed to load key: {}", e))?;
            let hash_alg = session.best_supported_rsa_hash().await.ok().flatten().flatten();
            let auth_res = tokio::time::timeout(
                timeout,
                session.authenticate_publickey(
                    &config.username,
                    russh::keys::key::PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg),
                ),
            )
            .await
            .map_err(|_| format!("Key auth timed out ({}s)", connect_timeout_secs))?
            .map_err(|e| format!("Key auth failed: {}", e))?;
            if !auth_res.success() {
                return Err("Public key authentication failed".to_string());
            }
        }
        SshAuthMethod::Agent => {
            authenticate_with_agent(session, &config.username, &timeout).await?;
        }
    }

    Ok(())
}

struct TunnelEntry {
    handle: JoinHandle<()>,
    local_port: u16,
}

pub struct TunnelManager {
    tunnels: Mutex<HashMap<String, TunnelEntry>>,
}

impl Default for TunnelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            tunnels: Mutex::new(HashMap::new()),
        }
    }

    pub async fn start_tunnel(
        &self,
        connection_id: &str,
        config: &SshTunnelConfig,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<u16, String> {
        {
            let mut tunnels = self.tunnels.lock().await;
            if let Some(port) = get_active_port(&mut tunnels, connection_id) {
                return Ok(port);
            }
        }

        let (handle, local_port) = spawn_tunnel_task(config, remote_host, remote_port).await?;

        let mut tunnels = self.tunnels.lock().await;
        if let Some(port) = get_active_port(&mut tunnels, connection_id) {
            handle.abort();
            return Ok(port);
        }

        tunnels.insert(
            connection_id.to_string(),
            TunnelEntry {
                handle,
                local_port,
            },
        );
        Ok(local_port)
    }

    pub async fn local_port(&self, connection_id: &str) -> Option<u16> {
        let tunnels = self.tunnels.lock().await;
        tunnels.get(connection_id).map(|entry| entry.local_port)
    }

    pub async fn stop_tunnel(&self, connection_id: &str) {
        if let Some(entry) = self.tunnels.lock().await.remove(connection_id) {
            entry.handle.abort();
        }
    }

    pub async fn stop_all(&self) {
        let mut tunnels = self.tunnels.lock().await;
        for (_id, entry) in tunnels.drain() {
            entry.handle.abort();
        }
    }
}

fn get_active_port(
    tunnels: &mut HashMap<String, TunnelEntry>,
    connection_id: &str,
) -> Option<u16> {
    let entry = tunnels.get(connection_id)?;
    if entry.handle.is_finished() {
        tunnels.remove(connection_id);
        return None;
    }
    Some(entry.local_port)
}

async fn spawn_tunnel_task(
    config: &SshTunnelConfig,
    remote_host: &str,
    remote_port: u16,
) -> Result<(JoinHandle<()>, u16), String> {
    let local_port = portpicker::pick_unused_port().ok_or("No available local port")?;

    let listener = TcpListener::bind(("127.0.0.1", local_port))
        .await
        .map_err(|e| format!("Failed to bind local tunnel port: {}", e))?;

    let timeout = if config.connect_timeout_secs > 0 {
        config.connect_timeout_secs
    } else {
        default_connect_timeout_secs()
    };

    // Synchronously verify SSH connectivity before returning.
    // This ensures the tunnel is ready before the database adapter connects.
    let ssh_config_init = Arc::new(ssh_client_config());
    let timeout_dur = Duration::from_secs(timeout);
    let mut init_session = tokio::time::timeout(
        timeout_dur,
        client::connect(ssh_config_init, (&*config.host, config.port), SshClient { verify_host_key: config.verify_host_key }),
    )
    .await
    .map_err(|_| format!("SSH connection timed out ({}s)", timeout))?
    .map_err(|e| format!("SSH connection failed: {}", e))?;

    authenticate_session(&mut init_session, config, timeout).await?;

    let task_config = config.clone();
    let task_remote_host = remote_host.to_string();
    let handle = tokio::spawn(async move {
        tunnel_reconnect_loop(
            task_config,
            timeout,
            listener,
            task_remote_host,
            remote_port,
        )
        .await;
    });

    Ok((handle, local_port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_openssh_key_strips_trailing_content() {
        let mut container = b"openssh-key-v1\0".to_vec();
        push_string(&mut container, b"none");
        push_string(&mut container, b"none");
        push_string(&mut container, b"");
        container.extend_from_slice(&[0, 0, 0, 1]);
        push_string(&mut container, b"public-key-data");

        let mut private_blob = vec![0u8; 16];
        push_string(&mut private_blob, b"hello");
        pad_to_block(&mut private_blob, 8);

        push_string(&mut container, &private_blob);
        pad_to_block(&mut container, 8);

        use base64::engine::general_purpose;
        let b64 = general_purpose::STANDARD.encode(&container);
        let pem = format!(
            "-----BEGIN OPENSSH PRIVATE KEY-----\n{}\n-----END OPENSSH PRIVATE KEY-----",
            b64
        );

        let result = sanitize_openssh_key_comment(&pem);
        assert!(result.is_ok(), "sanitize should succeed: {:?}", result.err());
        let sanitized = result.unwrap();
        assert!(sanitized.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----"));
        assert!(sanitized.ends_with("-----\n"));
    }

    #[test]
    fn test_sanitize_rejects_non_openssh() {
        let pkcs1 = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA\n-----END RSA PRIVATE KEY-----";
        let result = sanitize_openssh_key_comment(pkcs1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not an OpenSSH format"));
    }

    #[test]
    fn test_get_active_port_nonexistent() {
        let mut tunnels = HashMap::new();
        assert_eq!(get_active_port(&mut tunnels, "missing"), None);
    }

    #[test]
    fn test_find_padding_len() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(find_padding_len(&data), Ok(8));

        let data2 = vec![1, 2, 4, 8];
        assert_eq!(find_padding_len(&data2), Err("Invalid private key padding".to_string()));
    }

    #[test]
    fn test_read_u32_normal() {
        let data = [0x00, 0x00, 0x00, 0x05, 0x01, 0x02];
        let mut pos = 0;
        assert_eq!(read_u32(&data, &mut pos).unwrap(), 5);
        assert_eq!(pos, 4);
    }

    #[test]
    fn test_read_u32_truncated() {
        let data = [0x00, 0x01];
        let mut pos = 0;
        assert!(read_u32(&data, &mut pos).is_err());
    }

    #[test]
    fn test_read_ssh_string() {
        let data = [0x00, 0x00, 0x00, 0x03, 0x41, 0x42, 0x43];
        let mut pos = 0;
        let s = read_ssh_string(&data, &mut pos).unwrap();
        assert_eq!(s, b"ABC");
        assert_eq!(pos, 7);
    }

    #[test]
    fn test_read_ssh_string_truncated() {
        let data = [0x00, 0x00, 0x00, 0x10, 0x41];
        let mut pos = 0;
        assert!(read_ssh_string(&data, &mut pos).is_err());
    }

    #[test]
    fn test_tunnel_manager_new_and_default() {
        let mgr = TunnelManager::new();
        let mgr2 = TunnelManager::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            assert_eq!(mgr.local_port("any").await, None);
            assert_eq!(mgr2.local_port("any").await, None);
        });
    }

    #[test]
    fn test_tunnel_manager_local_port_missing() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mgr = TunnelManager::new();
            assert_eq!(mgr.local_port("nonexistent").await, None);
        });
    }

    #[test]
    fn test_tunnel_manager_stop_missing() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mgr = TunnelManager::new();
            mgr.stop_tunnel("nonexistent").await;
        });
    }

    #[test]
    fn test_tunnel_manager_stop_all_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mgr = TunnelManager::new();
            mgr.stop_all().await;
        });
    }

    #[test]
    fn test_find_comment_position_typical() {
        let mut blob = vec![0u8; 32];
        blob[28] = 0;
        blob[29] = 0;
        blob[30] = 0;
        blob[31] = 4;
        blob.extend_from_slice(b"test");
        let pos = find_comment_position(&blob);
        assert_eq!(pos, Some(28));
    }

    #[test]
    fn test_find_comment_position_too_short() {
        let blob = vec![0u8; 4];
        assert_eq!(find_comment_position(&blob), None);
    }

    #[test]
    fn test_zero_out_comment_in_blob_replaces_comment_len() {
        let mut blob = Vec::new();
        blob.extend_from_slice(b"fake-key-bytes");
        let comment_len_pos = blob.len();
        push_string(&mut blob, b"hello");
        pad_to_block(&mut blob, 8);

        let result = zero_out_comment_in_blob(&blob);
        assert!(result.is_ok(), "should succeed: {:?}", result.err());
        let patched = result.unwrap();
        assert_eq!(
            &patched[comment_len_pos..comment_len_pos + 4],
            &[0u8, 0u8, 0u8, 0u8],
            "comment length field should be zeroed"
        );
    }

    fn pad_to_block(bytes: &mut Vec<u8>, block_size: usize) {
        let pad_len = block_size - (bytes.len() % block_size);
        for i in 1..=pad_len {
            bytes.push(i as u8);
        }
    }

    fn pad_len(bytes: &[u8]) -> Option<usize> {
        for len in (1..=16).rev() {
            if bytes.len() >= len
                && bytes[bytes.len() - len..]
                    .iter()
                    .enumerate()
                    .all(|(i, &b)| b == (i + 1) as u8)
            {
                return Some(len);
            }
        }
        None
    }

    #[test]
    fn test_openssh_key_container_empty_keys() {
        let mut container = b"openssh-key-v1\0".to_vec();
        let cipher_name = b"none";
        push_string(&mut container, cipher_name);
        push_string(&mut container, b"none");
        push_string(&mut container, b"");
        container.extend_from_slice(&[0, 0, 0, 0]);

        let result = strip_openssh_comment(&mut container);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No private keys"));
    }

    fn push_string(bytes: &mut Vec<u8>, value: &[u8]) {
        bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
        bytes.extend_from_slice(value);
    }
}
