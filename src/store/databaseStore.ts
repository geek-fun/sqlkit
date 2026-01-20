import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { useConnectionStore } from './connectionStore'

export interface TableInfo {
  name: string
  schema?: string
  rowCount?: number
  size?: string
}

export interface DatabaseMetadata {
  databases: string[]
  schemas: Map<string, string[]>
  tables: Map<string, TableInfo[]>
  lastRefresh: Date
}

interface DatabaseStoreState {
  metadata: Map<string, DatabaseMetadata>
  selectedDatabase: string | null
  selectedSchema: string | null
  loading: boolean
}

export const useDatabaseStore = defineStore('databases', {
  state: (): DatabaseStoreState => ({
    metadata: new Map(),
    selectedDatabase: null,
    selectedSchema: null,
    loading: false,
  }),

  getters: {
    currentMetadata(state): DatabaseMetadata | null {
      const connectionStore = useConnectionStore()
      const connId = connectionStore.activeConnectionId
      return connId ? state.metadata.get(connId) ?? null : null
    },

    databases(): string[] {
      return this.currentMetadata?.databases ?? []
    },

    schemas(state): string[] {
      if (!state.selectedDatabase)
        return []
      return this.currentMetadata?.schemas.get(state.selectedDatabase) ?? []
    },

    tables(state): TableInfo[] {
      const key = state.selectedSchema
        ? `${state.selectedDatabase}.${state.selectedSchema}`
        : state.selectedDatabase
      return key ? this.currentMetadata?.tables.get(key) ?? [] : []
    },
  },

  actions: {
    async fetchDatabases(connectionId: string) {
      this.loading = true
      try {
        const result = await invoke<{ databases: { name: string }[] }>('list_databases', {
          connectionId,
        })
        const databases = result.databases.map(db => db.name)

        if (!this.metadata.has(connectionId)) {
          this.metadata.set(connectionId, {
            databases,
            schemas: new Map(),
            tables: new Map(),
            lastRefresh: new Date(),
          })
        }
        else {
          const meta = this.metadata.get(connectionId)!
          meta.databases = databases
          meta.lastRefresh = new Date()
        }
      }
      finally {
        this.loading = false
      }
    },

    async fetchSchemas(connectionId: string, database: string) {
      this.loading = true
      try {
        const result = await invoke<{ schemas: { name: string }[] }>('list_schemas', {
          connectionId,
          database,
        })
        const schemas = result.schemas.map(s => s.name)

        const meta = this.metadata.get(connectionId)
        if (meta) {
          meta.schemas.set(database, schemas)
        }
      }
      finally {
        this.loading = false
      }
    },

    async fetchTables(connectionId: string, database: string, schema?: string) {
      this.loading = true
      try {
        const result = await invoke<{ tables: TableInfo[] }>('list_tables', {
          connectionId,
          database,
          schema,
        })

        const meta = this.metadata.get(connectionId)
        if (meta) {
          const key = schema ? `${database}.${schema}` : database
          meta.tables.set(key, result.tables)
        }
      }
      finally {
        this.loading = false
      }
    },

    clearMetadata(connectionId: string) {
      this.metadata.delete(connectionId)
    },

    selectDatabase(database: string) {
      this.selectedDatabase = database
      this.selectedSchema = null
    },

    selectSchema(schema: string) {
      this.selectedSchema = schema
    },

    resetSelection() {
      this.selectedDatabase = null
      this.selectedSchema = null
    },
  },
})
