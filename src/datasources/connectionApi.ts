import { invoke } from '@tauri-apps/api/core'

export interface ServerConfig {
  id: string
  name: string
  db_type: string
  host: string
  port: number
  username: string
  password?: string | null
  database?: string | null
  ssl_mode?: string | null
}

export interface ConnectionStatus {
  is_connected: boolean
  server_version?: string | null
  current_database?: string | null
  current_user?: string | null
  metadata?: Record<string, string>
}

export const connectionApi = {
  /**
   * Save or update a connection configuration.
   */
  save: async (config: ServerConfig): Promise<string> => {
    return await invoke<string>('save_connection', { config })
  },

  /**
   * Get all saved connections.
   */
  list: async (): Promise<ServerConfig[]> => {
    return await invoke<ServerConfig[]>('list_connections')
  },

  /**
   * Delete a connection configuration.
   */
  delete: async (id: string): Promise<void> => {
    await invoke('delete_connection', { id })
  },

  /**
   * Test a connection without saving.
   */
  test: async (config: ServerConfig): Promise<ConnectionStatus> => {
    return await invoke<ConnectionStatus>('test_connection', { config })
  },

  /**
   * Connect to a server.
   */
  connect: async (config: ServerConfig): Promise<ConnectionStatus> => {
    return await invoke<ConnectionStatus>('connect_server', { config })
  },

  /**
   * Disconnect from a server.
   */
  disconnect: async (id: string): Promise<void> => {
    await invoke('disconnect_server', { id })
  },

  /**
   * Get connection status.
   */
  getStatus: async (id: string): Promise<ConnectionStatus> => {
    return await invoke<ConnectionStatus>('get_connection_status', { id })
  },
}
