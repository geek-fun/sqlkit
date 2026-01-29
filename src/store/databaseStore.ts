import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { useConnectionStore } from './connectionStore'

export interface TableInfo {
  name: string
  schema?: string
  table_type?: string
  rowCount?: number
  size?: string
}

export interface DatabaseMetadata {
  databases: string[]
  schemas: Record<string, string[]>
  tables: Record<string, TableInfo[]>
  lastRefresh: string
}

interface DatabaseStoreState {
  metadata: Record<string, DatabaseMetadata>
  selectedDatabase: string | null
  selectedSchema: string | null
  loading: boolean
}

export const useDatabaseStore = defineStore('databases', {
  state: (): DatabaseStoreState => ({
    metadata: {},
    selectedDatabase: null,
    selectedSchema: null,
    loading: false,
  }),

  getters: {
    currentMetadata(state): DatabaseMetadata | null {
      const connectionStore = useConnectionStore()
      const connId = connectionStore.activeConnectionId
      return connId ? state.metadata[connId] ?? null : null
    },

    databases(): string[] {
      return this.currentMetadata?.databases ?? []
    },

    schemas(state): string[] {
      if (!state.selectedDatabase)
        return []
      return this.currentMetadata?.schemas[state.selectedDatabase] ?? []
    },

    tables(state): TableInfo[] {
      const key = state.selectedSchema
        ? `${state.selectedDatabase}.${state.selectedSchema}`
        : state.selectedDatabase
      return key ? this.currentMetadata?.tables[key] ?? [] : []
    },
  },

  actions: {
    async fetchDatabases(connectionId: string) {
      this.loading = true
      try {
        const result = await invoke<string[]>('list_databases', {
          connectionId,
        })

        if (!result || !Array.isArray(result)) {
          console.error('Invalid response from server', result)
          return
        }

        const databases = result

        if (!this.metadata[connectionId]) {
          this.metadata[connectionId] = {
            databases,
            schemas: {},
            tables: {},
            lastRefresh: new Date().toISOString(),
          }
        }
        else {
          this.metadata[connectionId].databases = databases
          this.metadata[connectionId].lastRefresh = new Date().toISOString()
        }
      }
      catch (error) {
        console.error('Failed to fetch databases:', error)
      }
      finally {
        this.loading = false
      }
    },

    async fetchSchemas(connectionId: string, database: string) {
      this.loading = true
      try {
        const result = await invoke<string[]>('list_schemas', {
          connectionId,
          database,
        })

        if (!result || !Array.isArray(result)) {
          console.error('Invalid response from server', result)
          return
        }

        const schemas = result

        const meta = this.metadata[connectionId]
        if (meta) {
          meta.schemas[database] = schemas
        }
      }
      catch (error) {
        console.error('Failed to fetch schemas:', error)
      }
      finally {
        this.loading = false
      }
    },

    async fetchTables(connectionId: string, database: string, schema?: string) {
      this.loading = true
      try {
        const result = await invoke<TableInfo[]>('list_tables', {
          connectionId,
          database,
          schema,
        })

        if (!result || !Array.isArray(result)) {
          console.error('Invalid response from server', result)
          return
        }

        const meta = this.metadata[connectionId]
        if (meta) {
          const key = schema ? `${database}.${schema}` : database
          meta.tables[key] = result
        }
      }
      catch (error) {
        console.error('Failed to fetch tables:', error)
      }
      finally {
        this.loading = false
      }
    },

    clearMetadata(connectionId: string) {
      delete this.metadata[connectionId]
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
