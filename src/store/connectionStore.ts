import type { SslConfig } from '@/types/connection'
import { defineStore } from 'pinia'
import { sslModeFromBackend, sslModeToBackend } from '@/types/connection'
import { connectionApi } from '../datasources'

export enum DatabaseType {
  MYSQL = 'MYSQL',
  POSTGRESQL = 'POSTGRESQL',
  MARIADB = 'MARIADB',
  SQLITE = 'SQLITE',
  SQLSERVER = 'SQLSERVER',
  DUCKDB = 'DUCKDB',
  CLICKHOUSE = 'CLICKHOUSE',
  COCKROACHDB = 'COCKROACHDB',
  REDSHIFT = 'REDSHIFT',
  YUGABYTEDB = 'YUGABYTEDB',
  TIMESCALEDB = 'TIMESCALEDB',
  KINGBASEES = 'KINGBASEES',
  GAUSSDB = 'GAUSSDB',
  HIGHGO = 'HIGHGO',
  UXDB = 'UXDB',
  OPENGAUSS = 'OPENGAUSS',
  GBASE8C = 'GBASE8C',
  QUESTDB = 'QUESTDB',
  VASTBASE = 'VASTBASE',
  YASHANDB = 'YASHANDB',
  TIDB = 'TIDB',
  OCEANBASE = 'OCEANBASE',
  TDSQL = 'TDSQL',
  POLARDB = 'POLARDB',
  DM8 = 'DM8',
  DORIS = 'DORIS',
  SELECTDB = 'SELECTDB',
  STARROCKS = 'STARROCKS',
  DATABEND = 'DATABEND',
  GOLDENDB = 'GOLDENDB',
  MANTICORESEARCH = 'MANTICORESEARCH',
  ORACLE = 'ORACLE',
  DB2 = 'DB2',
  H2 = 'H2',
  SNOWFLAKE = 'SNOWFLAKE',
  DM8ORACLE = 'DM8ORACLE',
  XUGUDB = 'XUGUDB',
  GBASE8A = 'GBASE8A',
  TRINO = 'TRINO',
  PRESTO = 'PRESTO',
  DERBY = 'DERBY',
}

const PG_BACKEND = 'PostgreSQL'
const MYSQL_BACKEND = 'MySQL'

export const dbTypeToBackend: Record<DatabaseType, string> = {
  [DatabaseType.POSTGRESQL]: PG_BACKEND,
  [DatabaseType.MYSQL]: MYSQL_BACKEND,
  [DatabaseType.MARIADB]: MYSQL_BACKEND,
  [DatabaseType.SQLITE]: 'SQLite',
  [DatabaseType.SQLSERVER]: 'SqlServer',
  [DatabaseType.DUCKDB]: 'duckdb',
  [DatabaseType.CLICKHOUSE]: 'clickhouse',
  [DatabaseType.COCKROACHDB]: PG_BACKEND,
  [DatabaseType.REDSHIFT]: PG_BACKEND,
  [DatabaseType.YUGABYTEDB]: PG_BACKEND,
  [DatabaseType.TIMESCALEDB]: PG_BACKEND,
  [DatabaseType.KINGBASEES]: PG_BACKEND,
  [DatabaseType.GAUSSDB]: PG_BACKEND,
  [DatabaseType.HIGHGO]: PG_BACKEND,
  [DatabaseType.UXDB]: PG_BACKEND,
  [DatabaseType.OPENGAUSS]: PG_BACKEND,
  [DatabaseType.GBASE8C]: PG_BACKEND,
  [DatabaseType.QUESTDB]: PG_BACKEND,
  [DatabaseType.VASTBASE]: PG_BACKEND,
  [DatabaseType.YASHANDB]: PG_BACKEND,
  [DatabaseType.TIDB]: MYSQL_BACKEND,
  [DatabaseType.OCEANBASE]: MYSQL_BACKEND,
  [DatabaseType.TDSQL]: MYSQL_BACKEND,
  [DatabaseType.POLARDB]: MYSQL_BACKEND,
  [DatabaseType.DM8]: MYSQL_BACKEND,
  [DatabaseType.DORIS]: MYSQL_BACKEND,
  [DatabaseType.SELECTDB]: MYSQL_BACKEND,
  [DatabaseType.STARROCKS]: MYSQL_BACKEND,
  [DatabaseType.DATABEND]: MYSQL_BACKEND,
  [DatabaseType.GOLDENDB]: MYSQL_BACKEND,
  [DatabaseType.MANTICORESEARCH]: MYSQL_BACKEND,
  [DatabaseType.ORACLE]: 'oracle',
  [DatabaseType.DB2]: 'db2',
  [DatabaseType.H2]: 'h2',
  [DatabaseType.SNOWFLAKE]: 'snowflake',
  [DatabaseType.DM8ORACLE]: 'dm8_oracle',
  [DatabaseType.XUGUDB]: 'xugudb',
  [DatabaseType.GBASE8A]: 'gbase8a',
  [DatabaseType.TRINO]: 'trino',
  [DatabaseType.PRESTO]: 'presto',
  [DatabaseType.DERBY]: 'derby',
}

