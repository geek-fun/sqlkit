import { defineStore } from 'pinia'

export type HistoryEntryStatus = 'success' | 'error'

export interface HistoryEntry {
  id: string
  sql: string
  connectionId: string
  connectionName: string
  database?: string
  timestamp: number
  executionTime?: number
  status: HistoryEntryStatus
  errorMessage?: string
  isFavorite: boolean
}

interface HistoryStoreState {
  entries: HistoryEntry[]
  maxEntries: number
}

const generateId = (): string => crypto.randomUUID()

export const useHistoryStore = defineStore('history', {
  state: (): HistoryStoreState => ({
    entries: [],
    maxEntries: 500,
  }),

  getters: {
    sortedEntries: (state): HistoryEntry[] => {
      const favorites = state.entries.filter(e => e.isFavorite).sort((a, b) => b.timestamp - a.timestamp)
      const rest = state.entries.filter(e => !e.isFavorite).sort((a, b) => b.timestamp - a.timestamp)
      return [...favorites, ...rest]
    },

    filteredEntries: state => (search: string, status?: HistoryEntryStatus | ''): HistoryEntry[] => {
      const favorites = state.entries.filter(e => e.isFavorite).sort((a, b) => b.timestamp - a.timestamp)
      const rest = state.entries.filter(e => !e.isFavorite).sort((a, b) => b.timestamp - a.timestamp)
      const sorted = [...favorites, ...rest]

      return sorted.filter((entry) => {
        const matchesSearch = !search
          || entry.sql.toLowerCase().includes(search.toLowerCase())
          || entry.connectionName.toLowerCase().includes(search.toLowerCase())
          || (entry.database ?? '').toLowerCase().includes(search.toLowerCase())
        const matchesStatus = !status || entry.status === status
        return matchesSearch && matchesStatus
      })
    },
  },

  actions: {
    addEntry(entry: Omit<HistoryEntry, 'id' | 'isFavorite'>): void {
      const newEntry: HistoryEntry = {
        ...entry,
        id: generateId(),
        isFavorite: false,
      }
      this.entries.unshift(newEntry)

      // Trim to maxEntries, but always keep favorites
      if (this.entries.length > this.maxEntries) {
        const favorites = this.entries.filter(e => e.isFavorite)
        const nonFavorites = this.entries.filter(e => !e.isFavorite)
        const nonFavoriteSlot = Math.max(0, this.maxEntries - favorites.length)
        this.entries = [...favorites, ...nonFavorites.slice(0, nonFavoriteSlot)]
      }
    },

    deleteEntry(id: string): void {
      this.entries = this.entries.filter(e => e.id !== id)
    },

    toggleFavorite(id: string): void {
      const entry = this.entries.find(e => e.id === id)
      if (entry) {
        entry.isFavorite = !entry.isFavorite
      }
    },

    clearAll(): void {
      this.entries = []
    },

    clearNonFavorites(): void {
      this.entries = this.entries.filter(e => e.isFavorite)
    },

    setMaxEntries(max: number): void {
      this.maxEntries = max
    },
  },

  persist: {
    pick: ['entries', 'maxEntries'],
  },
})
