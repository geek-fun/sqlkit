import { invoke } from '@tauri-apps/api/core'

export type ObjectInfo = {
  name: string
  object_type: string
  schema: string | null
  detail: string | null
}

export type IndexInfo = {
  name: string
  columns: string[]
  index_type: string
  is_unique: boolean
  is_primary: boolean
}

export type ForeignKeyInfo = {
  constraint_name: string
  columns: string[]
  referenced_schema: string | null
  referenced_table: string
  referenced_columns: string[]
  on_update: string | null
  on_delete: string | null
}

export type TriggerInfo = {
  name: string
  action_timing: string
  event: string
  ddl: string | null
}

export const browseApi = {
  listViews: async (connectionId: string, database: string, schema?: string): Promise<ObjectInfo[]> => {
    return await invoke<ObjectInfo[]>('list_views', { connectionId, database, schema })
  },

  listProcedures: async (connectionId: string, database: string, schema?: string): Promise<ObjectInfo[]> => {
    return await invoke<ObjectInfo[]>('list_procedures', { connectionId, database, schema })
  },

  listFunctions: async (connectionId: string, database: string, schema?: string): Promise<ObjectInfo[]> => {
    return await invoke<ObjectInfo[]>('list_functions', { connectionId, database, schema })
  },

  listTriggers: async (connectionId: string, database: string, schema: string | null, table: string): Promise<TriggerInfo[]> => {
    return await invoke<TriggerInfo[]>('list_triggers', { connectionId, database, schema, table })
  },

  listIndexes: async (connectionId: string, database: string, schema: string | null, table: string): Promise<IndexInfo[]> => {
    return await invoke<IndexInfo[]>('list_indexes', { connectionId, database, schema, table })
  },

  listForeignKeys: async (connectionId: string, database: string, schema: string | null, table: string): Promise<ForeignKeyInfo[]> => {
    return await invoke<ForeignKeyInfo[]>('list_foreign_keys', { connectionId, database, schema, table })
  },

  getObjectDdl: async (connectionId: string, database: string, schema: string | null, objectName: string, objectType: string): Promise<string> => {
    return await invoke<string>('get_object_ddl', { connectionId, database, schema, objectName, objectType })
  },

  dropObject: async (connectionId: string, database: string, schema: string | null, objectName: string, objectType: string): Promise<void> => {
    return await invoke<void>('drop_object', { connectionId, database, schema, objectName, objectType })
  },

  renameObject: async (connectionId: string, database: string, schema: string | null, objectName: string, objectType: string, newName: string): Promise<void> => {
    return await invoke<void>('rename_object', { connectionId, database, schema, objectName, objectType, newName })
  },
}
