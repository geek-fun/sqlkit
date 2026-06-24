import { invoke } from '@tauri-apps/api/core'
import { save as showSaveDialog } from '@tauri-apps/plugin-dialog'

export type SaveResult = {
  success: boolean
  file_path?: string
  message: string
}

export type LoadResult = {
  success: boolean
  content?: string
  message: string
}

export type SavedQueryInfo = {
  file_name: string
  file_path: string
  folder: string
  modified_at: number
  size_bytes: number
}

export function saveQueryFile(content: string, filePath?: string, fileName?: string): Promise<SaveResult> {
  return invoke<SaveResult>('save_query_file', {
    content,
    filePath,
    fileName,
  })
}

export async function saveQueryFileAs(content: string, suggestedName: string = 'query.sql'): Promise<SaveResult | null> {
  const defaultName = suggestedName.endsWith('.sql') ? suggestedName : `${suggestedName}.sql`
  const selectedPath = await showSaveDialog({
    filters: [{ name: 'SQL Files', extensions: ['sql'] }],
    defaultPath: defaultName,
  })

  if (!selectedPath)
    return null

  const filePath = selectedPath.endsWith('.sql') ? selectedPath : `${selectedPath}.sql`
  return invoke<SaveResult>('save_query_file', { content, filePath })
}

export function loadQueryFile(filePath: string): Promise<LoadResult> {
  return invoke<LoadResult>('load_query_file', {
    filePath,
  })
}

export function listSavedQueryFiles(): Promise<SavedQueryInfo[]> {
  return invoke<SavedQueryInfo[]>('list_saved_queries')
}

export function deleteQueryFile(filePath: string): Promise<string> {
  return invoke<string>('delete_query_file', {
    filePath,
  })
}

export type SavedQueryMetadata = {
  connectionId: string | null
  connectionName: string | null
  createdAt: number
  modifiedAt: number
}

export type SavedQueriesMetadata = {
  queries: Record<string, SavedQueryMetadata>
}

export function readSavedQueriesMetadata(): Promise<SavedQueriesMetadata> {
  return invoke<SavedQueriesMetadata>('read_saved_queries_metadata')
}

export function writeSavedQueriesMetadata(metadata: SavedQueriesMetadata): Promise<void> {
  return invoke<void>('write_saved_queries_metadata', { metadata })
}

export function saveQueryMetadata(filePath: string, metadata: SavedQueryMetadata): Promise<void> {
  return invoke<void>('save_query_metadata', { filePath, metadata })
}
