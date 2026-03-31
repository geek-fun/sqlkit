import type { HistoryEntry } from '@/store/historyStore'
import { createPinia, setActivePinia } from 'pinia'
import { useHistoryStore } from '@/store/historyStore'

// Mock crypto.randomUUID
Object.defineProperty(globalThis, 'crypto', {
  value: {
    randomUUID: jest.fn(() => 'mock-uuid-123'),
  },
})

type NewHistoryEntry = Omit<HistoryEntry, 'id' | 'isFavorite'>

function makeEntry(overrides: Partial<NewHistoryEntry> = {}): NewHistoryEntry {
  return {
    sql: 'SELECT 1',
    connectionId: 'conn-1',
    connectionName: 'My DB',
    database: 'testdb',
    timestamp: 1000,
    executionTime: 50,
    status: 'success' as const,
    ...overrides,
  }
}

describe('historyStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    jest.clearAllMocks()
    let counter = 0
    ;(crypto.randomUUID as jest.Mock).mockImplementation(() => `uuid-${++counter}`)
  })

  describe('initial state', () => {
    it('should have empty entries and default maxEntries', () => {
      const store = useHistoryStore()

      expect(store.entries).toEqual([])
      expect(store.maxEntries).toBe(500)
    })
  })

  describe('addEntry', () => {
    it('should add a new entry with generated id and isFavorite=false', () => {
      const store = useHistoryStore()

      store.addEntry(makeEntry())

      expect(store.entries).toHaveLength(1)
      expect(store.entries[0].id).toBe('uuid-1')
      expect(store.entries[0].isFavorite).toBe(false)
      expect(store.entries[0].sql).toBe('SELECT 1')
    })

    it('should add entries in reverse-chronological order (newest first)', () => {
      const store = useHistoryStore()

      store.addEntry(makeEntry({ sql: 'SELECT 1', timestamp: 1000 }))
      store.addEntry(makeEntry({ sql: 'SELECT 2', timestamp: 2000 }))

      expect(store.entries[0].sql).toBe('SELECT 2')
      expect(store.entries[1].sql).toBe('SELECT 1')
    })

    it('should trim non-favorite entries when over maxEntries', () => {
      const store = useHistoryStore()
      store.maxEntries = 3

      store.addEntry(makeEntry({ sql: 'A', timestamp: 1 }))
      store.addEntry(makeEntry({ sql: 'B', timestamp: 2 }))
      store.addEntry(makeEntry({ sql: 'C', timestamp: 3 }))
      store.addEntry(makeEntry({ sql: 'D', timestamp: 4 }))

      expect(store.entries.length).toBeLessThanOrEqual(3)
    })

    it('should not remove favorite entries when trimming', () => {
      const store = useHistoryStore()
      store.maxEntries = 2

      store.addEntry(makeEntry({ sql: 'A', timestamp: 1 }))
      store.addEntry(makeEntry({ sql: 'B', timestamp: 2 }))
      store.entries[store.entries.length - 1].isFavorite = true // mark oldest ('A') as favorite

      store.addEntry(makeEntry({ sql: 'C', timestamp: 3 }))

      const sqls = store.entries.map(e => e.sql)
      // Favorite 'A' must be preserved; newest non-favorite 'C' fills the remaining slot
      expect(sqls).toContain('A') // favorite should always be kept
      expect(store.entries.length).toBeLessThanOrEqual(2)
    })
  })

  describe('deleteEntry', () => {
    it('should remove the entry by id', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry())
      const id = store.entries[0].id

      store.deleteEntry(id)

      expect(store.entries).toHaveLength(0)
    })

    it('should not throw for non-existent id', () => {
      const store = useHistoryStore()

      expect(() => store.deleteEntry('non-existent')).not.toThrow()
    })
  })

  describe('toggleFavorite', () => {
    it('should toggle isFavorite from false to true', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry())
      const id = store.entries[0].id

      store.toggleFavorite(id)

      expect(store.entries[0].isFavorite).toBe(true)
    })

    it('should toggle isFavorite from true to false', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry())
      const id = store.entries[0].id
      store.entries[0].isFavorite = true

      store.toggleFavorite(id)

      expect(store.entries[0].isFavorite).toBe(false)
    })
  })

  describe('clearAll', () => {
    it('should remove all entries including favorites', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry())
      store.addEntry(makeEntry())
      store.entries[0].isFavorite = true

      store.clearAll()

      expect(store.entries).toHaveLength(0)
    })
  })

  describe('clearNonFavorites', () => {
    it('should keep only favorite entries', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry({ sql: 'A' }))
      store.addEntry(makeEntry({ sql: 'B' }))
      store.entries[0].isFavorite = true

      store.clearNonFavorites()

      expect(store.entries).toHaveLength(1)
      expect(store.entries[0].isFavorite).toBe(true)
    })
  })

  describe('setMaxEntries', () => {
    it('should update maxEntries', () => {
      const store = useHistoryStore()

      store.setMaxEntries(100)

      expect(store.maxEntries).toBe(100)
    })
  })

  describe('sortedEntries getter', () => {
    it('should return favorites first, then by timestamp descending', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry({ sql: 'old-regular', timestamp: 100 }))
      store.addEntry(makeEntry({ sql: 'new-regular', timestamp: 300 }))
      store.addEntry(makeEntry({ sql: 'old-fav', timestamp: 200 }))
      store.entries.find(e => e.sql === 'old-fav')!.isFavorite = true

      const sorted = store.sortedEntries

      expect(sorted[0].sql).toBe('old-fav') // favorite first
      expect(sorted[1].sql).toBe('new-regular') // newest non-fav
      expect(sorted[2].sql).toBe('old-regular') // oldest non-fav
    })
  })

  describe('filteredEntries getter', () => {
    it('should filter by search keyword in sql', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry({ sql: 'SELECT * FROM users' }))
      store.addEntry(makeEntry({ sql: 'SELECT * FROM orders' }))

      const result = store.filteredEntries('users', '')

      expect(result).toHaveLength(1)
      expect(result[0].sql).toContain('users')
    })

    it('should filter by status', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry({ sql: 'A', status: 'success' }))
      store.addEntry(makeEntry({ sql: 'B', status: 'error' }))

      expect(store.filteredEntries('', 'success')).toHaveLength(1)
      expect(store.filteredEntries('', 'error')).toHaveLength(1)
    })

    it('should return all entries when no filter is applied', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry({ sql: 'A' }))
      store.addEntry(makeEntry({ sql: 'B' }))

      expect(store.filteredEntries('', '')).toHaveLength(2)
    })

    it('should filter by connection name', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry({ connectionName: 'Prod DB' }))
      store.addEntry(makeEntry({ connectionName: 'Dev DB' }))

      expect(store.filteredEntries('Prod', '')).toHaveLength(1)
    })

    it('should return favorites first in filtered results', () => {
      const store = useHistoryStore()
      store.addEntry(makeEntry({ sql: 'SELECT a', timestamp: 100 }))
      store.addEntry(makeEntry({ sql: 'SELECT b', timestamp: 200 }))
      store.entries.find(e => e.sql === 'SELECT a')!.isFavorite = true

      const result = store.filteredEntries('SELECT', '')

      expect(result[0].sql).toBe('SELECT a') // favorite first
    })
  })
})
