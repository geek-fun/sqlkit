import { LazyStore } from '@tauri-apps/plugin-store'

const store = new LazyStore('.sqlkit.dat')

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
  /**
   * Get a value from the store.
   * @param key - The key to retrieve
   * @param defaultValue - The default value if key doesn't exist
   * @returns The value from the store or the default value
   */
  get: async <T>(key: string, defaultValue: T): Promise<T> => {
    const val = (await store.get(key)) ?? defaultValue
    return val as T
  },

  /**
   * Set a value in the store.
   * @param key - The key to set
   * @param value - The value to store
   */
  set: async <T>(key: string, value: T) => {
    await store.set(key, value)
    await store.save()
  },

  /**
   * Get a sensitive value from the store.
   * @param key - The key to retrieve
   * @param defaultValue - The default value if key doesn't exist
   * @returns The value from the store or the default value
   */
  getSecret: async <T>(key: string, defaultValue: T): Promise<T> => {
    const encryptedValue = (await store.get(key)) ?? defaultValue
    return encryptedValue as T
  },

  /**
   * Set a sensitive value in the store.
   * @param key - The key to set
   * @param value - The value to store
   */
  setSecret: async (key: string, value: unknown) => {
    await store.set(key, value)
    await store.save()
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
