import { invoke } from '@tauri-apps/api/core'

export interface QueryHistoryItem {
  id: number
  query: string
  database: string
  executedAt: string
  executionTime?: number
  rowCount?: number
  error?: string
}

export const storeApi = {
  get: async <T>(key: string, defaultValue: T): Promise<T> => {
    const val = await invoke<T | null>('store_get', { key })
    return val ?? defaultValue
  },

  set: async <T>(key: string, value: T) => {
    await invoke('store_set', { key, value })
  },

  getSecret: async <T>(key: string, defaultValue: T) => {
    const encryptedValue = await invoke<T | null>('store_get', { key })
    return encryptedValue ?? defaultValue
  },

  setSecret: async (key: string, value: unknown) => {
    await invoke('store_set', { key, value })
  },

  /**
   * Save a query to the history.
   * @param query - The query history item to save
   */
  saveQueryHistory: async (query: QueryHistoryItem) => {
    const history = await storeApi.get<QueryHistoryItem[]>('queryHistory', [])
    history.unshift(query)
    // Keep last 100 entries
    await storeApi.set('queryHistory', history.slice(0, 100))
  },

  /**
   * Get the query history.
   * @returns The query history items
   */
  getQueryHistory: async (): Promise<QueryHistoryItem[]> => {
    return await storeApi.get<QueryHistoryItem[]>('queryHistory', [])
  },

  /**
   * Clear the query history.
   */
  clearQueryHistory: async () => {
    await storeApi.set('queryHistory', [])
  },
}
