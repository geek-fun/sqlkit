import type { ServerConnection } from '@/store/connectionStore'
import { createPinia, setActivePinia } from 'pinia'
import { connectionApi } from '@/datasources/connectionApi'
import { ConnectionStatus, DatabaseType, dbTypeToBackend, resolveDatabase, useConnectionStore } from '@/store/connectionStore'

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

describe('connectionStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    jest.clearAllMocks()
  })

  describe('dbTypeToBackend', () => {
    it('maps all DatabaseType variants to non-empty strings', () => {
      const allTypes = Object.values(DatabaseType)
      expect(allTypes.length).toBeGreaterThan(0)
      for (const type of allTypes) {
        const backend = dbTypeToBackend[type as DatabaseType]
        expect(backend).toBeDefined()
        expect(backend.length).toBeGreaterThan(0)
      }
    })

    it('maps each type to its own unique backend string (preserves round-trip)', () => {
      expect(dbTypeToBackend[DatabaseType.COCKROACHDB]).toBe('cockroachdb')
      expect(dbTypeToBackend[DatabaseType.REDSHIFT]).toBe('redshift')
      expect(dbTypeToBackend[DatabaseType.YUGABYTEDB]).toBe('yugabytedb')
      expect(dbTypeToBackend[DatabaseType.TIMESCALEDB]).toBe('timescaledb')
      expect(dbTypeToBackend[DatabaseType.KINGBASEES]).toBe('kingbasees')
      expect(dbTypeToBackend[DatabaseType.GAUSSDB]).toBe('gaussdb')
      expect(dbTypeToBackend[DatabaseType.HIGHGO]).toBe('highgo')
      expect(dbTypeToBackend[DatabaseType.UXDB]).toBe('uxdb')
      expect(dbTypeToBackend[DatabaseType.OPENGAUSS]).toBe('opengauss')
      expect(dbTypeToBackend[DatabaseType.GBASE8C]).toBe('gbase8c')
    })

    it('maps each MySQL-family type to its own unique backend string', () => {
      expect(dbTypeToBackend[DatabaseType.MARIADB]).toBe('mariadb')
      expect(dbTypeToBackend[DatabaseType.TIDB]).toBe('tidb')
      expect(dbTypeToBackend[DatabaseType.OCEANBASE]).toBe('oceanbase')
      expect(dbTypeToBackend[DatabaseType.TDSQL]).toBe('tdsql')
      expect(dbTypeToBackend[DatabaseType.POLARDB]).toBe('polardb')
      expect(dbTypeToBackend[DatabaseType.DAMENG]).toBe('dameng')
    })

    it('maps standalone native types correctly', () => {
      expect(dbTypeToBackend[DatabaseType.POSTGRESQL]).toBe('PostgreSQL')
      expect(dbTypeToBackend[DatabaseType.MYSQL]).toBe('MySQL')
      expect(dbTypeToBackend[DatabaseType.SQLSERVER]).toBe('SqlServer')
      expect(dbTypeToBackend[DatabaseType.SQLITE]).toBe('SQLite')
      expect(dbTypeToBackend[DatabaseType.DUCKDB]).toBe('duckdb')
      expect(dbTypeToBackend[DatabaseType.CLICKHOUSE]).toBe('clickhouse')
      expect(dbTypeToBackend[DatabaseType.ORACLE]).toBe('oracle')
    })

    it('maps JDBC bridge types correctly', () => {
      expect(dbTypeToBackend[DatabaseType.DB2]).toBe('db2')
      expect(dbTypeToBackend[DatabaseType.H2]).toBe('h2')
      expect(dbTypeToBackend[DatabaseType.SNOWFLAKE]).toBe('snowflake')
      expect(dbTypeToBackend[DatabaseType.DAMENG]).toBe('dameng')
      expect(dbTypeToBackend[DatabaseType.XUGUDB]).toBe('xugudb')
      expect(dbTypeToBackend[DatabaseType.GBASE8A]).toBe('gbase8a')
    })

    it('maps HTTP bridge types correctly', () => {
      expect(dbTypeToBackend[DatabaseType.TRINO]).toBe('trino')
      expect(dbTypeToBackend[DatabaseType.PRESTO]).toBe('presto')
    })
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

  describe('resolveDatabase', () => {
    it('returns provided database', () => {
      expect(resolveDatabase(DatabaseType.POSTGRESQL, 'mydb')).toBe('mydb')
    })

    it('returns default for PostgreSQL when no database provided', () => {
      expect(resolveDatabase(DatabaseType.POSTGRESQL)).toBe('postgres')
    })

    it('returns default for SQLServer when no database provided', () => {
      expect(resolveDatabase(DatabaseType.SQLSERVER)).toBe('master')
    })

    it('returns null for MySQL when no database provided', () => {
      expect(resolveDatabase(DatabaseType.MYSQL)).toBe(null)
    })

    it('returns null for SQLite when no database provided', () => {
      expect(resolveDatabase(DatabaseType.SQLITE)).toBe(null)
    })
  })

  describe('initial state', () => {
    it('should have empty connections array', () => {
      const store = useConnectionStore()
      expect(store.connections).toEqual([])
      expect(store.activeConnectionId).toBeNull()
      expect(store.connectionStatus).toEqual({})
      expect(store.currentDatabases).toEqual({})
    })
  })

  describe('getters', () => {
    describe('connectionOptions', () => {
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

    describe('getConnectionById', () => {
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

    describe('getConnectionByName', () => {
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

    describe('activeConnection', () => {
      it('returns active connection', () => {
        const store = useConnectionStore()
        store.connections = [
          { id: 'conn-1', name: 'DB1', type: DatabaseType.POSTGRESQL, host: 'localhost', port: 5432, ssl: true },
          { id: 'conn-2', name: 'DB2', type: DatabaseType.MYSQL, host: 'localhost', port: 3306, ssl: false },
        ]
        store.activeConnectionId = 'conn-1'

        expect(store.activeConnection?.name).toBe('DB1')
      })

      it('returns undefined when no active connection', () => {
        const store = useConnectionStore()
        expect(store.activeConnection).toBeUndefined()
      })
    })

    describe('connectedConnections', () => {
      it('returns only connected connections', () => {
        const store = useConnectionStore()
        store.connections = [
          { id: 'conn-1', name: 'DB1', type: DatabaseType.POSTGRESQL, host: 'localhost', port: 5432, ssl: true, isConnected: true },
          { id: 'conn-2', name: 'DB2', type: DatabaseType.MYSQL, host: 'localhost', port: 3306, ssl: false, isConnected: false },
          { id: 'conn-3', name: 'DB3', type: DatabaseType.SQLITE, host: '', port: 0, ssl: false, isConnected: true },
        ]

        expect(store.connectedConnections).toHaveLength(2)
        expect(store.connectedConnections.map(c => c.name)).toEqual(['DB1', 'DB3'])
      })
    })

    describe('getConnectionStatus', () => {
      it('returns connection status', () => {
        const store = useConnectionStore()
        store.connectionStatus['conn-1'] = ConnectionStatus.CONNECTED

        expect(store.getConnectionStatus('conn-1')).toBe(ConnectionStatus.CONNECTED)
      })

      it('returns DISCONNECTED for unknown connection', () => {
        const store = useConnectionStore()
        expect(store.getConnectionStatus('unknown')).toBe(ConnectionStatus.DISCONNECTED)
      })
    })

    describe('getCurrentDatabase', () => {
      it('returns current database from state', () => {
        const store = useConnectionStore()
        store.currentDatabases['conn-1'] = 'mydb'

        expect(store.getCurrentDatabase('conn-1')).toBe('mydb')
      })

      it('returns connection database when no current set', () => {
        const store = useConnectionStore()
        store.connections = [
          { id: 'conn-1', name: 'DB1', type: DatabaseType.POSTGRESQL, host: 'localhost', port: 5432, ssl: true, database: 'defaultdb' },
        ]

        expect(store.getCurrentDatabase('conn-1')).toBe('defaultdb')
      })

      it('returns empty string when no database info', () => {
        const store = useConnectionStore()
        expect(store.getCurrentDatabase('unknown')).toBe('')
      })
    })
  })

  describe('actions', () => {
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

      it('maps backend connection types correctly', async () => {
        connectionApi.list.mockResolvedValue([
          { id: '1', name: 'PG', db_type: 'PostgreSQL', host: 'localhost', port: 5432 },
          { id: '2', name: 'My', db_type: 'MySQL', host: 'localhost', port: 3306 },
          { id: '3', name: 'MS', db_type: 'SqlServer', host: 'localhost', port: 1433 },
          { id: '4', name: 'Lite', db_type: 'SQLite', host: '', port: 0 },
        ])

        const store = useConnectionStore()
        await store.fetchConnections()

        expect(store.connections[0].type).toBe(DatabaseType.POSTGRESQL)
        expect(store.connections[1].type).toBe(DatabaseType.MYSQL)
        expect(store.connections[2].type).toBe(DatabaseType.SQLSERVER)
        expect(store.connections[3].type).toBe(DatabaseType.SQLITE)
      })

      it('defaults unknown db_type to PostgreSQL', async () => {
        connectionApi.list.mockResolvedValue([
          { id: '1', name: 'Unknown', db_type: 'UnknownDB', host: 'localhost', port: 5432 },
        ])

        const store = useConnectionStore()
        await store.fetchConnections()

        expect(store.connections[0].type).toBe(DatabaseType.POSTGRESQL)
      })

      it('maps ssl_mode from backend to SslConfig', async () => {
        connectionApi.list.mockResolvedValue([
          { id: '1', name: 'Test', db_type: 'PostgreSQL', host: 'localhost', port: 5432, ssl_mode: 'prefer' },
        ])

        const store = useConnectionStore()
        await store.fetchConnections()

        expect(store.connections[0].ssl).toEqual({ mode: 'prefer' })
      })

      it('handles null ssl_mode from backend', async () => {
        connectionApi.list.mockResolvedValue([
          { id: '1', name: 'Test', db_type: 'PostgreSQL', host: 'localhost', port: 5432, ssl_mode: null },
        ])

        const store = useConnectionStore()
        await store.fetchConnections()

        expect(store.connections[0].ssl).toEqual({ mode: 'disable' })
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

      it('handles non-Error objects in catch', async () => {
        connectionApi.save.mockRejectedValue('string error')

        const store = useConnectionStore()
        const conn = createMockConnection()
        const result = await store.saveConnection(conn)

        expect(result.success).toBe(false)
        expect(result.message).toBe('Unknown error')
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

      it('does nothing when connection has no id', async () => {
        const store = useConnectionStore()
        const conn = createMockConnection({ id: undefined })
        store.connections = [conn]

        await store.removeConnection(conn)

        expect(connectionApi.delete).not.toHaveBeenCalled()
        expect(store.connections).toHaveLength(1)
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

    describe('connect', () => {
      it('connects successfully and updates state', async () => {
        connectionApi.connect.mockResolvedValue({
          is_connected: true,
          current_database: 'testdb',
          server_version: '15.0',
        })

        const store = useConnectionStore()
        store.connections = [
          { id: 'conn-1', name: 'TestDB', type: DatabaseType.POSTGRESQL, host: 'localhost', port: 5432, ssl: true },
        ]

        const result = await store.connect('conn-1')

        expect(store.connectionStatus['conn-1']).toBe(ConnectionStatus.CONNECTED)
        expect(store.activeConnectionId).toBe('conn-1')
        expect(store.connections[0].isConnected).toBe(true)
        expect(store.currentDatabases['conn-1']).toBe('testdb')
        expect(result.is_connected).toBe(true)
      })

      it('sets CONNECTING status during connection', async () => {
        let resolveConnect: ((value: unknown) => void) | undefined
        connectionApi.connect.mockImplementation(() => new Promise((resolve) => {
          resolveConnect = resolve
        }))

        const store = useConnectionStore()
        store.connections = [
          { id: 'conn-1', name: 'TestDB', type: DatabaseType.POSTGRESQL, host: 'localhost', port: 5432, ssl: true },
        ]

        const connectPromise = store.connect('conn-1')

        expect(store.connectionStatus['conn-1']).toBe(ConnectionStatus.CONNECTING)

        resolveConnect?.({ is_connected: true })
        await connectPromise
      })

      it('throws error when connection not found', async () => {
        const store = useConnectionStore()

        await expect(store.connect('non-existent')).rejects.toThrow('Connection not found')
      })

      it('sets ERROR status on connection failure', async () => {
        connectionApi.connect.mockRejectedValue(new Error('Connection refused'))

        const store = useConnectionStore()
        store.connections = [
          { id: 'conn-1', name: 'TestDB', type: DatabaseType.POSTGRESQL, host: 'localhost', port: 5432, ssl: true },
        ]

        await expect(store.connect('conn-1')).rejects.toThrow('Failed to connect')
        expect(store.connectionStatus['conn-1']).toBe(ConnectionStatus.ERROR)
      })

      it('uses resolved database when current_database not returned', async () => {
        connectionApi.connect.mockResolvedValue({
          is_connected: true,
        })

        const store = useConnectionStore()
        store.connections = [
          { id: 'conn-1', name: 'TestDB', type: DatabaseType.POSTGRESQL, host: 'localhost', port: 5432, ssl: true },
        ]

        await store.connect('conn-1')

        expect(store.currentDatabases['conn-1']).toBe('postgres')
      })

      it('sets lastUsed date on successful connection', async () => {
        connectionApi.connect.mockResolvedValue({ is_connected: true })

        const store = useConnectionStore()
        store.connections = [
          { id: 'conn-1', name: 'TestDB', type: DatabaseType.POSTGRESQL, host: 'localhost', port: 5432, ssl: true },
        ]

        await store.connect('conn-1')

        expect(store.connections[0].lastUsed).toBeInstanceOf(Date)
      })
    })

    describe('disconnect', () => {
      it('disconnects successfully', async () => {
        connectionApi.disconnect.mockResolvedValue(undefined)

        const store = useConnectionStore()
        store.connections = [
          { id: 'conn-1', name: 'TestDB', type: DatabaseType.POSTGRESQL, host: 'localhost', port: 5432, ssl: true, isConnected: true },
        ]
        store.activeConnectionId = 'conn-1'
        store.connectionStatus['conn-1'] = ConnectionStatus.CONNECTED
        store.currentDatabases['conn-1'] = 'testdb'

        await store.disconnect('conn-1')

        expect(store.connections[0].isConnected).toBe(false)
        expect(store.connectionStatus['conn-1']).toBe(ConnectionStatus.DISCONNECTED)
        expect(store.activeConnectionId).toBeNull()
        expect(store.currentDatabases['conn-1']).toBeUndefined()
      })

      it('handles disconnect when connection not found', async () => {
        connectionApi.disconnect.mockResolvedValue(undefined)

        const store = useConnectionStore()
        store.activeConnectionId = 'conn-1'
        store.connectionStatus['conn-1'] = ConnectionStatus.CONNECTED

        await store.disconnect('conn-1')

        expect(store.connectionStatus['conn-1']).toBe(ConnectionStatus.DISCONNECTED)
      })

      it('updates state even when API fails', async () => {
        connectionApi.disconnect.mockRejectedValue(new Error('Network error'))

        const store = useConnectionStore()
        store.connections = [
          { id: 'conn-1', name: 'TestDB', type: DatabaseType.POSTGRESQL, host: 'localhost', port: 5432, ssl: true, isConnected: true },
        ]
        store.connectionStatus['conn-1'] = ConnectionStatus.CONNECTED

        try {
          await store.disconnect('conn-1')
        }
        catch {}

        expect(store.connections[0].isConnected).toBe(false)
        expect(store.connectionStatus['conn-1']).toBe(ConnectionStatus.DISCONNECTED)
      })
    })

    describe('setActiveConnection', () => {
      it('sets active connection id', () => {
        const store = useConnectionStore()
        store.setActiveConnection('conn-1')

        expect(store.activeConnectionId).toBe('conn-1')
      })

      it('clears active connection when null', () => {
        const store = useConnectionStore()
        store.activeConnectionId = 'conn-1'
        store.setActiveConnection(null)

        expect(store.activeConnectionId).toBeNull()
      })
    })

    describe('setCurrentDatabase', () => {
      it('sets current database for connection', () => {
        const store = useConnectionStore()
        store.setCurrentDatabase('conn-1', 'newdb')

        expect(store.currentDatabases['conn-1']).toBe('newdb')
      })

      it('overwrites existing database', () => {
        const store = useConnectionStore()
        store.currentDatabases['conn-1'] = 'olddb'
        store.setCurrentDatabase('conn-1', 'newdb')

        expect(store.currentDatabases['conn-1']).toBe('newdb')
      })
    })
  })
})
