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

export const saveQueryFile = async (content: string, filePath?: string, fileName?: string): Promise<SaveResult> => {
  return invoke<SaveResult>('save_query_file', {
    content,
    filePath,
    fileName,
  })
}

export const saveQueryFileAs = async (content: string, suggestedName: string = 'query.sql'): Promise<SaveResult | null> => {
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

export const loadQueryFile = async (filePath: string): Promise<LoadResult> => {
  return invoke<LoadResult>('load_query_file', {
    filePath,
  })
}

export const listSavedQueries = async (): Promise<string[]> => {
  return invoke<string[]>('list_saved_queries')
}

export const deleteQueryFile = async (filePath: string): Promise<string> => {
  return invoke<string>('delete_query_file', {
    filePath,
  })
}
