import type { DatabaseSchema } from '@/store/databaseStore'
import { createPinia, setActivePinia } from 'pinia'
import { useConnectionStore } from '@/store/connectionStore'
import { useDatabaseStore } from '@/store/databaseStore'

const mockDb1: DatabaseSchema = { name: 'db1', is_system: false }
const mockDb2: DatabaseSchema = { name: 'db2', is_system: false }
const mockDb3: DatabaseSchema = { name: 'db3', is_system: false }
const mockSystemDb: DatabaseSchema = { name: 'master', is_system: true }

// Mock the Tauri invoke API
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}))

// eslint-disable-next-line ts/no-require-imports
const { invoke } = require('@tauri-apps/api/core')

// Mock the storeApi for connectionStore
jest.mock('@/datasources', () => ({
  storeApi: {
    get: jest.fn().mockResolvedValue([]),
    set: jest.fn(),
    getSecret: jest.fn(),
    setSecret: jest.fn(),
  },
}))

describe('databaseStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    jest.clearAllMocks()
  })

  describe('initial state', () => {
    it('should have correct default values', () => {
      const store = useDatabaseStore()

      expect(Object.keys(store.metadata)).toHaveLength(0)
      expect(store.selectedDatabase).toBeNull()
      expect(store.selectedSchema).toBeNull()
      expect(store.loading).toBe(false)
    })
  })

  describe('selectDatabase', () => {
    it('should select database and reset schema', () => {
      const store = useDatabaseStore()
      store.selectedSchema = 'public'

      store.selectDatabase('mydb')

      expect(store.selectedDatabase).toBe('mydb')
      expect(store.selectedSchema).toBeNull()
    })
  })

  describe('selectSchema', () => {
    it('should select schema', () => {
      const store = useDatabaseStore()

      store.selectSchema('public')

      expect(store.selectedSchema).toBe('public')
    })
  })

  describe('resetSelection', () => {
    it('should reset database and schema selection', () => {
      const store = useDatabaseStore()
      store.selectedDatabase = 'mydb'
      store.selectedSchema = 'public'

      store.resetSelection()

      expect(store.selectedDatabase).toBeNull()
      expect(store.selectedSchema).toBeNull()
    })
  })

  describe('clearMetadata', () => {
    it('should clear metadata for connection', () => {
      const store = useDatabaseStore()
      store.metadata['conn-1'] = {
        databases: [mockDb1],
        schemas: {},
        tables: {},
        lastRefresh: new Date().toISOString(),
      }

      store.clearMetadata('conn-1')

      expect(store.metadata['conn-1']).toBeUndefined()
    })
  })

  describe('fetchDatabases', () => {
    it('should fetch and store databases', async () => {
      invoke.mockResolvedValue([mockDb1, mockDb2])

      const store = useDatabaseStore()
      await store.fetchDatabases('conn-1')

      expect(invoke).toHaveBeenCalledWith('list_databases', { connectionId: 'conn-1' })
      expect(store.metadata['conn-1']?.databases).toEqual([mockDb1, mockDb2])
      expect(store.loading).toBe(false)
    })

    it('should update existing metadata', async () => {
      invoke.mockResolvedValue([mockDb3])

      const store = useDatabaseStore()
      store.metadata['conn-1'] = {
        databases: [mockDb1, mockDb2],
        schemas: { db1: ['public'] },
        tables: {},
        lastRefresh: new Date(2020, 0, 1).toISOString(),
      }

      await store.fetchDatabases('conn-1')

      expect(store.metadata['conn-1']?.databases).toEqual([mockDb3])
      // Should preserve schemas
      expect(store.metadata['conn-1']?.schemas.db1).toEqual(['public'])
    })

    it('should set loading to false even on error', async () => {
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {})
      invoke.mockRejectedValue(new Error('Connection error'))

      const store = useDatabaseStore()
      await store.fetchDatabases('conn-1')

      expect(store.loading).toBe(false)
      consoleSpy.mockRestore()
    })
  })

  describe('fetchSchemas', () => {
    it('should fetch and store schemas', async () => {
      invoke.mockResolvedValue(['public', 'private'])

      const store = useDatabaseStore()
      store.metadata['conn-1'] = {
        databases: [mockDb1],
        schemas: {},
        tables: {},
        lastRefresh: new Date().toISOString(),
      }

      await store.fetchSchemas('conn-1', 'db1')

      expect(invoke).toHaveBeenCalledWith('list_schemas', {
        connectionId: 'conn-1',
        database: 'db1',
      })
      expect(store.metadata['conn-1']?.schemas.db1).toEqual(['public', 'private'])
    })
  })

  describe('fetchTables', () => {
    it('should fetch and store tables', async () => {
      const mockTables = [
        { name: 'users', schema: 'public', rowCount: 100 },
        { name: 'orders', schema: 'public', rowCount: 500 },
      ]
      invoke.mockResolvedValue(mockTables)

      const store = useDatabaseStore()
      store.metadata['conn-1'] = {
        databases: [mockDb1],
        schemas: {},
        tables: {},
        lastRefresh: new Date().toISOString(),
      }

      await store.fetchTables('conn-1', 'db1', 'public')

      expect(invoke).toHaveBeenCalledWith('list_tables', {
        connectionId: 'conn-1',
        database: 'db1',
        schema: 'public',
      })
      expect(store.metadata['conn-1']?.tables['db1.public']).toEqual(mockTables)
    })

    it('should use database only as key when no schema', async () => {
      invoke.mockResolvedValue([{ name: 'users' }])

      const store = useDatabaseStore()
      store.metadata['conn-1'] = {
        databases: [mockDb1],
        schemas: {},
        tables: {},
        lastRefresh: new Date().toISOString(),
      }

      await store.fetchTables('conn-1', 'db1')

      expect(store.metadata['conn-1']?.tables.db1).toEqual([{ name: 'users' }])
    })
  })

  describe('getters', () => {
    it('databases getter should return empty array when no active connection', () => {
      const store = useDatabaseStore()

      expect(store.databases).toEqual([])
    })

    it('databases getter should return databases for active connection', () => {
      const connectionStore = useConnectionStore()
      connectionStore.activeConnectionId = 'conn-1'

      const store = useDatabaseStore()
      store.metadata['conn-1'] = {
        databases: [mockDb1, mockDb2],
        schemas: {},
        tables: {},
        lastRefresh: new Date().toISOString(),
      }

      expect(store.databases).toEqual([mockDb1, mockDb2])
    })

    it('userDatabases getter should filter out system databases', () => {
      const connectionStore = useConnectionStore()
      connectionStore.activeConnectionId = 'conn-1'

      const store = useDatabaseStore()
      store.metadata['conn-1'] = {
        databases: [mockDb1, mockSystemDb, mockDb2],
        schemas: {},
        tables: {},
        lastRefresh: new Date().toISOString(),
      }

      expect(store.userDatabases).toEqual([mockDb1, mockDb2])
    })

    it('systemDatabases getter should return only system databases', () => {
      const connectionStore = useConnectionStore()
      connectionStore.activeConnectionId = 'conn-1'

      const store = useDatabaseStore()
      store.metadata['conn-1'] = {
        databases: [mockDb1, mockSystemDb, mockDb2],
        schemas: {},
        tables: {},
        lastRefresh: new Date().toISOString(),
      }

      expect(store.systemDatabases).toEqual([mockSystemDb])
    })

    it('schemas getter should return empty array when no database selected', () => {
      const store = useDatabaseStore()

      expect(store.schemas).toEqual([])
    })

    it('schemas getter should return schemas for selected database', () => {
      const connectionStore = useConnectionStore()
      connectionStore.activeConnectionId = 'conn-1'

      const store = useDatabaseStore()
      store.selectedDatabase = 'db1'
      store.metadata['conn-1'] = {
        databases: [mockDb1],
        schemas: { db1: ['public', 'private'] },
        tables: {},
        lastRefresh: new Date().toISOString(),
      }

      expect(store.schemas).toEqual(['public', 'private'])
    })

    it('tables getter should return tables for selected database and schema', () => {
      const connectionStore = useConnectionStore()
      connectionStore.activeConnectionId = 'conn-1'

      const store = useDatabaseStore()
      store.selectedDatabase = 'db1'
      store.selectedSchema = 'public'

      const mockTables = [{ name: 'users' }, { name: 'orders' }]
      store.metadata['conn-1'] = {
        databases: [mockDb1],
        schemas: {},
        tables: { 'db1.public': mockTables },
        lastRefresh: new Date().toISOString(),
      }

      expect(store.tables).toEqual(mockTables)
    })
  })
})
