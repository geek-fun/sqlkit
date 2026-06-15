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

export type ListingTabMeta = {
  type: 'VIEW' | 'PROCEDURE' | 'FUNCTION'
  database: string
  schema?: string
}

export type QueryTab = {
  id: string
  name: string
  content: string
  /** Connection this tab originated from. Used to detect stale tabs across connection switches. */
  connectionId?: string
  database?: string
  schema?: string
  isExecuting: boolean
  hasUnsavedChanges: boolean
  filePath?: string
  results?: QueryResult
  error?: ApiError | string
  executionTime?: number
  tableView?: TableViewMeta
  listingTab?: ListingTabMeta
  /** If set, this tab is orphaned from the specified connection and cannot execute queries */
  orphanFromConnectionId?: string
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

    tabByFilePath: state => (filePath: string): QueryTab | undefined =>
      state.tabs.find(t => t.filePath === filePath),

    unsavedTabs: (state): QueryTab[] =>
      state.tabs.filter(t => t.hasUnsavedChanges),

    orphanTabs: (state): QueryTab[] =>
      state.tabs.filter(t => t.orphanFromConnectionId),

    isOrphanTab: state => (id: string): boolean =>
      state.tabs.find(t => t.id === id)?.orphanFromConnectionId !== undefined,

    tabCount: (state): number => state.tabs.length,
  },

  actions: {
    createTab(connectionId?: string, database?: string, schema?: string): QueryTab {
      const tab: QueryTab = {
        id: generateId(),
        name: `Query ${this.tabs.length + 1}`,
        content: '',
        connectionId,
        database,
        schema,
        isExecuting: false,
        hasUnsavedChanges: false,
      }
      this.tabs = [...this.tabs, tab]
      this.activeTabId = tab.id
      return tab
    },

    openTableViewTab(connectionId: string, database: string, tableName: string, schema?: string): QueryTab {
      const existing = this.tabs.find(
        t => t.tableView
          && t.tableView.tableName === tableName
          && t.tableView.database === database
          && (t.tableView.schema ?? null) === (schema ?? null)
          && t.connectionId === connectionId
          && !t.orphanFromConnectionId,
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
        schema,
        isExecuting: false,
        hasUnsavedChanges: false,
        tableView: { tableName, database, schema },
      }
      this.tabs = [...this.tabs, tab]
      this.activeTabId = tab.id
      return tab
    },

    openListingTab(connectionId: string, database: string, type: 'VIEW' | 'PROCEDURE' | 'FUNCTION', schema?: string): QueryTab {
      const existing = this.tabs.find(
        t => t.listingTab
          && t.listingTab.type === type
          && t.listingTab.database === database
          && (t.listingTab.schema ?? null) === (schema ?? null)
          && t.connectionId === connectionId
          && !t.orphanFromConnectionId,
      )
      if (existing) {
        this.activeTabId = existing.id
        return existing
      }

      const typeLabel = type === 'VIEW' ? 'Views' : type === 'PROCEDURE' ? 'Procedures' : 'Functions'
      const tabName = schema ? `${typeLabel} — ${schema}` : `${typeLabel} — ${database}`
      const tab: QueryTab = {
        id: generateId(),
        name: tabName,
        content: '',
        connectionId,
        database,
        schema,
        isExecuting: false,
        hasUnsavedChanges: false,
        listingTab: { type, database, schema },
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
      const remaining = this.tabs.filter(t => t.connectionId !== connectionId)
      this.tabs = remaining
      if (this.activeTabId && !remaining.find(t => t.id === this.activeTabId)) {
        this.activeTabId = remaining[0]?.id ?? null
      }
    },

    closeNonOrphanTabs() {
      this.tabs = this.tabs.filter(t => t.orphanFromConnectionId)
      if (this.activeTabId && !this.tabs.find(t => t.id === this.activeTabId)) {
        const orphanTab = this.tabs.find(t => t.orphanFromConnectionId)
        this.activeTabId = orphanTab?.id ?? null
      }
    },

    reconcileTabsForConnection(currentConnectionId: string) {
      const isStale = (t: QueryTab) =>
        !t.orphanFromConnectionId
        && t.connectionId !== currentConnectionId

      const staleTableViewIds = this.tabs
        .filter(t => isStale(t) && t.tableView)
        .map(t => t.id)

      const staleSavedQueryIds = this.tabs
        .filter(t => isStale(t) && !t.tableView && !t.hasUnsavedChanges)
        .map(t => t.id)

      const staleUnsavedQueryTabs = this.tabs.filter(
        t => isStale(t) && !t.tableView && t.hasUnsavedChanges,
      )

      const toCloseIds = new Set([...staleTableViewIds, ...staleSavedQueryIds])

      const orphanedIds = new Set(staleUnsavedQueryTabs.map(t => t.id))

      this.tabs = this.tabs
        .filter(t => !toCloseIds.has(t.id))
        .map(t =>
          orphanedIds.has(t.id)
            ? { ...t, orphanFromConnectionId: t.connectionId ?? 'unknown' }
            : t,
        )

      if (this.activeTabId && !this.tabs.find(t => t.id === this.activeTabId)) {
        const nonOrphanTab = this.tabs.find(t => !t.orphanFromConnectionId)
        const orphanTab = this.tabs.find(t => t.orphanFromConnectionId)
        this.activeTabId = nonOrphanTab?.id ?? orphanTab?.id ?? null
      }
    },

    transitionTabsForConnection(oldConnectionId: string) {
      const tableViewTabIds = this.tabs
        .filter(t => t.tableView && !t.orphanFromConnectionId)
        .map(t => t.id)

      const savedQueryTabIds = this.tabs
        .filter(t => !t.tableView && !t.hasUnsavedChanges && !t.orphanFromConnectionId)
        .map(t => t.id)

      const unsavedQueryTabs = this.tabs.filter(
        t => !t.tableView && t.hasUnsavedChanges && !t.orphanFromConnectionId,
      )

      const toCloseIds = new Set([...tableViewTabIds, ...savedQueryTabIds])

      this.tabs = this.tabs.filter(t => !toCloseIds.has(t.id))

      unsavedQueryTabs.forEach((tab) => {
        const index = this.tabs.findIndex(t => t.id === tab.id)
        if (index !== -1) {
          const orphanedTab = { ...tab, orphanFromConnectionId: oldConnectionId }
          this.tabs = [
            ...this.tabs.slice(0, index),
            orphanedTab,
            ...this.tabs.slice(index + 1),
          ]
        }
      })

      if (this.activeTabId && !this.tabs.find(t => t.id === this.activeTabId)) {
        const nonOrphanTab = this.tabs.find(t => !t.orphanFromConnectionId)
        const orphanTab = this.tabs.find(t => t.orphanFromConnectionId)
        this.activeTabId = nonOrphanTab?.id ?? orphanTab?.id ?? null
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
      if (!tab || tab.orphanFromConnectionId) {
        return
      }

      const sql = sqlToExecute !== undefined ? sqlToExecute : tab.content

      if (typeof sql !== 'string' || sql.trim() === '') {
        return
      }

      const activeConnectionId = tab.connectionId
      if (!activeConnectionId) {
        return
      }

      tab.isExecuting = true
      tab.error = undefined

      const connectionStore = useConnectionStore()
      const connection = connectionStore.getConnectionById(activeConnectionId)
      const historyStore = useHistoryStore()

      const queryStartTime = Date.now()
      let actualExecutionTime = 0

      try {
        let response: ApiResponse<QueryResult>

        await withMinLoadingTime(async () => {
          response = await invoke<ApiResponse<QueryResult>>('execute_query', {
            connectionId: activeConnectionId,
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
            connectionId: activeConnectionId,
            connectionName: connection?.name ?? activeConnectionId,
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
            connectionId: activeConnectionId,
            connectionName: connection?.name ?? activeConnectionId,
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
          connectionId: activeConnectionId,
          connectionName: connection?.name ?? activeConnectionId,
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
          const fileName = filePath.split('/').pop() || filePath
          tab.name = fileName
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
