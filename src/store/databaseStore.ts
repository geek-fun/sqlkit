import type { ObjectInfo } from '@/datasources/browseApi'
import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { browseApi } from '@/datasources/browseApi'
import { useConnectionStore } from './connectionStore'

export type TableInfo = {
  name: string
  schema?: string
  table_type?: string
  rowCount?: number
  size?: string
}

export type DatabaseSchema = {
  name: string
  description?: string
  is_system: boolean
}

export type SchemaObjects = {
  views: ObjectInfo[]
  procedures: ObjectInfo[]
  functions: ObjectInfo[]
}

export type DatabaseMetadata = {
  databases: DatabaseSchema[]
  schemas: Record<string, string[]>
  tables: Record<string, TableInfo[]>
  objects: Record<string, SchemaObjects>
  lastRefresh: string
}

type DatabaseStoreState = {
  metadata: Record<string, DatabaseMetadata>
  selectedDatabase: string | null
  selectedSchema: string | null
  loading: boolean
  fetching: Record<string, boolean>
}

export const useDatabaseStore = defineStore('databases', {
  state: (): DatabaseStoreState => ({
    metadata: {},
    selectedDatabase: null,
    selectedSchema: null,
    loading: false,
    fetching: {},
  }),

  getters: {
    currentMetadata(state): DatabaseMetadata | null {
      const connectionStore = useConnectionStore()
      const connId = connectionStore.activeConnectionId
      return connId ? state.metadata[connId] ?? null : null
    },

    databases(): DatabaseSchema[] {
      return this.currentMetadata?.databases ?? []
    },

    userDatabases(): DatabaseSchema[] {
      return this.databases.filter(db => !db.is_system)
    },

    systemDatabases(): DatabaseSchema[] {
      return this.databases.filter(db => db.is_system)
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
        const result = await invoke<DatabaseSchema[]>('list_databases', {
          connectionId,
        })

        if (!result || !Array.isArray(result)) {
          console.error('Invalid response from server', result)
          return
        }

        let databases = result

        if (databases.length === 0) {
          const connectionStore = useConnectionStore()
          const currentDb = connectionStore.getCurrentDatabase(connectionId)
          if (currentDb) {
            databases = [{ name: currentDb, is_system: false }]
          }
          else {
            // JDBC databases (Oracle, Dameng, DB2, etc.) often don't support
            // listing databases. Show a single synthetic entry so the browser
            // tree (via userDatabaseNodes) is non-empty and schemas can be
            // displayed underneath.
            const conn = connectionStore.getConnectionById(connectionId)
            const label = conn?.name || 'Default'
            databases = [{ name: label, is_system: false }]
          }
        }

        if (!this.metadata[connectionId]) {
          this.metadata[connectionId] = {
            databases,
            schemas: {},
            tables: {},
            objects: {},
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

    async fetchSchemaObjects(connectionId: string, database: string, schema: string) {
      const objectKey = `${database}.${schema}`
      if (this.fetching[objectKey]) {
        return
      }
      this.fetching = { ...this.fetching, [objectKey]: true }

      try {
        const [views, procedures, functions] = await Promise.all([
          browseApi.listViews(connectionId, database, schema),
          browseApi.listProcedures(connectionId, database, schema),
          browseApi.listFunctions(connectionId, database, schema),
        ])

        const meta = this.metadata[connectionId]
        if (meta) {
          meta.objects[objectKey] = { views, procedures, functions }
        }
      }
      catch (error) {
        console.error('Failed to fetch schema objects:', error)
      }
      finally {
        this.fetching = { ...this.fetching, [objectKey]: false }
      }
    },

    getSchemaObjects(connectionId: string, database: string, schema: string): SchemaObjects | null {
      return this.metadata[connectionId]?.objects[`${database}.${schema}`] ?? null
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
