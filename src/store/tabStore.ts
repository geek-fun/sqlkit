import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'

export interface QueryResult {
  columns: string[]
  rows: Record<string, unknown>[]
  rowCount: number
}

export interface QueryTab {
  id: string
  name: string
  content: string
  connectionId: string
  database?: string
  isExecuting: boolean
  hasUnsavedChanges: boolean
  results?: QueryResult
  error?: string
  executionTime?: number
}

interface TabStoreState {
  tabs: QueryTab[]
  activeTabId: string | null
}

const generateId = (): string => crypto.randomUUID()

export const useTabStore = defineStore('tabs', {
  state: (): TabStoreState => ({
    tabs: [],
    activeTabId: null,
  }),

  getters: {
    activeTab: (state): QueryTab | undefined =>
      state.tabs.find(t => t.id === state.activeTabId),

    tabById: state => (id: string): QueryTab | undefined =>
      state.tabs.find(t => t.id === id),

    unsavedTabs: (state): QueryTab[] =>
      state.tabs.filter(t => t.hasUnsavedChanges),

    tabCount: (state): number => state.tabs.length,
  },

  actions: {
    createTab(connectionId: string, database?: string): QueryTab {
      const tab: QueryTab = {
        id: generateId(),
        name: `Query ${this.tabs.length + 1}`,
        content: '',
        connectionId,
        database,
        isExecuting: false,
        hasUnsavedChanges: false,
      }
      this.tabs.push(tab)
      this.activeTabId = tab.id
      return tab
    },

    closeTab(tabId: string) {
      const index = this.tabs.findIndex(t => t.id === tabId)
      if (index === -1)
        return

      this.tabs.splice(index, 1)

      if (this.activeTabId === tabId) {
        this.activeTabId = this.tabs[Math.max(0, index - 1)]?.id ?? null
      }
    },

    closeAllTabs() {
      this.tabs = []
      this.activeTabId = null
    },

    closeTabsForConnection(connectionId: string) {
      this.tabs = this.tabs.filter(t => t.connectionId !== connectionId)
      if (this.activeTabId && !this.tabs.find(t => t.id === this.activeTabId)) {
        this.activeTabId = this.tabs[0]?.id ?? null
      }
    },

    updateTabContent(tabId: string, content: string) {
      const tab = this.tabs.find(t => t.id === tabId)
      if (tab) {
        tab.content = content
        tab.hasUnsavedChanges = true
      }
    },

    updateTabName(tabId: string, name: string) {
      const tab = this.tabs.find(t => t.id === tabId)
      if (tab) {
        tab.name = name
      }
    },

    async executeQuery(tabId: string) {
      const tab = this.tabs.find(t => t.id === tabId)
      if (!tab)
        return

      tab.isExecuting = true
      tab.error = undefined

      const startTime = Date.now()

      try {
        const result = await invoke<QueryResult>('execute_query', {
          connectionId: tab.connectionId,
          sql: tab.content,
        })

        tab.results = result
        tab.executionTime = Date.now() - startTime
      }
      catch (error) {
        tab.error = String(error)
      }
      finally {
        tab.isExecuting = false
      }
    },

    setActiveTab(tabId: string) {
      if (this.tabs.find(t => t.id === tabId)) {
        this.activeTabId = tabId
      }
    },

    markTabSaved(tabId: string) {
      const tab = this.tabs.find(t => t.id === tabId)
      if (tab) {
        tab.hasUnsavedChanges = false
      }
    },

    clearResults(tabId: string) {
      const tab = this.tabs.find(t => t.id === tabId)
      if (tab) {
        tab.results = undefined
        tab.error = undefined
        tab.executionTime = undefined
      }
    },
  },

  persist: {
    pick: ['tabs', 'activeTabId'],
  },
})
