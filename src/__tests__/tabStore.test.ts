import { createPinia, setActivePinia } from 'pinia'
import { useTabStore } from '../store/tabStore'

// Mock the Tauri invoke API
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}))

// eslint-disable-next-line ts/no-require-imports
const { invoke } = require('@tauri-apps/api/core')

// Mock crypto.randomUUID
Object.defineProperty(globalThis, 'crypto', {
  value: {
    randomUUID: jest.fn(() => 'mock-uuid-123'),
  },
})

describe('tabStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    jest.clearAllMocks()
    ;(crypto.randomUUID as jest.Mock).mockReturnValue(`uuid-${Date.now()}`)
  })

  describe('initial state', () => {
    it('should have empty tabs and no active tab', () => {
      const store = useTabStore()

      expect(store.tabs).toEqual([])
      expect(store.activeTabId).toBeNull()
    })
  })

  describe('createTab', () => {
    it('should create a new tab with correct defaults', () => {
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      const store = useTabStore()

      const tab = store.createTab('conn-1')

      expect(tab).toEqual({
        id: 'tab-1',
        name: 'Query 1',
        content: '',
        connectionId: 'conn-1',
        database: undefined,
        isExecuting: false,
        hasUnsavedChanges: false,
      })
      expect(store.tabs).toHaveLength(1)
      expect(store.activeTabId).toBe('tab-1')
    })

    it('should create tab with database', () => {
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      const store = useTabStore()

      const tab = store.createTab('conn-1', 'mydb')

      expect(tab.database).toBe('mydb')
    })

    it('should increment tab name based on count', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-1')
      store.createTab('conn-1')
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-2')
      const tab2 = store.createTab('conn-1')

      expect(tab2.name).toBe('Query 2')
    })
  })

  describe('closeTab', () => {
    it('should remove tab from tabs', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      store.createTab('conn-1')

      store.closeTab('tab-1')

      expect(store.tabs).toHaveLength(0)
      expect(store.activeTabId).toBeNull()
    })

    it('should set previous tab as active when closing active tab', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-1')
      store.createTab('conn-1')
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-2')
      store.createTab('conn-1')

      store.closeTab('tab-2')

      expect(store.activeTabId).toBe('tab-1')
    })

    it('should not affect other tabs when closing non-active tab', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-1')
      store.createTab('conn-1')
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-2')
      store.createTab('conn-1')

      store.closeTab('tab-1')

      expect(store.tabs).toHaveLength(1)
      expect(store.activeTabId).toBe('tab-2')
    })

    it('should handle closing non-existent tab', () => {
      const store = useTabStore()

      store.closeTab('non-existent')

      expect(store.tabs).toHaveLength(0)
    })
  })

  describe('closeAllTabs', () => {
    it('should remove all tabs', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-1')
      store.createTab('conn-1')
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-2')
      store.createTab('conn-1')

      store.closeAllTabs()

      expect(store.tabs).toHaveLength(0)
      expect(store.activeTabId).toBeNull()
    })
  })

  describe('closeTabsForConnection', () => {
    it('should close only tabs for specified connection', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-1')
      store.createTab('conn-1')
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-2')
      store.createTab('conn-2')
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-3')
      store.createTab('conn-1')

      store.closeTabsForConnection('conn-1')

      expect(store.tabs).toHaveLength(1)
      expect(store.tabs[0].connectionId).toBe('conn-2')
    })

    it('should update active tab if needed', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-1')
      store.createTab('conn-1')
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-2')
      store.createTab('conn-2')
      store.setActiveTab('tab-1')

      store.closeTabsForConnection('conn-1')

      expect(store.activeTabId).toBe('tab-2')
    })
  })

  describe('updateTabContent', () => {
    it('should update tab content and mark as unsaved', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      store.createTab('conn-1')

      store.updateTabContent('tab-1', 'SELECT * FROM users')

      expect(store.tabs[0].content).toBe('SELECT * FROM users')
      expect(store.tabs[0].hasUnsavedChanges).toBe(true)
    })

    it('should not throw for non-existent tab', () => {
      const store = useTabStore()

      expect(() => store.updateTabContent('non-existent', 'content')).not.toThrow()
    })
  })

  describe('updateTabName', () => {
    it('should update tab name', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      store.createTab('conn-1')

      store.updateTabName('tab-1', 'My Query')

      expect(store.tabs[0].name).toBe('My Query')
    })
  })

  describe('setActiveTab', () => {
    it('should set active tab id', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-1')
      store.createTab('conn-1')
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-2')
      store.createTab('conn-1')

      store.setActiveTab('tab-1')

      expect(store.activeTabId).toBe('tab-1')
    })

    it('should not set active tab if tab does not exist', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      store.createTab('conn-1')

      store.setActiveTab('non-existent')

      expect(store.activeTabId).toBe('tab-1')
    })
  })

  describe('markTabSaved', () => {
    it('should mark tab as saved', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      store.createTab('conn-1')
      store.updateTabContent('tab-1', 'content')
      expect(store.tabs[0].hasUnsavedChanges).toBe(true)

      store.markTabSaved('tab-1')

      expect(store.tabs[0].hasUnsavedChanges).toBe(false)
    })
  })

  describe('clearResults', () => {
    it('should clear tab results, error, and execution time', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      store.createTab('conn-1')
      store.tabs[0].results = { columns: [], rows: [], rowCount: 0 }
      store.tabs[0].error = 'Some error'
      store.tabs[0].executionTime = 100

      store.clearResults('tab-1')

      expect(store.tabs[0].results).toBeUndefined()
      expect(store.tabs[0].error).toBeUndefined()
      expect(store.tabs[0].executionTime).toBeUndefined()
    })
  })

  describe('executeQuery', () => {
    it('should execute query and store results', async () => {
      const mockResult = {
        columns: ['id', 'name'],
        rows: [{ id: 1, name: 'Test' }],
        rowCount: 1,
      }
      invoke.mockResolvedValue(mockResult)

      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      store.createTab('conn-1')
      store.updateTabContent('tab-1', 'SELECT * FROM users')

      await store.executeQuery('tab-1')

      expect(invoke).toHaveBeenCalledWith('execute_query', {
        connectionId: 'conn-1',
        sql: 'SELECT * FROM users',
      })
      expect(store.tabs[0].results).toEqual(mockResult)
      expect(store.tabs[0].isExecuting).toBe(false)
      expect(store.tabs[0].executionTime).toBeDefined()
    })

    it('should set error on failure', async () => {
      invoke.mockRejectedValue(new Error('Query failed'))

      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      store.createTab('conn-1')
      store.updateTabContent('tab-1', 'INVALID SQL')

      await store.executeQuery('tab-1')

      expect(store.tabs[0].error).toBe('Error: Query failed')
      expect(store.tabs[0].isExecuting).toBe(false)
    })

    it('should not execute for non-existent tab', async () => {
      const store = useTabStore()

      await store.executeQuery('non-existent')

      expect(invoke).not.toHaveBeenCalled()
    })
  })

  describe('getters', () => {
    it('activeTab getter should return active tab', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      store.createTab('conn-1')

      expect(store.activeTab?.id).toBe('tab-1')
    })

    it('activeTab getter should return undefined when no active tab', () => {
      const store = useTabStore()

      expect(store.activeTab).toBeUndefined()
    })

    it('tabById getter should find tab by id', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValue('tab-1')
      store.createTab('conn-1')

      expect(store.tabById('tab-1')?.id).toBe('tab-1')
      expect(store.tabById('non-existent')).toBeUndefined()
    })

    it('unsavedTabs getter should return tabs with unsaved changes', () => {
      const store = useTabStore()
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-1')
      store.createTab('conn-1')
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-2')
      store.createTab('conn-1')
      store.updateTabContent('tab-1', 'modified')

      expect(store.unsavedTabs).toHaveLength(1)
      expect(store.unsavedTabs[0].id).toBe('tab-1')
    })

    it('tabCount getter should return number of tabs', () => {
      const store = useTabStore()

      expect(store.tabCount).toBe(0)

      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-1')
      store.createTab('conn-1')
      ;(crypto.randomUUID as jest.Mock).mockReturnValueOnce('tab-2')
      store.createTab('conn-1')

      expect(store.tabCount).toBe(2)
    })
  })
})
