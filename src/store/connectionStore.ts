import { invoke } from '@tauri-apps/api/core'
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

export enum ConnectionStatus {
  DISCONNECTED = 'disconnected',
  CONNECTING = 'connecting',
  CONNECTED = 'connected',
  ERROR = 'error',
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
  isConnected?: boolean
  lastUsed?: Date
}

interface ConnectionStoreState {
  connections: ServerConnection[]
  activeConnectionId: string | null
  connectionStatus: Record<string, ConnectionStatus>
}

export const useConnectionStore = defineStore('connectionStore', {
  state: (): ConnectionStoreState => ({
    connections: [],
    activeConnectionId: null,
    connectionStatus: {},
  }),
  getters: {
    activeConnection: (state): ServerConnection | undefined =>
      state.connections.find(c => c.id === state.activeConnectionId),

    connectedConnections: (state): ServerConnection[] =>
      state.connections.filter(c => c.isConnected),

    connectionOptions(state) {
      return state.connections.map(({ name }) => ({ label: name, value: name }))
    },
    getConnectionById: state => (id: string) => {
      return state.connections.find(c => c.id === id)
    },
    getConnectionByName: state => (name: string) => {
      return state.connections.find(c => c.name === name)
    },
    getConnectionStatus: state => (id: string): ConnectionStatus =>
      state.connectionStatus[id] ?? ConnectionStatus.DISCONNECTED,
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

    async connect(connectionId: string) {
      const connection = this.getConnectionById(connectionId)
      if (!connection) {
        throw new Error(`Connection not found: ${connectionId}`)
      }

      this.connectionStatus[connectionId] = ConnectionStatus.CONNECTING

      try {
        const result = await invoke('connect_server', { id: connectionId })
        connection.isConnected = true
        connection.lastUsed = new Date()
        this.connectionStatus[connectionId] = ConnectionStatus.CONNECTED
        this.activeConnectionId = connectionId
        return result
      }
      catch (error) {
        this.connectionStatus[connectionId] = ConnectionStatus.ERROR
        throw new Error(`Failed to connect: ${error}`)
      }
    },

    async disconnect(connectionId: string) {
      try {
        await invoke('disconnect_server', { id: connectionId })
      }
      finally {
        const connection = this.getConnectionById(connectionId)
        if (connection) {
          connection.isConnected = false
        }
        this.connectionStatus[connectionId] = ConnectionStatus.DISCONNECTED
        if (this.activeConnectionId === connectionId) {
          this.activeConnectionId = null
        }
      }
    },

    setActiveConnection(connectionId: string | null) {
      this.activeConnectionId = connectionId
    },
  },
})
