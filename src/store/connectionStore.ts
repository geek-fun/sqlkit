import type { SslConfig } from '@/types/connection'
import { listen } from '@tauri-apps/api/event'
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
  OCEANBASE_ORACLE = 'OCEANBASE_ORACLE',
  TDSQL = 'TDSQL',
  POLARDB = 'POLARDB',
  DAMENG = 'DAMENG',
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
  XUGUDB = 'XUGUDB',
  GBASE8A = 'GBASE8A',
  TRINO = 'TRINO',
  PRESTO = 'PRESTO',
  DERBY = 'DERBY',
  HIVE = 'HIVE',
  DATABRICKS = 'DATABRICKS',
  HANA = 'HANA',
  TERADATA = 'TERADATA',
  VERTICA = 'VERTICA',
  EXASOL = 'EXASOL',
  BIGQUERY = 'BIGQUERY',
  INFORMIX = 'INFORMIX',
  KYLIN = 'KYLIN',
  CASSANDRA = 'CASSANDRA',
  IRIS = 'IRIS',
  ACCESS = 'ACCESS',
  FIREBIRD = 'FIREBIRD',
  RQLITE = 'RQLITE',
  TURSO = 'TURSO',
  TDENGINE = 'TDENGINE',
  GREENPLUM = 'GREENPLUM',
  ENTERPRISEDB = 'ENTERPRISEDB',
  CRATEDB = 'CRATEDB',
  MATERIALIZE = 'MATERIALIZE',
  ALLOYDB = 'ALLOYDB',
  CLOUDSQLPG = 'CLOUDSQLPG',
  FUJITSUPG = 'FUJITSUPG',
  SINGLESTOREMEMSQL = 'SINGLESTOREMEMSQL',
  CLOUDSQLMYSQL = 'CLOUDSQLMYSQL',
}

const PG_BACKEND = 'PostgreSQL'
const MYSQL_BACKEND = 'MySQL'

export const dbTypeToBackend: Record<DatabaseType, string> = {
  [DatabaseType.POSTGRESQL]: PG_BACKEND,
  [DatabaseType.MYSQL]: MYSQL_BACKEND,
  [DatabaseType.MARIADB]: 'mariadb',
  [DatabaseType.SQLITE]: 'SQLite',
  [DatabaseType.SQLSERVER]: 'SqlServer',
  [DatabaseType.DUCKDB]: 'duckdb',
  [DatabaseType.CLICKHOUSE]: 'clickhouse',
  [DatabaseType.COCKROACHDB]: 'cockroachdb',
  [DatabaseType.REDSHIFT]: 'redshift',
  [DatabaseType.YUGABYTEDB]: 'yugabytedb',
  [DatabaseType.TIMESCALEDB]: 'timescaledb',
  [DatabaseType.KINGBASEES]: 'kingbasees',
  [DatabaseType.GAUSSDB]: 'gaussdb',
  [DatabaseType.HIGHGO]: 'highgo',
  [DatabaseType.UXDB]: 'uxdb',
  [DatabaseType.OPENGAUSS]: 'opengauss',
  [DatabaseType.GBASE8C]: 'gbase8c',
  [DatabaseType.QUESTDB]: 'questdb',
  [DatabaseType.VASTBASE]: 'vastbase',
  [DatabaseType.YASHANDB]: 'yashandb',
  [DatabaseType.TIDB]: 'tidb',
  [DatabaseType.OCEANBASE]: 'oceanbase',
  [DatabaseType.OCEANBASE_ORACLE]: 'oceanbase-oracle',
  [DatabaseType.TDSQL]: 'tdsql',
  [DatabaseType.POLARDB]: 'polardb',
  [DatabaseType.DAMENG]: 'dameng',
  [DatabaseType.DORIS]: 'doris',
  [DatabaseType.SELECTDB]: 'selectdb',
  [DatabaseType.STARROCKS]: 'starrocks',
  [DatabaseType.DATABEND]: 'databend',
  [DatabaseType.GOLDENDB]: 'goldendb',
  [DatabaseType.MANTICORESEARCH]: 'manticore',
  [DatabaseType.ORACLE]: 'oracle',
  [DatabaseType.DB2]: 'db2',
  [DatabaseType.H2]: 'h2',
  [DatabaseType.SNOWFLAKE]: 'snowflake',
  [DatabaseType.XUGUDB]: 'xugudb',
  [DatabaseType.GBASE8A]: 'gbase8a',
  [DatabaseType.TRINO]: 'trino',
  [DatabaseType.PRESTO]: 'presto',
  [DatabaseType.DERBY]: 'derby',
  [DatabaseType.HIVE]: 'hive',
  [DatabaseType.DATABRICKS]: 'databricks',
  [DatabaseType.HANA]: 'hana',
  [DatabaseType.TERADATA]: 'teradata',
  [DatabaseType.VERTICA]: 'vertica',
  [DatabaseType.EXASOL]: 'exasol',
  [DatabaseType.BIGQUERY]: 'bigquery',
  [DatabaseType.INFORMIX]: 'informix',
  [DatabaseType.KYLIN]: 'kylin',
  [DatabaseType.CASSANDRA]: 'cassandra',
  [DatabaseType.IRIS]: 'iris',
  [DatabaseType.ACCESS]: 'access',
  [DatabaseType.FIREBIRD]: 'firebird',
  [DatabaseType.RQLITE]: 'rqlite',
  [DatabaseType.TURSO]: 'turso',
  [DatabaseType.TDENGINE]: 'tdengine',
  [DatabaseType.GREENPLUM]: 'greenplum',
  [DatabaseType.ENTERPRISEDB]: 'enterprisedb',
  [DatabaseType.CRATEDB]: 'cratedb',
  [DatabaseType.MATERIALIZE]: 'materialize',
  [DatabaseType.ALLOYDB]: 'alloydb',
  [DatabaseType.CLOUDSQLPG]: 'cloudsqlpg',
  [DatabaseType.FUJITSUPG]: 'fujitsupg',
  [DatabaseType.SINGLESTOREMEMSQL]: 'singlestore',
  [DatabaseType.CLOUDSQLMYSQL]: 'cloudsqlmysql',
}

