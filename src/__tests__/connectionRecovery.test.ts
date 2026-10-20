import { listen } from '@tauri-apps/api/event'
import { createPinia, setActivePinia } from 'pinia'
import { ConnectionStatus, useConnectionStore } from '@/store/connectionStore'

jest.mock('@tauri-apps/api/event', () => ({ listen: jest.fn().mockResolvedValue(jest.fn()) }))
jest.mock('@/datasources', () => ({ connectionApi: {} }))
jest.mock('@/common', () => ({ markSshTunnelStripped: jest.fn(), shouldStripSshTunnel: jest.fn() }))
jest.mock('@/store/entitlementStore', () => ({ useEntitlementStore: jest.fn() }))
jest.mock('@/store/sshProfileStore', () => ({ useSshProfileStore: jest.fn(), profileToTransportLayer: jest.fn() }))

describe('connection recovery events', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    jest.clearAllMocks()
  })

  it('keeps connection flags and status synchronized through a failed reconnect and recovery', async () => {
    const store = useConnectionStore()
    store.connections = [{ id: 'test', isConnected: true } as typeof store.connections[number]]
    store.activeConnectionId = 'test'
    store.currentDatabases.test = 'sample_database'
    await store.startStateListener()
    const handler = jest.mocked(listen).mock.calls[0][1]
    const changeState = (state: string) => handler({
      event: 'connection-state-changed',
      id: 0,
      payload: { connection_id: 'test', state, error: null },
    })

    changeState('dead')
    expect(store.getConnectionStatus('test')).toBe(ConnectionStatus.ERROR)
    expect(store.connections[0].isConnected).toBe(false)
    changeState('reconnecting')
    expect(store.getConnectionStatus('test')).toBe(ConnectionStatus.CONNECTING)
    expect(store.connections[0].isConnected).toBe(false)
    changeState('dead')
    expect(store.getConnectionStatus('test')).toBe(ConnectionStatus.ERROR)
    changeState('healthy')
    expect(store.getConnectionStatus('test')).toBe(ConnectionStatus.CONNECTED)
    expect(store.connections[0].isConnected).toBe(true)
    expect(store.activeConnectionId).toBe('test')
    expect(store.currentDatabases.test).toBe('sample_database')
  })
})
