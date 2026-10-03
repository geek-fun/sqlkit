/**
 * Pure helpers for file-based database connections (SQLite / SQLCipher / DuckDB).
 *
 * Kept side-effect free so the connection form logic stays testable: the Tauri
 * probe command (`probe_database_file`) and the dialogs live in the components.
 */

export type DatabaseFileFormat = 'duckdb' | 'sqlite' | 'unknown'

export type DatabaseFileProbe = {
  path: string
  exists: boolean
  isDirectory: boolean
  sizeBytes: number | null
  parentExists: boolean
  parentWritable: boolean
  writable: boolean
  format: DatabaseFileFormat | null
  /** DuckDB storage-format version read from the file header. */
  storageVersion: number | null
  /** Whether a sibling `<file>.wal` write-ahead log exists. */
  hasWal: boolean
}

export type DatabaseFileStatus
  = | 'memory'
    | 'unknown'
    | 'will-create'
    | 'missing-parent'
    | 'unwritable'
    | 'directory'
    | 'empty'
    | 'ready'
    | 'wrong-format'
    | 'unsupported-path'

export type DatabaseFileAssessment = {
  status: DatabaseFileStatus
  format: DatabaseFileFormat | null
  sizeBytes: number | null
}

export type RecentDatabaseEntry = {
  path: string
  timestamp: number
}

export const DUCKDB_EXTENSION = 'duckdb'
export const SQLITE_EXTENSIONS = ['db', 'sqlite', 'sqlite3', 'sqlcipher', 'db3']
export const DUCKDB_EXTENSIONS = ['duckdb', 'db']
export const DUCKDB_MEMORY_PREFIX = 'memory:'
export const MAX_RECENT_DATABASES = 10

const RECENT_KEYS: Record<'sqlite' | 'duckdb', string> = {
  sqlite: 'sqlite_recent_databases',
  duckdb: 'duckdb_recent_databases',
}

/** localStorage key holding the recent database list for the given engine. */
export const recentDatabasesKey = (engine: 'sqlite' | 'duckdb'): string => RECENT_KEYS[engine]

/**
 * DuckDB in-memory databases are per-connection instances, so the pooled
 * JDBC connections would otherwise each open their own empty database.
 * A named instance (`jdbc:duckdb:memory:<label>`) is shared by every
 * connection that uses the same label.
 */
export function isMemoryDatabaseHost(host: string): boolean {
  return host === ':memory:' || host.startsWith(DUCKDB_MEMORY_PREFIX)
}

export const buildMemoryDatabaseHost = (label: string): string => `${DUCKDB_MEMORY_PREFIX}${label}`

export function memoryDatabaseLabel(host: string): string {
  return isMemoryDatabaseHost(host) ? host.slice(DUCKDB_MEMORY_PREFIX.length) : ''
}

export function createMemoryDatabaseLabel(): string {
  return `sqlkit_${Math.random().toString(36).slice(2, 10)}`
}

/** Append `.<extension>` unless the file name already carries an extension. */
export function withDatabaseExtension(path: string, extension: string): string {
  const trimmed = path.trim()
  if (!trimmed)
    return trimmed

  const separatorIndex = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'))
  const fileName = trimmed.slice(separatorIndex + 1)
  if (fileName.includes('.'))
    return trimmed

  return `${trimmed}.${extension}`
}

/**
 * Path form used for duplicate detection: trailing separators removed and
 * case-folded on case-insensitive file systems (macOS, Windows).
 */
export function normalizeComparablePath(path: string, platform: string): string {
  const collapsed = path.trim().replace(/[/\\]+$/, '')
  const caseInsensitive = platform === 'macos' || platform === 'windows'
  return caseInsensitive ? collapsed.toLowerCase() : collapsed
}

export function isSameDatabaseFile(a: string, b: string, platform: string): boolean {
  return Boolean(a && b) && normalizeComparablePath(a, platform) === normalizeComparablePath(b, platform)
}