export const dbTypeFromBackend: Record<string, DatabaseType> = {
  'PostgreSQL': DatabaseType.POSTGRESQL,
  'MySQL': DatabaseType.MYSQL,
  'mariadb': DatabaseType.MARIADB,
  'SqlServer': DatabaseType.SQLSERVER,
  'SQLite': DatabaseType.SQLITE,
  'duckdb': DatabaseType.DUCKDB,
  'clickhouse': DatabaseType.CLICKHOUSE,
  'cockroachdb': DatabaseType.COCKROACHDB,
  'redshift': DatabaseType.REDSHIFT,
  'yugabytedb': DatabaseType.YUGABYTEDB,
  'timescaledb': DatabaseType.TIMESCALEDB,
  'kingbasees': DatabaseType.KINGBASEES,
  'gaussdb': DatabaseType.GAUSSDB,
  'highgo': DatabaseType.HIGHGO,
  'uxdb': DatabaseType.UXDB,
  'opengauss': DatabaseType.OPENGAUSS,
  'gbase8c': DatabaseType.GBASE8C,
  'questdb': DatabaseType.QUESTDB,
  'vastbase': DatabaseType.VASTBASE,
  'yashandb': DatabaseType.YASHANDB,
  'tidb': DatabaseType.TIDB,
  'oceanbase': DatabaseType.OCEANBASE,
  'oceanbase-oracle': DatabaseType.OCEANBASE_ORACLE,
  'oceanbase_oracle': DatabaseType.OCEANBASE_ORACLE,
  'tdsql': DatabaseType.TDSQL,
  'polardb': DatabaseType.POLARDB,
  'dameng': DatabaseType.DAMENG,
  'doris': DatabaseType.DORIS,
  'selectdb': DatabaseType.SELECTDB,
  'starrocks': DatabaseType.STARROCKS,
  'databend': DatabaseType.DATABEND,
  'goldendb': DatabaseType.GOLDENDB,
  'manticore': DatabaseType.MANTICORESEARCH,
  'oracle': DatabaseType.ORACLE,
  'db2': DatabaseType.DB2,
  'h2': DatabaseType.H2,
  'snowflake': DatabaseType.SNOWFLAKE,
  'xugudb': DatabaseType.XUGUDB,
  'gbase8a': DatabaseType.GBASE8A,
  'trino': DatabaseType.TRINO,
  'presto': DatabaseType.PRESTO,
  'derby': DatabaseType.DERBY,
  'hive': DatabaseType.HIVE,
  'databricks': DatabaseType.DATABRICKS,
  'hana': DatabaseType.HANA,
  'teradata': DatabaseType.TERADATA,
  'vertica': DatabaseType.VERTICA,
  'exasol': DatabaseType.EXASOL,
  'bigquery': DatabaseType.BIGQUERY,
  'informix': DatabaseType.INFORMIX,
  'kylin': DatabaseType.KYLIN,
  'cassandra': DatabaseType.CASSANDRA,
  'iris': DatabaseType.IRIS,
  'access': DatabaseType.ACCESS,
  'firebird': DatabaseType.FIREBIRD,
  'rqlite': DatabaseType.RQLITE,
  'turso': DatabaseType.TURSO,
  'tdengine': DatabaseType.TDENGINE,
  'greenplum': DatabaseType.GREENPLUM,
  'enterprisedb': DatabaseType.ENTERPRISEDB,
  'cratedb': DatabaseType.CRATEDB,
  'materialize': DatabaseType.MATERIALIZE,
  'alloydb': DatabaseType.ALLOYDB,
  'cloudsqlpg': DatabaseType.CLOUDSQLPG,
  'fujitsupg': DatabaseType.FUJITSUPG,
  'singlestore': DatabaseType.SINGLESTOREMEMSQL,
  'cloudsqlmysql': DatabaseType.CLOUDSQLMYSQL,
}

