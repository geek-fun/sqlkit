import { defineStore } from 'pinia'
import { pureObject } from '../common'
import { storeApi } from '../datasources'

export enum DatabaseType {
  MYSQL = 'MYSQL',
  POSTGRESQL = 'POSTGRESQL',
  MARIADB = 'MARIADB',
  SQLITE = 'SQLITE',
  SQLSERVER = 'SQLSERVER',
}

export interface SSHTunnelConfig {
  enabled: boolean
  host: string
  port: number
  username: string
  authMethod: 'password' | 'privateKey'
  password?: string
  privateKey?: string
}

export interface ServerConnection {
  id?: string
  name: string
  type: DatabaseType
  host: string
  port: number
  username?: string
  password?: string
  database?: string
  ssl: boolean
  sshTunnel?: SSHTunnelConfig
}

export const useConnectionStore = defineStore('connectionStore', {
  state: (): { connections: ServerConnection[] } => ({
    connections: [],
  }),
  getters: {
    connectionOptions(state) {
      return state.connections.map(({ name }) => ({ label: name, value: name }))
    },
    getConnectionById: state => (id: string) => {
      return state.connections.find(c => c.id === id)
    },
    getConnectionByName: state => (name: string) => {
      return state.connections.find(c => c.name === name)
    },
  },
  actions: {
    async fetchConnections() {
      try {
        const fetchedConnections = await storeApi.get<ServerConnection[]>('connections', [])
        this.connections = fetchedConnections
      }
      catch (error) {
        console.error('Failed to fetch connections:', error)
        this.connections = []
      }
    },

    async saveConnection(connection: ServerConnection): Promise<{ success: boolean, message: string }> {
      try {
        const newConnection = {
          ...connection,
          id: connection.id || crypto.randomUUID(),
        }

        if (connection.id) {
          const index = this.connections.findIndex(c => c.id === connection.id)
          if (index !== -1) {
            this.connections[index] = newConnection
          }
        }
        else {
          this.connections.push(newConnection)
        }

        await storeApi.set('connections', pureObject(this.connections))
        return { success: true, message: 'Connection saved successfully' }
      }
      catch (error) {
        return {
          success: false,
          message: error instanceof Error ? error.message : 'Unknown error',
        }
      }
    },

    async removeConnection(connection: ServerConnection) {
      const updatedConnections = this.connections.filter(c => c.id !== connection.id)
      this.connections = updatedConnections
      await storeApi.set('connections', pureObject(updatedConnections))
    },

    async testConnection(_connection: ServerConnection): Promise<boolean> {
      // Implement via Tauri command
      // return await invoke('test_db_connection', { connection });
      return true
    },
  },
})
