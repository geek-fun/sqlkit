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

export async function saveQueryFile(content: string, filePath?: string, fileName?: string): Promise<SaveResult> {
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

export async function loadQueryFile(filePath: string): Promise<LoadResult> {
  return invoke<LoadResult>('load_query_file', {
    filePath,
  })
}

export async function listSavedQueries(): Promise<string[]> {
  return invoke<string[]>('list_saved_queries')
}

export async function deleteQueryFile(filePath: string): Promise<string> {
  return invoke<string>('delete_query_file', {
    filePath,
  })
}
