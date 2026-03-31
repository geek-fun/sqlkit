import { defineStore } from 'pinia'
import { connectionApi } from '../datasources'
import type { SslConfig } from '@/types/connection'
import { sslModeFromBackend, sslModeToBackend } from '@/types/connection'

export enum DatabaseType {
  MYSQL = 'MYSQL',
  POSTGRESQL = 'POSTGRESQL',
  MARIADB = 'MARIADB',
  SQLITE = 'SQLITE',
  SQLSERVER = 'SQLSERVER',
}

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

const defaultDatabaseFor: Partial<Record<DatabaseType, string>> = {
  [DatabaseType.POSTGRESQL]: 'postgres',
  [DatabaseType.SQLSERVER]: 'master',
}

function resolveDatabase(type: DatabaseType, database?: string): string | null {
  return database || defaultDatabaseFor[type] || null
}

export { resolveDatabase }

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
  ssl: SslConfig
  sshTunnel?: SSHTunnelConfig
  isConnected?: boolean
  lastUsed?: Date
}

interface ConnectionStoreState {
  connections: ServerConnection[]
  activeConnectionId: string | null
  connectionStatus: Record<string, ConnectionStatus>
  currentDatabases: Record<string, string>
}

export const useConnectionStore = defineStore('connectionStore', {
  state: (): ConnectionStoreState => ({
    connections: [],
    activeConnectionId: null,
    connectionStatus: {},
    currentDatabases: {},
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
    /** Returns the currently active database for a connection: backend-resolved default or configured value. */
    getCurrentDatabase: state => (id: string): string =>
      state.currentDatabases[id] ?? state.connections.find(c => c.id === id)?.database ?? '',
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
          ssl: sslModeFromBackend(conn.ssl_mode),
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
          ssl_mode: sslModeToBackend(connection.ssl),
          ssl_ca_cert: connection.ssl.caCertPath || null,
          ssl_client_cert: connection.ssl.clientCertPath || null,
          ssl_client_key: connection.ssl.clientKeyPath || null,
          trust_server_certificate: connection.ssl.trustServerCertificate ?? null,
        }

        await connectionApi.save(serverConfig)

        // Update local state
        const newConnection = { ...connection, id }
        if (connection.id) {
          const index = this.connections.findIndex(c => c.id === connection.id)
          if (index !== -1) {
            this.connections = [
              ...this.connections.slice(0, index),
              newConnection,
              ...this.connections.slice(index + 1),
            ]
          }
        }
        else {
          this.connections = [...this.connections, newConnection]
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
          database: resolveDatabase(connection.type, connection.database),
          ssl_mode: sslModeToBackend(connection.ssl),
          ssl_ca_cert: connection.ssl.caCertPath || null,
          ssl_client_cert: connection.ssl.clientCertPath || null,
          ssl_client_key: connection.ssl.clientKeyPath || null,
          trust_server_certificate: connection.ssl.trustServerCertificate ?? null,
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
          database: resolveDatabase(connection.type, connection.database),
          ssl_mode: sslModeToBackend(connection.ssl),
          ssl_ca_cert: connection.ssl.caCertPath || null,
          ssl_client_cert: connection.ssl.clientCertPath || null,
          ssl_client_key: connection.ssl.clientKeyPath || null,
          trust_server_certificate: connection.ssl.trustServerCertificate ?? null,
        }

        const result = await connectionApi.connect(serverConfig)

        connection.isConnected = true
        connection.lastUsed = new Date()
        this.connectionStatus[connectionId] = ConnectionStatus.CONNECTED
        this.activeConnectionId = connectionId
        // Persist the actual connected database (may be the resolved default).
        const resolvedDb = result.current_database || resolveDatabase(connection.type, connection.database)
        if (resolvedDb) {
          this.currentDatabases[connectionId] = resolvedDb
        }
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
        delete this.currentDatabases[connectionId]
        if (this.activeConnectionId === connectionId) {
          this.activeConnectionId = null
        }
      }
    },

    setActiveConnection(connectionId: string | null) {
      this.activeConnectionId = connectionId
    },

    /** Persist the user-selected database for a connection so it survives navigation. */
    setCurrentDatabase(connectionId: string, database: string) {
      this.currentDatabases[connectionId] = database
    },
  },
})