const defaultDatabaseFor: Partial<Record<DatabaseType, string>> = {
  [DatabaseType.POSTGRESQL]: 'postgres',
  [DatabaseType.GBASE8C]: 'postgres',
  [DatabaseType.SQLSERVER]: 'master',
  [DatabaseType.DUCKDB]: ':memory:',
  [DatabaseType.CLICKHOUSE]: 'default',
  [DatabaseType.HIGHGO]: 'highgo',
  [DatabaseType.COCKROACHDB]: 'defaultdb',
  [DatabaseType.YUGABYTEDB]: 'yugabyte',
  [DatabaseType.ALLOYDB]: 'postgres',
  [DatabaseType.CLOUDSQLPG]: 'postgres',
  [DatabaseType.FUJITSUPG]: 'postgres',
  [DatabaseType.GREENPLUM]: 'postgres',
  [DatabaseType.OPENGAUSS]: 'postgres',
  [DatabaseType.GAUSSDB]: 'postgres',
  [DatabaseType.VASTBASE]: 'vastbase',
  [DatabaseType.UXDB]: 'uxdb',
  [DatabaseType.MATERIALIZE]: 'materialize',
  [DatabaseType.CRATEDB]: 'crate',
  [DatabaseType.QUESTDB]: 'qdb',
  [DatabaseType.KINGBASEES]: 'test',
}

const requiredDatabaseFor: Set<DatabaseType> = new Set([
  DatabaseType.ENTERPRISEDB,
  DatabaseType.TIMESCALEDB,
  DatabaseType.REDSHIFT,
])

const databasePlaceholderFor: Partial<Record<DatabaseType, string>> = {
  [DatabaseType.ENTERPRISEDB]: 'edb (Oracle-compat) / postgres (PG mode)',
  [DatabaseType.TIMESCALEDB]: 'tsdb (cloud) / postgres (self-hosted)',
  [DatabaseType.REDSHIFT]: 'database name (required)',
}

function isDatabaseRequired(type: DatabaseType): boolean {
  return requiredDatabaseFor.has(type)
}

function resolveDatabase(type: DatabaseType, database?: string): string | null {
  return database || defaultDatabaseFor[type] || null
}

export { databasePlaceholderFor, isDatabaseRequired, resolveDatabase }

