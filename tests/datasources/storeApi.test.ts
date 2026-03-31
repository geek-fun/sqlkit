import type { QueryHistoryItem } from '@/datasources/storeApi'
import { storeApi } from '@/datasources/storeApi'

jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}))

const { invoke } = require('@tauri-apps/api/core')

describe('storeApi', () => {
  beforeEach(() => {
    jest.clearAllMocks()
  })

  describe('get', () => {
    it('returns value from invoke', async () => {
      invoke.mockResolvedValue('stored-value')

      const result = await storeApi.get('myKey', 'default')

      expect(invoke).toHaveBeenCalledWith('store_get', { key: 'myKey' })
      expect(result).toBe('stored-value')
    })

    it('returns default value when invoke returns null', async () => {
      invoke.mockResolvedValue(null)

      const result = await storeApi.get('missingKey', { default: true })

      expect(result).toEqual({ default: true })
    })

    it('returns default value when invoke returns undefined', async () => {
      invoke.mockResolvedValue(null)

      const result = await storeApi.get('missingKey', [])

      expect(result).toEqual([])
    })
  })

  describe('set', () => {
    it('calls invoke with key and value', async () => {
      invoke.mockResolvedValue(undefined)

      await storeApi.set('myKey', { data: 'value' })

      expect(invoke).toHaveBeenCalledWith('store_set', { key: 'myKey', value: { data: 'value' } })
    })

    it('handles primitive values', async () => {
      invoke.mockResolvedValue(undefined)

      await storeApi.set('counter', 42)

      expect(invoke).toHaveBeenCalledWith('store_set', { key: 'counter', value: 42 })
    })
  })

  describe('getSecret', () => {
    it('returns encrypted value from invoke', async () => {
      invoke.mockResolvedValue('encrypted-password')

      const result = await storeApi.getSecret('password', 'default-pwd')

      expect(invoke).toHaveBeenCalledWith('store_get', { key: 'password' })
      expect(result).toBe('encrypted-password')
    })

    it('returns default when value is null', async () => {
      invoke.mockResolvedValue(null)

      const result = await storeApi.getSecret('missing', 'fallback')

      expect(result).toBe('fallback')
    })
  })

  describe('setSecret', () => {
    it('stores secret value', async () => {
      invoke.mockResolvedValue(undefined)

      await storeApi.setSecret('apiKey', 'secret-123')

      expect(invoke).toHaveBeenCalledWith('store_set', { key: 'apiKey', value: 'secret-123' })
    })
  })

  describe('saveQueryHistory', () => {
    it('adds new entry to history and limits to 100', async () => {
      invoke.mockResolvedValueOnce([])
      invoke.mockResolvedValueOnce(undefined)

      const entry: QueryHistoryItem = {
        id: 1,
        query: 'SELECT * FROM users',
        database: 'mydb',
        executedAt: '2024-01-01T00:00:00Z',
        executionTime: 50,
        rowCount: 100,
      }

      await storeApi.saveQueryHistory(entry)

      expect(invoke).toHaveBeenCalledWith('store_get', { key: 'queryHistory' })
      expect(invoke).toHaveBeenCalledWith('store_set', {
        key: 'queryHistory',
        value: [entry],
      })
    })

    it('prepends entry to existing history', async () => {
      const existingHistory: QueryHistoryItem[] = [
        { id: 2, query: 'OLD QUERY', database: 'db', executedAt: '2023-01-01' },
      ]
      invoke.mockResolvedValueOnce(existingHistory)
      invoke.mockResolvedValueOnce(undefined)

      const newEntry: QueryHistoryItem = {
        id: 1,
        query: 'NEW QUERY',
        database: 'mydb',
        executedAt: '2024-01-01T00:00:00Z',
      }

      await storeApi.saveQueryHistory(newEntry)

      const setCall = invoke.mock.calls.find(c => c[0] === 'store_set')
      const savedHistory = setCall[1].value
      expect(savedHistory).toHaveLength(2)
      expect(savedHistory[0].query).toBe('NEW QUERY')
      expect(savedHistory[1].query).toBe('OLD QUERY')
    })

    it('truncates history to 100 entries', async () => {
      const existingHistory: QueryHistoryItem[] = Array.from({ length: 150 }, (_, i) => ({
        id: i,
        query: `Query ${i}`,
        database: 'db',
        executedAt: `2024-01-${i + 1}`,
      }))
      invoke.mockResolvedValueOnce(existingHistory)
      invoke.mockResolvedValueOnce(undefined)

      const newEntry: QueryHistoryItem = {
        id: 999,
        query: 'NEW',
        database: 'mydb',
        executedAt: '2024-02-01',
      }

      await storeApi.saveQueryHistory(newEntry)

      const setCall = invoke.mock.calls.find(c => c[0] === 'store_set')
      expect(setCall[1].value.length).toBe(100)
      expect(setCall[1].value[0]).toEqual(newEntry)
    })
  })

  describe('getQueryHistory', () => {
    it('returns query history array', async () => {
      const history: QueryHistoryItem[] = [
        { id: 1, query: 'SELECT 1', database: 'db', executedAt: '2024-01-01' },
      ]
      invoke.mockResolvedValue(history)

      const result = await storeApi.getQueryHistory()

      expect(result).toEqual(history)
    })

    it('returns empty array when no history', async () => {
      invoke.mockResolvedValue(null)

      const result = await storeApi.getQueryHistory()

      expect(result).toEqual([])
    })
  })

  describe('clearQueryHistory', () => {
    it('clears query history', async () => {
      invoke.mockResolvedValue(undefined)

      await storeApi.clearQueryHistory()

      expect(invoke).toHaveBeenCalledWith('store_set', { key: 'queryHistory', value: [] })
    })
  })
})