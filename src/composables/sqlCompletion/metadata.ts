/**
 * SchemaMetadataService (Layer 2) — 3-level schema cache.
 *
 * Level A (connection-level): databases/schemas/tables — read from the app's
 *   databaseStore via an injected getter (tests inject fakes; the default
 *   reads the real Pinia store lazily).
 * Level B (object-level): column cache — module-local Map filled by
 *   prefetchColumns() (chunked, concurrency-capped) or lazily by getColumns().
 * Level C (derived, v1): document aliases/CTEs → real table names, injected
 *   by the caller (setDerivedTables).
 *
 * Never writes to databaseStore; never persists; never throws on missing data
 * (returns empty). Pure-ish module: the only tauri dependency is `invoke`,
 * isolated so tests can mock it.
 */

import type { ColumnSuggestion } from './types'
import { invoke } from '@tauri-apps/api/core'

export type ColumnInfo = {
  name: string
  data_type: string
  nullable: boolean
  default_value?: string
  is_primary_key: boolean
  is_auto_increment: boolean
  max_length?: number
  precision?: number
  scale?: number
  description?: string
  metadata?: Record<string, string>
}

/** Minimal view of the app store's metadata that this service consumes. */
export type StoreMetadata = {
  databases: { name: string, is_system: boolean }[]
  schemas: Record<string, string[]>
  tables: Record<string, { name: string, schema?: string }[]>
}

export type MetadataGetter = (connectionId: string) => StoreMetadata | null

const DEFAULT_MAX_TABLES = 100
const DEFAULT_CONCURRENCY = 5

/**
 * The Pinia store accessor, registered once by app wiring (main.ts) so this
 * module never statically imports the store — keeps it loadable in jest
 * without Pinia, and browser-safe (no CommonJS `require`).
 */
let storeLoader: (() => typeof import('@/store/databaseStore')['useDatabaseStore']) | null = null

/** Register the database-store accessor (app bootstrap). Null disables (tests). */
export function setDatabaseStoreLoader(
  loader: (() => typeof import('@/store/databaseStore')['useDatabaseStore']) | null,
) {
  storeLoader = loader
}

const DEFAULT_GETTER: MetadataGetter = (connectionId) => {
  const useStore = storeLoader?.()
  if (!useStore)
    return null
  const store = useStore()
  const meta = store.metadata[connectionId]
  if (!meta)
    return null
  return {
    databases: meta.databases,
    schemas: meta.schemas,
    tables: meta.tables,
  }
}

function columnKey(connId: string, database: string, schema: string | undefined, table: string): string {
  return `${connId}|${database}|${schema ?? ''}|${table}`
}

function toColumnSuggestion(col: ColumnInfo): ColumnSuggestion {
  return {
    name: col.name,
    dataType: col.data_type,
    isPrimaryKey: col.is_primary_key,
  }
}

export class SchemaMetadataService {
  private columns = new Map<string, ColumnSuggestion[]>()
  private derivedTables = new Map<string, string>()
  private readonly maxTables: number
  private readonly concurrency: number
  private readonly getMetadata: MetadataGetter

  constructor(opts: {
    getMetadata?: MetadataGetter
    maxTables?: number
    concurrency?: number
  } = {}) {
    this.getMetadata = opts.getMetadata ?? DEFAULT_GETTER
    this.maxTables = opts.maxTables ?? DEFAULT_MAX_TABLES
    this.concurrency = opts.concurrency ?? DEFAULT_CONCURRENCY
  }

  // ── Level A: connection-level objects (from the app store) ──

  getDatabases(connectionId: string): { name: string, isSystem: boolean }[] {
    const meta = this.getMetadata(connectionId)
    if (!meta)
      return []
    return meta.databases.map(db => ({ name: db.name, isSystem: db.is_system }))
  }

  getSchemas(connectionId: string, database: string): string[] {
    const meta = this.getMetadata(connectionId)
    return meta?.schemas[database] ?? []
  }

