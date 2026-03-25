import { invoke } from '@tauri-apps/api/core'
import { save as showSaveDialog } from '@tauri-apps/plugin-dialog'

export interface SaveResult {
  success: boolean
  file_path?: string
  message: string
}

export interface LoadResult {
  success: boolean
  content?: string
  message: string
}

/**
 * Save a SQL query to a file
 */
export async function saveQueryFile(content: string, filePath?: string, fileName?: string): Promise<SaveResult> {
  return invoke<SaveResult>('save_query_file', {
    content,
    filePath,
    fileName,
  })
}

/**
 * Show OS "Save As" dialog, ensure .sql extension, then write the file.
 * Returns SaveResult, or null if the user cancelled.
 */
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

/**
  Load a SQL query from a file
 */
export async function loadQueryFile(filePath: string): Promise<LoadResult> {
  return invoke<LoadResult>('load_query_file', {
    filePath,
  })
}

/**
 * List all saved SQL query files
 */
export async function listSavedQueries(): Promise<string[]> {
  return invoke<string[]>('list_saved_queries')
}

/**
 * Delete a saved SQL query file
 */
export async function deleteQueryFile(filePath: string): Promise<string> {
  return invoke<string>('delete_query_file', {
    filePath,
  })
}
