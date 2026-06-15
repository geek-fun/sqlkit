use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TransportLayerConfig {
    Ssh(SshTunnelConfig),
}

impl TransportLayerConfig {
    pub fn id(&self) -> &str {
        match self {
            TransportLayerConfig::Ssh(_) => "ssh",
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            TransportLayerConfig::Ssh(layer) => layer.enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshTunnelConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    pub auth_method: SshAuthMethod,
    #[serde(default)]
    pub connect_timeout_secs: u64,
    #[serde(default)]
    pub keepalive_interval_secs: u64,
    #[serde(default)]
    pub verify_host_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "method", rename_all = "camelCase")]
pub enum SshAuthMethod {
    Password {
        password: String,
    },
    PrivateKey {
        private_key_path: String,
        #[serde(default)]
        passphrase: Option<String>,
    },
    Agent,
}

fn default_enabled() -> bool {
    true
}

fn default_ssh_port() -> u16 {
    22
}

pub const fn default_connect_timeout_secs() -> u64 {
    10
}

pub const INITIAL_RECONNECT_DELAY_SECS: u64 = 5;
pub const MAX_RECONNECT_DELAY_SECS: u64 = 60;
pub const MAX_RECONNECT_ATTEMPTS: u32 = 10;
pub const IDLE_PING_TIMEOUT_SECS: u64 = 10;
