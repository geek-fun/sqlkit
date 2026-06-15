use crate::ssh::config::TransportLayerConfig;
use crate::ssh::TunnelManager;

pub async fn start_transport_layers(
    connection_id: &str,
    layers: &[TransportLayerConfig],
    remote_host: &str,
    remote_port: u16,
    tunnels: &TunnelManager,
) -> Result<Option<u16>, String> {
    if layers.is_empty() {
        return Ok(None);
    }

    let enabled: Vec<&TransportLayerConfig> = layers.iter().filter(|l| l.enabled()).collect();

    if enabled.is_empty() {
        return Ok(None);
    }

    if enabled.len() > 1 {
        return Err(format!(
            "Multi-hop transport chains are not yet supported (got {} layers). \
             Only single-hop SSH tunnels are currently available in this version.",
            enabled.len()
        ));
    }

    match enabled[0] {
        TransportLayerConfig::Ssh(ssh_config) => {
            let local_port = tunnels
                .start_tunnel(connection_id, ssh_config, remote_host, remote_port)
                .await?;
            Ok(Some(local_port))
        }
    }
}

pub async fn stop_transport_layers(
    connection_id: &str,
    tunnels: &TunnelManager,
) {
    tunnels.stop_tunnel(connection_id).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::config::{SshAuthMethod, SshTunnelConfig, TransportLayerConfig};

    fn ssh_config() -> SshTunnelConfig {
        SshTunnelConfig {
            enabled: true,
            host: "test.example.com".to_string(),
            port: 22,
            username: "testuser".to_string(),
            auth_method: SshAuthMethod::Agent,
            connect_timeout_secs: 5,
            keepalive_interval_secs: 30,
        }
    }

    #[tokio::test]
    async fn test_empty_layers_returns_none() {
        let tunnels = TunnelManager::new();
        let result = start_transport_layers("test", &[], "db.example.com", 5432, &tunnels).await;
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_disabled_layers_return_none() {
        let mut config = ssh_config();
        config.enabled = false;
        let layers = vec![TransportLayerConfig::Ssh(config)];
        let tunnels = TunnelManager::new();
        let result = start_transport_layers("test", &layers, "db.example.com", 5432, &tunnels).await;
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_multi_hop_rejected() {
        let layers = vec![
            TransportLayerConfig::Ssh(ssh_config()),
            TransportLayerConfig::Ssh(ssh_config()),
        ];
        let tunnels = TunnelManager::new();
        let result = start_transport_layers("test", &layers, "db.example.com", 5432, &tunnels).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Multi-hop"));
    }

    #[tokio::test]
    async fn test_stop_empty_does_nothing() {
        let tunnels = TunnelManager::new();
        stop_transport_layers("nonexistent", &tunnels).await;
    }
}