  getTables(connectionId: string, database: string, schema?: string): string[] {
    const meta = this.getMetadata(connectionId)
    if (!meta)
      return []
    const key = schema ? `${database}.${schema}` : database
    const tables = meta.tables[key]
    if (!tables)
      return []
    return tables.map(t => t.name)
  }

  /** Resolve a document alias/CTE name to a real table name (Level C). */
  resolveDerived(name: string): string | undefined {
    return this.derivedTables.get(name)
  }

  setDerivedTables(entries: Record<string, string>) {
    this.derivedTables = new Map(Object.entries(entries))
  }

  // ── Level B: column cache ──

  getColumns(connectionId: string, database: string, schema: string | undefined, table: string): ColumnSuggestion[] {
    const key = columnKey(connectionId, database, schema, table)
    const cached = this.columns.get(key)
    if (cached)
      return cached
    return []
  }

  /**
   * Prefetch columns for the given tables, chunked at `concurrency` in-flight
   * invokes, capped at `maxTables` tables per call. Never throws: failures
   * leave the affected cache keys absent.
   */
  async prefetchColumns(connectionId: string, database: string, schema: string | undefined, tableNames: string[]): Promise<void> {
    const targets = tableNames.slice(0, this.maxTables)
    for (let i = 0; i < targets.length; i += this.concurrency) {
      const chunk = targets.slice(i, i + this.concurrency)
      await Promise.all(chunk.map(table => this.fetchAndCache(connectionId, database, schema, table)))
    }
  }

  /** Lazily fetch one table's columns on demand (used on cache miss). */
  async fetchTableColumns(connectionId: string, database: string, schema: string | undefined, table: string): Promise<ColumnSuggestion[]> {
    return this.fetchAndCache(connectionId, database, schema, table)
  }

  private async fetchAndCache(
    connectionId: string,
    database: string,
    schema: string | undefined,
    table: string,
  ): Promise<ColumnSuggestion[]> {
    const key = columnKey(connectionId, database, schema, table)
    const cached = this.columns.get(key)
    if (cached)
      return cached

    try {
      const result = await invoke<ColumnInfo[]>('list_columns', {
        connectionId,
        database,
        schema,
        tableName: table,
      })
      const suggestions = (result ?? []).map(toColumnSuggestion)
      this.columns.set(key, suggestions)
      return suggestions
    }
    catch (error) {
      console.error(`[sqlCompletion] failed to load columns for ${database}.${table}:`, error)
      return []
    }
  }

  /** Remove cached columns for one connection (or all when connId omitted). */
  clearColumnCache(connectionId?: string) {
    if (connectionId === undefined) {
      this.columns.clear()
      return
    }
    const prefix = `${connectionId}|`
    for (const key of this.columns.keys()) {
      if (key.startsWith(prefix)) {
        this.columns.delete(key)
      }
    }
  }

  /** Snapshot the column cache for one connection (or all when connId omitted). */
  getCachedColumns(connectionId?: string): Record<string, ColumnSuggestion[]> {
    if (connectionId === undefined) {
      return Object.fromEntries(this.columns)
    }
    const prefix = `${connectionId}|`
    const out: Record<string, ColumnSuggestion[]> = {}
    for (const [key, cols] of this.columns) {
      if (key.startsWith(prefix)) {
        out[key] = cols
      }
    }
    return out
  }

  clearAll() {
    this.columns.clear()
    this.derivedTables.clear()
  }
}

/** Module-level singleton used by the provider and page wiring. */
let instance: SchemaMetadataService | null = null

export function getMetadataService(): SchemaMetadataService {
  if (!instance) {
    instance = new SchemaMetadataService()
  }
  return instance
}

/** Test hook: replace the singleton (e.g. with a fake-getter instance). */
export function setMetadataServiceForTests(service: SchemaMetadataService | null) {
  instance = service
}