const dbTypeFromBackend: Record<string, DatabaseType> = {
  PostgreSQL: DatabaseType.POSTGRESQL,
  MySQL: DatabaseType.MYSQL,
  SqlServer: DatabaseType.SQLSERVER,
  SQLite: DatabaseType.SQLITE,
  duckdb: DatabaseType.DUCKDB,
  clickhouse: DatabaseType.CLICKHOUSE,
  oracle: DatabaseType.ORACLE,
  db2: DatabaseType.DB2,
  h2: DatabaseType.H2,
  snowflake: DatabaseType.SNOWFLAKE,
  dm8_oracle: DatabaseType.DM8ORACLE,
  xugudb: DatabaseType.XUGUDB,
  gbase8a: DatabaseType.GBASE8A,
  trino: DatabaseType.TRINO,
  presto: DatabaseType.PRESTO,
  derby: DatabaseType.DERBY,
}

const defaultDatabaseFor: Partial<Record<DatabaseType, string>> = {
  [DatabaseType.POSTGRESQL]: 'postgres',
  [DatabaseType.SQLSERVER]: 'master',
  [DatabaseType.DUCKDB]: ':memory:',
  [DatabaseType.CLICKHOUSE]: 'default',
  [DatabaseType.HIGHGO]: 'highgo',
}

function resolveDatabase(type: DatabaseType, database?: string): string | null {
  return database || defaultDatabaseFor[type] || null
}

export { resolveDatabase }

export function isJdbcDatabase(type: DatabaseType): boolean {
  return [
    DatabaseType.ORACLE,
    DatabaseType.DB2,
    DatabaseType.H2,
    DatabaseType.SNOWFLAKE,
    DatabaseType.DM8ORACLE,
    DatabaseType.XUGUDB,
    DatabaseType.GBASE8A,
    DatabaseType.DERBY,
  ].includes(type)
}

export const jdbcDatabaseTypes: DatabaseType[] = [
  DatabaseType.ORACLE,
  DatabaseType.DB2,
  DatabaseType.H2,
  DatabaseType.SNOWFLAKE,
  DatabaseType.DM8ORACLE,
  DatabaseType.XUGUDB,
  DatabaseType.GBASE8A,
  DatabaseType.DERBY,
]

export enum ConnectionStatus {
  DISCONNECTED = 'disconnected',
  CONNECTING = 'connecting',
  CONNECTED = 'connected',
  ERROR = 'error',
}

export type SshAuthMethodType = 'password' | 'privateKey' | 'agent'

export type SSHTunnelConfig = {
  enabled: boolean
  host: string
  port: number
  username: string
  authMethod: SshAuthMethodType
  password?: string
  privateKey?: string
  privateKeyPassphrase?: string
}

