import { invoke } from '@tauri-apps/api/core'

export type ForeignKeyInfo = {
  constraint_name: string | null
  source_schema: string
  source_table: string
  source_column: string
  target_schema: string
  target_table: string
  target_column: string
}

export async function getForeignKeys(connectionId: string, database: string, schema: string | null): Promise<ForeignKeyInfo[]> {
  return invoke<ForeignKeyInfo[]>('get_foreign_keys', {
    connectionId,
    database,
    schema,
  })
}
