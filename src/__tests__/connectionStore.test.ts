import type { ServerConnection } from '../store/connectionStore'
import { createPinia, setActivePinia } from 'pinia'
import { DatabaseType, useConnectionStore } from '../store/connectionStore'

// Mock the storeApi
jest.mock('../datasources', () => ({
  storeApi: {
    get: jest.fn(),
    set: jest.fn(),
    getSecret: jest.fn(),
    setSecret: jest.fn(),
  },
}))

// eslint-disable-next-line ts/no-require-imports
const { storeApi } = require('../datasources')

describe('connectionStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    jest.clearAllMocks()
  })

  const createMockConnection = (overrides: Partial<ServerConnection> = {}): ServerConnection => ({
    id: crypto.randomUUID(),
    name: 'Test Connection',
    type: DatabaseType.POSTGRESQL,
    host: 'localhost',
    port: 5432,
    username: 'admin',
    password: 'secret',
    database: 'testdb',
    ssl: true,
    ...overrides,
  })

  describe('initial state', () => {
    it('should have empty connections array', () => {
      const store = useConnectionStore()
      expect(store.connections).toEqual([])
    })
  })

  describe('connectionOptions getter', () => {
    it('should return connection options', () => {
      const store = useConnectionStore()
      store.connections = [
        createMockConnection({ name: 'DB1' }),
        createMockConnection({ name: 'DB2' }),
      ]

      expect(store.connectionOptions).toEqual([
        { label: 'DB1', value: 'DB1' },
        { label: 'DB2', value: 'DB2' },
      ])
    })

    it('should return empty array when no connections', () => {
      const store = useConnectionStore()
      expect(store.connectionOptions).toEqual([])
    })
  })

  describe('getConnectionById getter', () => {
    it('should find connection by id', () => {
      const store = useConnectionStore()
      const conn = createMockConnection({ id: 'test-uuid-123' })
      store.connections = [conn]

      expect(store.getConnectionById('test-uuid-123')).toEqual(conn)
    })

    it('should return undefined if connection not found', () => {
      const store = useConnectionStore()
      expect(store.getConnectionById('non-existent-id')).toBeUndefined()
    })
  })

  describe('getConnectionByName getter', () => {
    it('should find connection by name', () => {
      const store = useConnectionStore()
      const conn = createMockConnection({ name: 'MyDB' })
      store.connections = [conn]

      expect(store.getConnectionByName('MyDB')).toEqual(conn)
    })

    it('should return undefined if connection not found', () => {
      const store = useConnectionStore()
      expect(store.getConnectionByName('NonExistent')).toBeUndefined()
    })
  })

  describe('fetchConnections', () => {
    it('should fetch connections from store', async () => {
      const mockConnections = [createMockConnection()]
      storeApi.get.mockResolvedValue(mockConnections)

      const store = useConnectionStore()
      await store.fetchConnections()

      expect(storeApi.get).toHaveBeenCalledWith('connections', [])
      expect(store.connections).toEqual(mockConnections)
    })

    it('should set empty array on error', async () => {
      storeApi.get.mockRejectedValue(new Error('Network error'))

      const store = useConnectionStore()
      await store.fetchConnections()

      expect(store.connections).toEqual([])
    })
  })

  describe('saveConnection', () => {
    it('should add new connection', async () => {
      storeApi.set.mockResolvedValue(undefined)

      const store = useConnectionStore()
      const conn = createMockConnection({ id: undefined })
      const result = await store.saveConnection(conn)

      expect(result.success).toBe(true)
      expect(result.message).toBe('Connection saved successfully')
      expect(store.connections).toHaveLength(1)
      expect(store.connections[0].id).toBeDefined()
      expect(storeApi.set).toHaveBeenCalled()
    })

    it('should update existing connection', async () => {
      storeApi.set.mockResolvedValue(undefined)

      const store = useConnectionStore()
      const existingConn = createMockConnection({ id: 'existing-uuid-123', name: 'Old Name' })
      store.connections = [existingConn]

      const updatedConn = { ...existingConn, name: 'New Name' }
      const result = await store.saveConnection(updatedConn)

      expect(result.success).toBe(true)
      expect(store.connections).toHaveLength(1)
      expect(store.connections[0].name).toBe('New Name')
    })

    it('should return error on failure', async () => {
      storeApi.set.mockRejectedValue(new Error('Save failed'))

      const store = useConnectionStore()
      const conn = createMockConnection()
      const result = await store.saveConnection(conn)

      expect(result.success).toBe(false)
      expect(result.message).toBe('Save failed')
    })
  })

  describe('removeConnection', () => {
    it('should remove connection', async () => {
      storeApi.set.mockResolvedValue(undefined)

      const store = useConnectionStore()
      const conn1 = createMockConnection({ id: 'conn-1' })
      const conn2 = createMockConnection({ id: 'conn-2' })
      store.connections = [conn1, conn2]

      await store.removeConnection(conn1)

      expect(store.connections).toHaveLength(1)
      expect(store.connections[0].id).toBe('conn-2')
      expect(storeApi.set).toHaveBeenCalled()
    })
  })

  describe('testConnection', () => {
    it('should return true (placeholder)', async () => {
      const store = useConnectionStore()
      const conn = createMockConnection()
      const result = await store.testConnection(conn)

      expect(result).toBe(true)
    })
  })
})
