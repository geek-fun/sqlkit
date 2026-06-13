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
  TIDB = 'TIDB',
  OCEANBASE = 'OCEANBASE',
  TDSQL = 'TDSQL',
  POLARDB = 'POLARDB',
  DM8 = 'DM8',
  ORACLE = 'ORACLE',
  DB2 = 'DB2',
  H2 = 'H2',
  SNOWFLAKE = 'SNOWFLAKE',
  DM8ORACLE = 'DM8ORACLE',
  XUGUDB = 'XUGUDB',
  GBASE8A = 'GBASE8A',
  TRINO = 'TRINO',
  PRESTO = 'PRESTO',
}

const PG_BACKEND = 'PostgreSQL'
const MYSQL_BACKEND = 'MySQL'

const dbTypeToBackend: Record<DatabaseType, string> = {
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
  [DatabaseType.TIDB]: MYSQL_BACKEND,
  [DatabaseType.OCEANBASE]: MYSQL_BACKEND,
  [DatabaseType.TDSQL]: MYSQL_BACKEND,
  [DatabaseType.POLARDB]: MYSQL_BACKEND,
  [DatabaseType.DM8]: MYSQL_BACKEND,
  [DatabaseType.ORACLE]: 'oracle',
  [DatabaseType.DB2]: 'db2',
  [DatabaseType.H2]: 'h2',
  [DatabaseType.SNOWFLAKE]: 'snowflake',
  [DatabaseType.DM8ORACLE]: 'dm8_oracle',
  [DatabaseType.XUGUDB]: 'xugudb',
  [DatabaseType.GBASE8A]: 'gbase8a',
  [DatabaseType.TRINO]: 'trino',
  [DatabaseType.PRESTO]: 'presto',
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

export enum ConnectionStatus {
  DISCONNECTED = 'disconnected',
  CONNECTING = 'connecting',
  CONNECTED = 'connected',
  ERROR = 'error',
}

export type SSHTunnelConfig = {
  enabled: boolean
  host: string
  port: number
  username: string
  authMethod: 'password' | 'privateKey'
  password?: string
  privateKey?: string
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

export const useConnectionStore = defineStore('connectionStore', {
  state: (): ConnectionStoreState => ({