export function formatServerVersion(version: string): string {
  const kingbaseMatch = version.match(/V(\d+)R(\d+)/i)
  if (kingbaseMatch) {
    return `V${Number.parseInt(kingbaseMatch[1], 10)}R${Number.parseInt(kingbaseMatch[2], 10)}`
  }

  const tidbMatch = version.match(/TiDB-v?(\d+\.\d+)/i)
  if (tidbMatch) {
    return `VTiDB ${tidbMatch[1]}`
  }

  const oceanbaseMatch = version.match(/OceanBase\D*(\d+\.\d+\.\d+)/i)
  if (oceanbaseMatch) {
    return `VOceanBase ${oceanbaseMatch[1]}`
  }

  const greenplumMatch = version.match(/Greenplum\s+v?(\d+\.\d+)/i)
  if (greenplumMatch) {
    return `VGreenplum ${greenplumMatch[1]}`
  }

  const openGaussMatch = version.match(/openGauss\s+(\d+\.\d+)/i)
  if (openGaussMatch) {
    return `VOpenGauss ${openGaussMatch[1]}`
  }

  const versionMatch = version.match(/(\d+\.\d+)/)
  if (versionMatch) {
    return `V${versionMatch[1]}`
  }

  return version.length > 20 ? `${version.slice(0, 20)}...` : version
}

export function isJdbcDatabase(type: DatabaseType): boolean {
  return [
    DatabaseType.ORACLE,
    DatabaseType.DB2,
    DatabaseType.H2,
    DatabaseType.SNOWFLAKE,
    DatabaseType.DAMENG,
    DatabaseType.XUGUDB,
    DatabaseType.GBASE8A,
    DatabaseType.DERBY,
    DatabaseType.HIVE,
    DatabaseType.DATABRICKS,
    DatabaseType.HANA,
    DatabaseType.TERADATA,
    DatabaseType.VERTICA,
    DatabaseType.EXASOL,
    DatabaseType.BIGQUERY,
    DatabaseType.INFORMIX,
    DatabaseType.KYLIN,
    DatabaseType.CASSANDRA,
    DatabaseType.IRIS,
    DatabaseType.ACCESS,
    DatabaseType.TDENGINE,
    DatabaseType.DUCKDB,
    DatabaseType.FIREBIRD,
    DatabaseType.YASHANDB,
    DatabaseType.KINGBASEES,
    DatabaseType.OCEANBASE_ORACLE,
  ].includes(type)
}

export const jdbcDatabaseTypes: DatabaseType[] = [
  DatabaseType.ORACLE,
  DatabaseType.DB2,
  DatabaseType.H2,
  DatabaseType.SNOWFLAKE,
  DatabaseType.DAMENG,
  DatabaseType.XUGUDB,
  DatabaseType.GBASE8A,
  DatabaseType.DERBY,
  DatabaseType.HIVE,
  DatabaseType.DATABRICKS,
  DatabaseType.HANA,
  DatabaseType.TERADATA,
  DatabaseType.VERTICA,
  DatabaseType.EXASOL,
  DatabaseType.BIGQUERY,
  DatabaseType.INFORMIX,
  DatabaseType.KYLIN,
  DatabaseType.CASSANDRA,
  DatabaseType.IRIS,
  DatabaseType.ACCESS,
  DatabaseType.TDENGINE,
  DatabaseType.DUCKDB,
  DatabaseType.FIREBIRD,
  DatabaseType.YASHANDB,
  DatabaseType.KINGBASEES,
  DatabaseType.OCEANBASE_ORACLE,
]

export type ConnectionStrategyType = 'native' | 'jdbc-bridge' | 'http-bridge'

export function getConnectionStrategy(type: DatabaseType): ConnectionStrategyType {
  if (isJdbcDatabase(type))
    return 'jdbc-bridge'
  const httpBridgeTypes: DatabaseType[] = [
    DatabaseType.TRINO,
    DatabaseType.PRESTO,
    DatabaseType.RQLITE,
    DatabaseType.TURSO,
  ]
  if (httpBridgeTypes.includes(type))
    return 'http-bridge'
  return 'native'
}

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

