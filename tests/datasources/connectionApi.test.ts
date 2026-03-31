import type { ConnectionStatus, ServerConfig } from '@/datasources/connectionApi'
import { connectionApi } from '@/datasources/connectionApi'

jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}))

const invoke = jest.requireMock('@tauri-apps/api/core').invoke as jest.Mock

describe('connectionApi', () => {
  beforeEach(() => {
    jest.clearAllMocks()
  })

  const createMockConfig = (overrides: Partial<ServerConfig> = {}): ServerConfig => ({
    id: 'conn-1',
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

  describe('save', () => {
    it('saves connection and returns id', async () => {
      invoke.mockResolvedValue('new-conn-uuid')

      const config = createMockConfig()
      const result = await connectionApi.save(config)

      expect(invoke).toHaveBeenCalledWith('save_connection', { config })
      expect(result).toBe('new-conn-uuid')
    })

    it('handles connection without optional fields', async () => {
      invoke.mockResolvedValue('conn-id')

      const config = createMockConfig({ password: undefined, database: undefined })
      await connectionApi.save(config)

      expect(invoke).toHaveBeenCalledWith('save_connection', { config })
    })
  })

  describe('list', () => {
    it('returns list of server configs', async () => {
      const mockConfigs: ServerConfig[] = [
        createMockConfig({ id: 'conn-1', name: 'DB1' }),
        createMockConfig({ id: 'conn-2', name: 'DB2' }),
      ]
      invoke.mockResolvedValue(mockConfigs)

      const result = await connectionApi.list()

      expect(invoke).toHaveBeenCalledWith('list_connections')
      expect(result).toEqual(mockConfigs)
    })

    it('returns empty array when no connections', async () => {
      invoke.mockResolvedValue([])

      const result = await connectionApi.list()

      expect(result).toEqual([])
    })
  })

  describe('delete', () => {
    it('deletes connection by id', async () => {
      invoke.mockResolvedValue(undefined)

      await connectionApi.delete('conn-1')

      expect(invoke).toHaveBeenCalledWith('delete_connection', { id: 'conn-1' })
    })
  })

  describe('test', () => {
    it('tests connection and returns status', async () => {
      const mockStatus: ConnectionStatus = {
        is_connected: true,
        server_version: '15.0',
        current_database: 'testdb',
        current_user: 'admin',
      }
      invoke.mockResolvedValue(mockStatus)

      const config = createMockConfig()
      const result = await connectionApi.test(config)

      expect(invoke).toHaveBeenCalledWith('test_connection', { config })
      expect(result).toEqual(mockStatus)
    })

    it('returns disconnected status on failure', async () => {
      const mockStatus: ConnectionStatus = {
        is_connected: false,
      }
      invoke.mockResolvedValue(mockStatus)

      const result = await connectionApi.test(createMockConfig())

      expect(result.is_connected).toBe(false)
    })
  })

  describe('connect', () => {
    it('connects to server and returns status', async () => {
      const mockStatus: ConnectionStatus = {
        is_connected: true,
        server_version: '15.0',
        current_database: 'testdb',
      }
      invoke.mockResolvedValue(mockStatus)

      const config = createMockConfig()
      const result = await connectionApi.connect(config)

      expect(invoke).toHaveBeenCalledWith('connect_server', { config })
      expect(result).toEqual(mockStatus)
    })
  })

  describe('disconnect', () => {
    it('disconnects from server', async () => {
      invoke.mockResolvedValue(undefined)

      await connectionApi.disconnect('conn-1')

      expect(invoke).toHaveBeenCalledWith('disconnect_server', { id: 'conn-1' })
    })
  })

  describe('getStatus', () => {
    it('returns connection status', async () => {
      const mockStatus: ConnectionStatus = {
        is_connected: true,
        server_version: '15.0',
        metadata: { uptime: '12345' },
      }
      invoke.mockResolvedValue(mockStatus)

      const result = await connectionApi.getStatus('conn-1')

      expect(invoke).toHaveBeenCalledWith('get_connection_status', { id: 'conn-1' })
      expect(result).toEqual(mockStatus)
    })

    it('returns disconnected status', async () => {
      const mockStatus: ConnectionStatus = {
        is_connected: false,
      }
      invoke.mockResolvedValue(mockStatus)

      const result = await connectionApi.getStatus('conn-1')

      expect(result.is_connected).toBe(false)
    })
  })
})
