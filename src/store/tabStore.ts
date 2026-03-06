import type { ApiError, ApiResponse } from '@/types/api'
import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { isApiError, isApiSuccess } from '@/types/api'

export interface QueryResult {
  columns: string[]
  rows: Record<string, unknown>[]
  rowCount: number
  rowsAffected?: number
  executionTimeMs?: number
}

/** Metadata for a table-view tab (opened by clicking a table in the sidebar). */
export interface TableViewMeta {
  tableName: string
  database: string
  schema?: string
}

export interface QueryTab {
  id: string
  name: string
  content: string
  connectionId: string
  database?: string
  isExecuting: boolean
  hasUnsavedChanges: boolean
  filePath?: string
  results?: QueryResult
  error?: ApiError | string
  executionTime?: number
  /** When set, this tab renders a DataTableView instead of the SQL editor. */
  tableView?: TableViewMeta
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

    /** Open a table-view tab for the given table, or switch to one that is already open. */
    openTableViewTab(connectionId: string, database: string, tableName: string, schema?: string): QueryTab {
      const existing = this.tabs.find(
        t => t.tableView
          && t.tableView.tableName === tableName
          && t.tableView.database === database
          && (t.tableView.schema ?? null) === (schema ?? null)
          && t.connectionId === connectionId,
      )
      if (existing) {
        this.activeTabId = existing.id
        return existing
      }

      const tab: QueryTab = {
        id: generateId(),
        name: tableName,
        content: '',
        connectionId,
        database,
        isExecuting: false,
        hasUnsavedChanges: false,
        tableView: { tableName, database, schema },
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

    async executeQuery(tabId: string, sqlToExecute?: string) {
      const tab = this.tabs.find(t => t.id === tabId)
      if (!tab) {
        return
      }

      const sql = sqlToExecute !== undefined ? sqlToExecute : tab.content

      // Validate SQL is a non-empty string
      if (typeof sql !== 'string' || sql.trim() === '') {
        return
      }

      tab.isExecuting = true
      tab.error = undefined

      const startTime = Date.now()

      try {
        const response = await invoke<ApiResponse<QueryResult>>('execute_query', {
          connectionId: tab.connectionId,
          sql,
        })

        if (isApiSuccess(response)) {
          tab.results = response.data
          tab.executionTime = Date.now() - startTime
        }
        else if (isApiError(response)) {
          tab.error = response.error
        }
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

    markTabSaved(tabId: string, filePath?: string) {
      const tab = this.tabs.find(t => t.id === tabId)
      if (tab) {
        tab.hasUnsavedChanges = false
        if (filePath) {
          tab.filePath = filePath
          // Update tab name to just the filename
          const fileName = filePath.split('/').pop() || filePath
          tab.name = fileName.replace(/\.sql$/, '')
        }
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
