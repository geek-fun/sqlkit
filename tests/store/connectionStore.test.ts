import type { ServerConnection } from '@/store/connectionStore'
import { createPinia, setActivePinia } from 'pinia'
import { DatabaseType, useConnectionStore } from '@/store/connectionStore'

jest.mock('@/datasources/connectionApi', () => ({
  connectionApi: {
    list: jest.fn(),
    save: jest.fn(),
    delete: jest.fn(),
    test: jest.fn(),
    connect: jest.fn(),
    disconnect: jest.fn(),
    getStatus: jest.fn(),
  },
}))

// eslint-disable-next-line ts/no-require-imports
const { connectionApi } = require('@/datasources/connectionApi')

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

  const createMockBackendConnection = (overrides = {}) => ({
    id: 'test-uuid-123',
    name: 'Test Connection',
    db_type: 'PostgreSQL',
    host: 'localhost',
    port: 5432,
    username: 'admin',
    password: 'secret',
    database: 'testdb',
    ssl_mode: 'require',
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
    it('should fetch connections from API', async () => {
      const mockBackendConnections = [createMockBackendConnection()]
      connectionApi.list.mockResolvedValue(mockBackendConnections)

      const store = useConnectionStore()
      await store.fetchConnections()

      expect(connectionApi.list).toHaveBeenCalled()
      expect(store.connections).toHaveLength(1)
      expect(store.connections[0].name).toBe('Test Connection')
    })

    it('should set empty array on error', async () => {
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {})
      connectionApi.list.mockRejectedValue(new Error('Network error'))

      const store = useConnectionStore()
      await store.fetchConnections()

      expect(store.connections).toEqual([])
      consoleSpy.mockRestore()
    })
  })

  describe('saveConnection', () => {
    it('should add new connection', async () => {
      connectionApi.save.mockResolvedValue('new-uuid-123')

      const store = useConnectionStore()
      const conn = createMockConnection({ id: undefined })
      const result = await store.saveConnection(conn)

      expect(result.success).toBe(true)
      expect(result.message).toBe('Connection saved successfully')
      expect(store.connections).toHaveLength(1)
      expect(store.connections[0].id).toBeDefined()
      expect(connectionApi.save).toHaveBeenCalled()
    })

    it('should update existing connection', async () => {
      connectionApi.save.mockResolvedValue('existing-uuid-123')

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
      connectionApi.save.mockRejectedValue(new Error('Save failed'))

      const store = useConnectionStore()
      const conn = createMockConnection()
      const result = await store.saveConnection(conn)

      expect(result.success).toBe(false)
      expect(result.message).toBe('Save failed')
    })
  })

  describe('removeConnection', () => {
    it('should remove connection', async () => {
      connectionApi.delete.mockResolvedValue(undefined)

      const store = useConnectionStore()
      const conn1 = createMockConnection({ id: 'conn-1' })
      const conn2 = createMockConnection({ id: 'conn-2' })
      store.connections = [conn1, conn2]

      await store.removeConnection(conn1)

      expect(store.connections).toHaveLength(1)
      expect(store.connections[0].id).toBe('conn-2')
      expect(connectionApi.delete).toHaveBeenCalledWith('conn-1')
    })
  })

  describe('testConnection', () => {
    it('should return true when connection succeeds', async () => {
      connectionApi.test.mockResolvedValue({ is_connected: true })

      const store = useConnectionStore()
      const conn = createMockConnection()
      const result = await store.testConnection(conn)

      expect(result).toBe(true)
      expect(connectionApi.test).toHaveBeenCalled()
    })

    it('should return false on error', async () => {
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {})
      connectionApi.test.mockRejectedValue(new Error('Connection failed'))

      const store = useConnectionStore()
      const conn = createMockConnection()
      const result = await store.testConnection(conn)

      expect(result).toBe(false)
      consoleSpy.mockRestore()
    })
  })
})