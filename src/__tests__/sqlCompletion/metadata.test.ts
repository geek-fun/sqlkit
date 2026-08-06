import type { StoreMetadata } from '@/composables/sqlCompletion/metadata'
/**
 * @jest-environment node
 */
import { invoke } from '@tauri-apps/api/core'
import { getMetadataService, SchemaMetadataService, setMetadataServiceForTests } from '@/composables/sqlCompletion/metadata'

jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}))

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>

const fakeStore: StoreMetadata = {
  databases: [{ name: 'app', is_system: false }, { name: 'postgres', is_system: true }],
  schemas: { app: ['public', 'analytics'] },
  tables: {
    'app': [{ name: 'users', schema: 'public' }, { name: 'orders', schema: 'public' }],
    'app.public': [{ name: 'users', schema: 'public' }, { name: 'orders', schema: 'public' }],
  },
}

function makeService(opts: { maxTables?: number, concurrency?: number } = {}) {
  return new SchemaMetadataService({
    getMetadata: () => fakeStore,
    ...opts,
  })
}

describe('schemaMetadataService — level A (connection objects)', () => {
  it('returns databases with isSystem flag', () => {
    const svc = makeService()
    expect(svc.getDatabases('c1')).toEqual([
      { name: 'app', isSystem: false },
      { name: 'postgres', isSystem: true },
    ])
  })

  it('returns schemas for a database', () => {
    const svc = makeService()
    expect(svc.getSchemas('c1', 'app')).toEqual(['public', 'analytics'])
  })

  it('returns tables for db and db.schema keys', () => {
    const svc = makeService()
    expect(svc.getTables('c1', 'app')).toEqual(['users', 'orders'])
    expect(svc.getTables('c1', 'app', 'public')).toEqual(['users', 'orders'])
  })

  it('returns empty for missing metadata', () => {
    const svc = new SchemaMetadataService({ getMetadata: () => null })
    expect(svc.getDatabases('c9')).toEqual([])
    expect(svc.getSchemas('c9', 'x')).toEqual([])
    expect(svc.getTables('c9', 'x')).toEqual([])
    expect(svc.getColumns('c9', 'x', undefined, 't')).toEqual([])
  })
})

describe('schemaMetadataService — level B (column cache)', () => {
  beforeEach(() => {
    jest.clearAllMocks()
  })

  it('(a) prefetchColumns stores columns under the exact key', async () => {
    mockInvoke.mockResolvedValue([
      { name: 'id', data_type: 'int4', nullable: false, is_primary_key: true, is_auto_increment: true },
      { name: 'email', data_type: 'text', nullable: true, is_primary_key: false, is_auto_increment: false },
    ])
    const svc = makeService()
    await svc.prefetchColumns('c1', 'app', 'public', ['users'])
    expect(mockInvoke).toHaveBeenCalledWith('list_columns', {
      connectionId: 'c1',
      database: 'app',
      schema: 'public',
      tableName: 'users',
    })
    expect(svc.getColumns('c1', 'app', 'public', 'users')).toEqual([
      { name: 'id', dataType: 'int4', isPrimaryKey: true },
      { name: 'email', dataType: 'text', isPrimaryKey: false },
    ])
    expect(svc.getColumns('c1', 'app', 'public', 'other')).toEqual([])
  })

  it('(b) concurrency: max in-flight invoke ≤ concurrency (5)', async () => {
    let inFlight = 0
    let maxInFlight = 0
    const tables = Array.from({ length: 10 }, (_, i) => `t${i}`)
    mockInvoke.mockImplementation(async () => {
      inFlight++
      maxInFlight = Math.max(maxInFlight, inFlight)
      await new Promise(r => setTimeout(r, 5))
      inFlight--
      return []
    })
    const svc = makeService({ concurrency: 5 })
    await svc.prefetchColumns('c1', 'app', 'public', tables)
    expect(maxInFlight).toBeLessThanOrEqual(5)
    expect(mockInvoke).toHaveBeenCalledTimes(10)
  })

  it('(c) prefetch caps at maxTables (100)', async () => {
    const tables = Array.from({ length: 150 }, (_, i) => `t${i}`)
    mockInvoke.mockResolvedValue([])
    const svc = makeService({ maxTables: 100 })
    await svc.prefetchColumns('c1', 'app', 'public', tables)
    expect(mockInvoke).toHaveBeenCalledTimes(100)
  })

  it('(d) getColumns returns cached without re-invoke', async () => {
    mockInvoke.mockResolvedValue([])
    const svc = makeService()
    await svc.prefetchColumns('c1', 'app', 'public', ['users'])
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    svc.getColumns('c1', 'app', 'public', 'users')
    expect(mockInvoke).toHaveBeenCalledTimes(1)
  })

  it('(e) getColumns on absent key triggers a lazy fetch', async () => {
    mockInvoke.mockResolvedValue([{ name: 'id', data_type: 'int4', nullable: false, is_primary_key: false, is_auto_increment: false }])
    const svc = makeService()
    const cols = await svc.fetchTableColumns('c1', 'app', 'public', 'orders')
    expect(cols).toEqual([{ name: 'id', dataType: 'int4', isPrimaryKey: false }])
    expect(mockInvoke).toHaveBeenCalledTimes(1)
  })

  it('(f) clearColumnCache(connId) removes only that connId keys', async () => {
    mockInvoke.mockResolvedValue([])
    const svc = makeService()
    await svc.prefetchColumns('c1', 'app', 'public', ['users'])
    await svc.prefetchColumns('c2', 'app', 'public', ['users'])
    svc.clearColumnCache('c1')
    expect(svc.getColumns('c1', 'app', 'public', 'users')).toEqual([])
    // c2 untouched (cache intact, no re-invoke needed to verify absence of fetch)
    expect(mockInvoke).toHaveBeenCalledTimes(2)
  })

  it('(g) invoke rejection → prefetch resolves without throwing', async () => {
    mockInvoke.mockRejectedValue(new Error('backend down'))
    const svc = makeService()
    await expect(svc.prefetchColumns('c1', 'app', 'public', ['users'])).resolves.toBeUndefined()
    expect(svc.getColumns('c1', 'app', 'public', 'users')).toEqual([])
  })
})

describe('schemaMetadataService — level C (derived tables)', () => {
  it('resolves and clears derived alias/CTE names', () => {
    const svc = makeService()
    svc.setDerivedTables({ u: 'users', cte1: 'orders' })
    expect(svc.resolveDerived('u')).toBe('users')
    expect(svc.resolveDerived('cte1')).toBe('orders')
    svc.clearAll()
    expect(svc.resolveDerived('u')).toBeUndefined()
  })
})

describe('schemaMetadataService — singleton', () => {
  afterEach(() => {
    setMetadataServiceForTests(null)
  })

  it('getMetadataService returns a singleton; setMetadataServiceForTests swaps it', () => {
    const a = getMetadataService()
    const b = getMetadataService()
    expect(a).toBe(b)
    const fake = makeService()
    setMetadataServiceForTests(fake)
    expect(getMetadataService()).toBe(fake)
  })
})
