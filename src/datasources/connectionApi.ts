import { invoke } from '@tauri-apps/api/core'

export type SshAuthMethod
  = | { method: 'password', password: string }
    | { method: 'privateKey', private_key_path: string, passphrase?: string | null }
    | { method: 'agent' }

// Matches Rust's internally-tagged serde enum:
// #[serde(tag = "type")] → { "type": "ssh", "host": "...", ... } (flattened)
export type TransportLayerConfig = {
  type: 'ssh'
  host: string
  port: number
  username: string
  auth_method: SshAuthMethod
  enabled: boolean
  connect_timeout_secs: number
  keepalive_interval_secs: number
}

export type OracleConnectionOptions = {
  connection_method: 'basic' | 'tns' | 'cloud_wallet'
  sid_or_service?: 'sid' | 'service_name'
  role?: 'NORMAL' | 'SYSDBA' | 'SYSOPER'
  tns_admin_dir?: string
  tns_alias?: string
  wallet_password?: string
  service_level?: 'low' | 'medium' | 'high' | 'tp' | 'tpurgent'
}

export type ServerConfig = {
  id: string
  name: string
  db_type: string
  host: string
  port: number
  username: string
  password?: string | null
  database?: string | null
  ssl_mode?: string | null
  ssl_ca_cert?: string | null
  ssl_client_cert?: string | null
  ssl_client_key?: string | null
  trust_server_certificate?: boolean | null
  transport_layers?: TransportLayerConfig[] | null
  oracle_options?: OracleConnectionOptions | null
}

export type ConnectionStatus = {
  is_connected: boolean
  server_version?: string | null
  current_database?: string | null
  current_user?: string | null
  metadata?: Record<string, string>
}

export const connectionApi = {
  save: async (config: ServerConfig): Promise<string> => {
    return await invoke<string>('save_connection', { config })
  },

  list: async (): Promise<ServerConfig[]> => {
    return await invoke<ServerConfig[]>('list_connections')
  },

  delete: async (id: string): Promise<void> => {
    await invoke('delete_connection', { id })
  },

  test: async (config: ServerConfig): Promise<ConnectionStatus> => {
    return await invoke<ConnectionStatus>('test_connection', { config })
  },

  connect: async (config: ServerConfig): Promise<ConnectionStatus> => {
    return await invoke<ConnectionStatus>('connect_server', { config })
  },

  disconnect: async (id: string): Promise<void> => {
    await invoke('disconnect_server', { id })
  },

  getStatus: async (id: string): Promise<ConnectionStatus> => {
    return await invoke<ConnectionStatus>('get_connection_status', { id })
  },
}
