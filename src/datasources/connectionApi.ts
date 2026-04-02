import { invoke } from '@tauri-apps/api/core'

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
