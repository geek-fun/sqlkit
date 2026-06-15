import { invoke } from '@tauri-apps/api/core'

export type ForeignKeyInfo = {
  constraint_name: string
  source_table: string
  columns: string[]
  referenced_schema: string | null
  referenced_table: string
  referenced_columns: string[]
  on_update: string | null
  on_delete: string | null
}

export async function getForeignKeys(connectionId: string, database: string, schema: string | null): Promise<ForeignKeyInfo[]> {
  return invoke<ForeignKeyInfo[]>('get_foreign_keys', {
    connectionId,
    database,
    schema,
  })
}