export type ServerConnection = {
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

type ConnectionStoreState = {
  connections: ServerConnection[]
  activeConnectionId: string | null
  connectionStatus: Record<string, ConnectionStatus>
  currentDatabases: Record<string, string>
}

function safeBool(value: unknown, fallback: boolean): boolean {
  return typeof value === 'boolean' ? value : fallback
}

function safeStr(value: unknown, fallback: string): string {
  return typeof value === 'string' ? value : fallback
}

function safePort(value: unknown): number {
  if (typeof value === 'number')
    return value
  if (typeof value === 'string') {
    const parsed = parseInt(value, 10)
    return Number.isNaN(parsed) ? 22 : parsed
  }
  return 22
}

function extractSshTunnelFromTransport(transportLayers: unknown): SSHTunnelConfig | undefined {
  if (!Array.isArray(transportLayers) || transportLayers.length === 0) {
    return undefined
  }

  const sshLayer = transportLayers.find(
    (layer: unknown) => typeof layer === 'object' && layer !== null && (layer as Record<string, unknown>).type === 'ssh',
  ) as Record<string, unknown> | undefined
  if (!sshLayer) {
    return undefined
  }

  const authMethodRaw = sshLayer.auth_method
  const authMethod = typeof authMethodRaw === 'object' && authMethodRaw !== null
    ? (authMethodRaw as Record<string, unknown>)
    : undefined

  if (authMethod && authMethod.method === 'agent') {
    return {
      enabled: safeBool(sshLayer.enabled, true),
      host: safeStr(sshLayer.host, ''),
      port: safePort(sshLayer.port),
      username: safeStr(sshLayer.username, ''),
      authMethod: 'agent',
    }
  }

  let method: 'password' | 'privateKey' = 'password'
  let password: string | undefined
  let privateKey: string | undefined
  let passphrase: string | undefined

  if (authMethod) {
    if (authMethod.method === 'password') {
      method = 'password'
      password = safeStr(authMethod.password, '')
    }
    else if (authMethod.method === 'privateKey') {
      method = 'privateKey'
      privateKey = safeStr(authMethod.private_key_path, '')
      passphrase = typeof authMethod.passphrase === 'string' ? authMethod.passphrase : undefined
    }
  }

  return {
    enabled: safeBool(sshLayer.enabled, true),
    host: safeStr(sshLayer.host, ''),
    port: safePort(sshLayer.port),
    username: safeStr(sshLayer.username, ''),
    authMethod: method,
    password,
    privateKey,
    privateKeyPassphrase: passphrase,
  }
}

export function buildTransportLayers(sshTunnel?: SSHTunnelConfig): import('@/datasources/connectionApi').TransportLayerConfig[] | null {
  if (!sshTunnel?.enabled || !sshTunnel.host) {
    return null
  }

  let authMethod: import('@/datasources/connectionApi').SshAuthMethod

  switch (sshTunnel.authMethod) {
    case 'password':
      authMethod = { method: 'password', password: sshTunnel.password || '' }
      break
    case 'privateKey':
      authMethod = { method: 'privateKey', private_key_path: sshTunnel.privateKey || '', passphrase: sshTunnel.privateKeyPassphrase || null }
      break
    default:
      authMethod = { method: 'agent' }
      break
  }

  return [{
    type: 'ssh',
    host: sshTunnel.host,
    port: sshTunnel.port,
    username: sshTunnel.username,
    auth_method: authMethod,
    enabled: true,
    connect_timeout_secs: 10,
    keepalive_interval_secs: 30,
  }]
}

export const useConnectionStore = defineStore('connectionStore', {
  state: (): ConnectionStoreState => ({
    connections: [],
    activeConnectionId: null,
    connectionStatus: {},
    currentDatabases: {},
  }),

  getters: {
    connectionOptions: (state) => {
      return state.connections.map(conn => ({
        label: conn.name,
        value: conn.name,
      }))
    },

    getConnectionById: (state) => {
      return (id: string) => state.connections.find(conn => conn.id === id)
    },

    getConnectionByName: (state) => {
      return (name: string) => state.connections.find(conn => conn.name === name)
    },

    activeConnection: (state) => {
      return state.connections.find(conn => conn.id === state.activeConnectionId) || undefined
    },

    connectedConnections: (state) => {
      return state.connections.filter(conn => conn.isConnected)
    },

    getConnectionStatus: (state) => {
      return (id: string) => state.connectionStatus[id] || ConnectionStatus.DISCONNECTED
    },

    getCurrentDatabase: (state) => {
      return (connectionId: string) => {
        if (state.currentDatabases[connectionId])
          return state.currentDatabases[connectionId]
        const connection = state.connections.find(c => c.id === connectionId)
        return connection?.database || ''
      }
    },
  },

  actions: {
    async fetchConnections() {
      try {
        const result = await connectionApi.list()
        this.connections = result.map((item: Record<string, unknown>) => ({
          id: item.id as string,
          name: item.name as string,
          type: dbTypeFromBackend[item.db_type as string] || DatabaseType.POSTGRESQL,
          host: item.host as string,
          port: item.port as number,
          username: item.username as string,
          password: item.password as string | undefined,
          database: item.database as string | undefined,
          ssl: {
            mode: sslModeFromBackend(item.ssl_mode as string | null).mode,
            caCertPath: item.ssl_ca_cert as string | undefined,
            clientCertPath: item.ssl_client_cert as string | undefined,
            clientKeyPath: item.ssl_client_key as string | undefined,
            trustServerCertificate: item.trust_server_certificate as boolean | undefined,
          },
          sshTunnel: extractSshTunnelFromTransport(item.transport_layers),
          isConnected: item.is_connected as boolean | undefined,
          lastUsed: item.last_used ? new Date(item.last_used as string) : undefined,
        }))
      }
      catch (error) {
        console.error('Failed to fetch connections:', error)
        this.connections = []
      }
    },

    async saveConnection(connection: ServerConnection): Promise<{ success: boolean, message: string }> {
      try {
        const transportLayers = buildTransportLayers(connection.sshTunnel)

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
          transport_layers: transportLayers,
        }

        const resultId = await connectionApi.save(serverConfig)

        const existingIndex = this.connections.findIndex(c => c.id === resultId)
        if (existingIndex >= 0) {
          this.connections[existingIndex] = { ...connection, id: resultId }
        }
        else {
          this.connections.push({ ...connection, id: resultId })
        }

        return { success: true, message: 'Connection saved successfully' }
      }
      catch (error) {
        console.error('Failed to save connection:', error)
        return { success: false, message: error instanceof Error ? error.message : 'Unknown error' }
      }
    },

    async removeConnection(connection: ServerConnection) {
      if (!connection.id)
        return

      await connectionApi.delete(connection.id)
      this.connections = this.connections.filter(c => c.id !== connection.id)
    },

    async testConnection(connection: ServerConnection): Promise<boolean> {
      try {
        const transportLayers = buildTransportLayers(connection.sshTunnel)

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
          transport_layers: transportLayers,
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
        const transportLayers = buildTransportLayers(connection.sshTunnel)

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
          transport_layers: transportLayers,
        }

        const result = await connectionApi.connect(serverConfig)

        connection.isConnected = true
        connection.lastUsed = new Date()
        this.connectionStatus[connectionId] = ConnectionStatus.CONNECTED
        this.activeConnectionId = connectionId
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

    setCurrentDatabase(connectionId: string, database: string) {
      this.currentDatabases[connectionId] = database
    },
  },
})
