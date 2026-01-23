import { defineStore } from 'pinia'
import { connectionApi } from '../datasources'

const dbTypeToBackend: Record<DatabaseType, string> = {
  [DatabaseType.POSTGRESQL]: 'PostgreSQL',
  [DatabaseType.MYSQL]: 'MySQL',
  [DatabaseType.MARIADB]: 'MySQL',
  [DatabaseType.SQLITE]: 'SQLite',
  [DatabaseType.SQLSERVER]: 'SqlServer',
}

const dbTypeFromBackend: Record<string, DatabaseType> = {
  PostgreSQL: DatabaseType.POSTGRESQL,
  MySQL: DatabaseType.MYSQL,
  SqlServer: DatabaseType.SQLSERVER,
  SQLite: DatabaseType.SQLITE,
}

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
        const backendConnections = await connectionApi.list()

        this.connections = backendConnections.map(conn => ({
          id: conn.id,
          name: conn.name,
          type: dbTypeFromBackend[conn.db_type] || DatabaseType.POSTGRESQL,
          host: conn.host,
          port: conn.port,
          username: conn.username,
          password: conn.password || undefined,
          database: conn.database || undefined,
          ssl: conn.ssl_mode === 'require',
          isConnected: false,
        }))
      }
      catch (error) {
        console.error('Failed to fetch connections:', error)
        this.connections = []
      }
    },

    async saveConnection(connection: ServerConnection): Promise<{ success: boolean, message: string }> {
      try {
        const id = connection.id || crypto.randomUUID()

        const serverConfig = {
          id,
          name: connection.name,
          db_type: dbTypeToBackend[connection.type] || 'PostgreSQL',
          host: connection.host,
          port: connection.port,
          username: connection.username || '',
          password: connection.password || null,
          database: connection.database || null,
          ssl_mode: connection.ssl ? 'require' : 'disable',
        }

        await connectionApi.save(serverConfig)

        // Update local state
        const newConnection = { ...connection, id }
        if (connection.id) {
          const index = this.connections.findIndex(c => c.id === connection.id)
          if (index !== -1) {
            this.connections[index] = newConnection
          }
        }
        else {
          this.connections.push(newConnection)
        }

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
      if (connection.id) {
        await connectionApi.delete(connection.id)
        this.connections = this.connections.filter(c => c.id !== connection.id)
      }
    },

    async testConnection(connection: ServerConnection): Promise<boolean> {
      try {
        const serverConfig = {
          id: connection.id || crypto.randomUUID(),
          name: connection.name,
          db_type: dbTypeToBackend[connection.type] || 'PostgreSQL',
          host: connection.host,
          port: connection.port,
          username: connection.username || '',
          password: connection.password || null,
          database: connection.database || null,
          ssl_mode: connection.ssl ? 'require' : 'disable',
        }

        const result = await connectionApi.test(serverConfig)
        return result.is_connected
      }
      catch (error) {
        console.error('Connection test failed:', error)
        return false
      }
    },

    async connect(connectionId: string) {
      const connection = this.getConnectionById(connectionId)
      if (!connection) {
        throw new Error(`Connection not found: ${connectionId}`)
      }

      this.connectionStatus[connectionId] = ConnectionStatus.CONNECTING

      try {
        const serverConfig = {
          id: connection.id!,
          name: connection.name,
          db_type: dbTypeToBackend[connection.type] || 'PostgreSQL',
          host: connection.host,
          port: connection.port,
          username: connection.username || '',
          password: connection.password || null,
          database: connection.database || null,
          ssl_mode: connection.ssl ? 'require' : 'disable',
        }

        const result = await connectionApi.connect(serverConfig)

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
        await connectionApi.disconnect(connectionId)
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