/** Human-readable file size, e.g. `1.2 MB`. */
export function formatFileSize(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || Number.isNaN(bytes))
    return ''
  if (bytes < 1024)
    return `${bytes} B`

  const units = ['KB', 'MB', 'GB', 'TB']
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length)
  const value = bytes / 1024 ** exponent
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[exponent - 1]}`
}

/**
 * Oldest DuckDB release that can open a file with the given storage version.
 *
 * Values come from DuckDB's storage-version table; 64 is what `duckdb_jdbc`
 * 1.5.6 writes for a plain file, and the engine itself reports it as
 * `storage_version=v1.0.0+`. Unknown (newer) numbers are surfaced raw so a
 * "this file cannot be opened" report can be diagnosed.
 */
const STORAGE_VERSION_MIN_RELEASE: Record<number, string> = {
  1: 'v0.2.1',
  4: 'v0.2.2',
  6: 'v0.2.3',
  11: 'v0.2.4',
  13: 'v0.2.5',
  15: 'v0.2.6',
  17: 'v0.2.7',
  18: 'v0.2.8',
  21: 'v0.2.9',
  25: 'v0.3.0',
  27: 'v0.3.1',
  31: 'v0.3.2',
  33: 'v0.3.3',
  38: 'v0.5.0',
  39: 'v0.6.0',
  43: 'v0.7.0',
  51: 'v0.8.0',
  64: 'v1.0.0',
  65: 'v1.2.0',
  66: 'v1.3.0',
}

/** Oldest DuckDB release able to read a file with this storage version. */
export function storageVersionMinRelease(storageVersion: number | null): string | null {
  if (storageVersion === null || storageVersion === undefined)
    return null
  return STORAGE_VERSION_MIN_RELEASE[storageVersion] ?? null
}

/**
 * Turn a probe result into the state the form should explain to the user.
 * `expectedFormat` is the engine the connection is being created for.
 */
export function assessDatabaseFile(probe: DatabaseFileProbe | null, expectedFormat: DatabaseFileFormat, host: string): DatabaseFileAssessment {
  if (isMemoryDatabaseHost(host))
    return { status: 'memory', format: null, sizeBytes: null }

  // Verified against duckdb_jdbc 1.5.6: the driver splits the connection URL on
  // ';' before opening the database, so such a path cannot be opened at all.
  if (expectedFormat === 'duckdb' && host.includes(';'))
    return { status: 'unsupported-path', format: null, sizeBytes: null }

  if (!host.trim())
    return { status: 'unknown', format: null, sizeBytes: null }

  if (!probe)
    return { status: 'unknown', format: null, sizeBytes: null }

  if (probe.isDirectory)
    return { status: 'directory', format: null, sizeBytes: probe.sizeBytes }

  if (!probe.exists) {
    if (!probe.parentExists)
      return { status: 'missing-parent', format: null, sizeBytes: null }
    return probe.parentWritable
      ? { status: 'will-create', format: null, sizeBytes: null }
      : { status: 'unwritable', format: null, sizeBytes: null }
  }

  if (!probe.writable)
    return { status: 'unwritable', format: probe.format, sizeBytes: probe.sizeBytes }

  if (probe.sizeBytes === 0)
    return { status: 'empty', format: probe.format, sizeBytes: 0 }

  if (probe.format && probe.format !== expectedFormat)
    return { status: 'wrong-format', format: probe.format, sizeBytes: probe.sizeBytes }

  return { status: 'ready', format: probe.format, sizeBytes: probe.sizeBytes }
}

/** Newest first, de-duplicated by path, capped at `limit`. */
export function mergeRecentDatabase(entries: RecentDatabaseEntry[], path: string, timestamp: number, limit: number = MAX_RECENT_DATABASES): RecentDatabaseEntry[] {
  return [
    { path, timestamp },
    ...entries.filter(entry => entry.path !== path),
  ].slice(0, limit)
}

/** Ensure a file name is offered in the platform save dialog. */
export function suggestDatabaseFileName(label?: string): string {
  const base = (label || 'database').trim().replace(/[/\\:*?"<>|]+/g, '-').replace(/\s+/g, '-')
  return withDatabaseExtension(base || 'database', DUCKDB_EXTENSION)
}