export type OracleConnectionOptions = {
  connectionMethod: 'basic' | 'tns' | 'cloud_wallet'
  sidOrService?: 'sid' | 'service_name'
  role?: 'NORMAL' | 'SYSDBA' | 'SYSOPER'
  tnsAdminDir?: string
  tnsAlias?: string
  walletPassword?: string
  serviceLevel?: 'low' | 'medium' | 'high' | 'tp' | 'tpurgent'
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
  oracleOptions?: OracleConnectionOptions
  connectTimeoutSecs?: number
  queryTimeoutSecs?: number
  isConnected?: boolean
  lastUsed?: Date
  serverVersion?: string
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
    const parsed = Number.parseInt(value, 10)
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

function toUndefined<T>(val: T | null | undefined): T | undefined {
  return val ?? undefined
}

export function buildOracleOptions(
  opts: OracleConnectionOptions | undefined,
): import('@/datasources/connectionApi').OracleConnectionOptions | undefined {
  if (!opts)
    return undefined
  return {
    connection_method: opts.connectionMethod,
    sid_or_service: toUndefined(opts.sidOrService),
    role: toUndefined(opts.role),
    tns_admin_dir: toUndefined(opts.tnsAdminDir),
    tns_alias: toUndefined(opts.tnsAlias),
    wallet_password: toUndefined(opts.walletPassword),
    service_level: toUndefined(opts.serviceLevel),
  }
}

function extractOracleOptions(raw: unknown): OracleConnectionOptions | undefined {
  if (!raw || typeof raw !== 'object')
    return undefined
  const item = raw as Record<string, unknown>
  if (!item.connection_method)
    return undefined
  return {
    connectionMethod: item.connection_method as OracleConnectionOptions['connectionMethod'],
    sidOrService: (item.sid_or_service as OracleConnectionOptions['sidOrService']) || undefined,
    role: (item.role as OracleConnectionOptions['role']) || undefined,
    tnsAdminDir: (item.tns_admin_dir as string) || undefined,
    tnsAlias: (item.tns_alias as string) || undefined,
    walletPassword: (item.wallet_password as string) || undefined,
    serviceLevel: (item.service_level as OracleConnectionOptions['serviceLevel']) || undefined,
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
        // The backend doesn't track runtime connection state, so we preserve
        // `isConnected` from the existing store entries.
        const existing = new Map(this.connections.map(c => [c.id, { isConnected: c.isConnected, serverVersion: c.serverVersion }]))
        this.connections = result.map((item: Record<string, unknown>) => {
          const id = item.id as string
          const prev = existing.get(id)
          return {
            id,
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
            oracleOptions: extractOracleOptions(item.oracle_options),
            isConnected: prev?.isConnected ?? (item.is_connected as boolean | undefined),
            lastUsed: item.last_used ? new Date(item.last_used as string) : undefined,
            serverVersion: prev?.serverVersion,
          }
        })
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
          oracle_options: buildOracleOptions(connection.oracleOptions),
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
          connect_timeout_secs: connection.connectTimeoutSecs ?? 10,
          query_timeout_secs: connection.queryTimeoutSecs ?? 30,
          transport_layers: transportLayers,
          oracle_options: buildOracleOptions(connection.oracleOptions),
        }

        const result = await connectionApi.test(serverConfig)
        if (result.server_version) {
          connection.serverVersion = result.server_version
        }
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
          connect_timeout_secs: connection.connectTimeoutSecs ?? 10,
          query_timeout_secs: connection.queryTimeoutSecs ?? 30,
          transport_layers: transportLayers,
          oracle_options: buildOracleOptions(connection.oracleOptions),
        }

        const result = await connectionApi.connect(serverConfig)

        if (result.server_version) {
          connection.serverVersion = result.server_version
        }
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

    async startStateListener() {
      const unlisten = await listen<{
        connection_id: string
        state: 'healthy' | 'degraded' | 'dead' | 'reconnecting'
        error: string | null
      }>('connection-state-changed', (event) => {
        const { connection_id, state, error } = event.payload
        if (state === 'dead') {
          this.connectionStatus[connection_id] = ConnectionStatus.ERROR
        }
        else if (state === 'reconnecting') {
          this.connectionStatus[connection_id] = ConnectionStatus.CONNECTING
        }
        else if (state === 'healthy' || state === 'degraded') {
          const conn = this.getConnectionById(connection_id)
          if (conn) {
            conn.isConnected = true
          }
          this.connectionStatus[connection_id] = ConnectionStatus.CONNECTED
        }
        if (error) {
          console.warn(`[${connection_id}] ${state}: ${error}`)
        }
      })
      return unlisten
    },
  },
})
