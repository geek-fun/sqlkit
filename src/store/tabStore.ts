import type { ApiError, ApiResponse } from '@/types/api'
import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { withMinLoadingTime } from '@/composables/useMinLoadingTime'
import { isApiError, isApiSuccess } from '@/types/api'
import { useConnectionStore } from './connectionStore'
import { useHistoryStore } from './historyStore'

export type QueryResult = {
  columns: string[]
  rows: Record<string, unknown>[]
  rowCount: number
  rowsAffected?: number
  executionTimeMs?: number
}

export type TableViewMeta = {
  tableName: string
  database: string
  schema?: string
}

export type QueryTab = {
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
  tableView?: TableViewMeta
}

type TabStoreState = {
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
      this.tabs = [...this.tabs, tab]
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
      this.tabs = [...this.tabs, tab]
      this.activeTabId = tab.id
      return tab
    },

    closeTab(tabId: string) {
      const index = this.tabs.findIndex(t => t.id === tabId)
      if (index === -1)
        return

      this.tabs = [...this.tabs.slice(0, index), ...this.tabs.slice(index + 1)]

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

      const connectionStore = useConnectionStore()
      const connection = connectionStore.getConnectionById(tab.connectionId)
      const historyStore = useHistoryStore()

      const queryStartTime = Date.now()
      let actualExecutionTime = 0

      try {
        let response: ApiResponse<QueryResult>

        await withMinLoadingTime(async () => {
          response = await invoke<ApiResponse<QueryResult>>('execute_query', {
            connectionId: tab.connectionId,
            sql,
            database: tab.database ?? null,
          })
          actualExecutionTime = Date.now() - queryStartTime
        })

        if (isApiSuccess(response!)) {
          tab.results = response!.data
          tab.executionTime = actualExecutionTime
          historyStore.addEntry({
            sql,
            connectionId: tab.connectionId,
            connectionName: connection?.name ?? tab.connectionId,
            database: tab.database,
            timestamp: Date.now(),
            executionTime: actualExecutionTime,
            status: 'success',
          })
        }
        else if (isApiError(response!)) {
          const err = response!.error
          if (tab.database && err.details) {
            const firstBreak = err.details.indexOf('\n')
            if (firstBreak !== -1) {
              err.details = `${err.details.slice(0, firstBreak)} in database: "${tab.database}"${err.details.slice(firstBreak)}`
            }
            else {
              err.details = `${err.details} in database: "${tab.database}"`
            }
          }
          else if (tab.database) {
            err.details = `in database: "${tab.database}"`
          }
          tab.error = err
          historyStore.addEntry({
            sql,
            connectionId: tab.connectionId,
            connectionName: connection?.name ?? tab.connectionId,
            database: tab.database,
            timestamp: Date.now(),
            executionTime: actualExecutionTime,
            status: 'error',
            errorMessage: response!.error.message,
          })
        }
      }
      catch (error) {
        actualExecutionTime = Date.now() - queryStartTime
        tab.error = String(error)
        historyStore.addEntry({
          sql,
          connectionId: tab.connectionId,
          connectionName: connection?.name ?? tab.connectionId,
          database: tab.database,
          timestamp: Date.now(),
          executionTime: actualExecutionTime,
          status: 'error',
          errorMessage: String(error),
        })
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
