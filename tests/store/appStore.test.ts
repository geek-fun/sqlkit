import { createPinia, setActivePinia } from 'pinia'
import { LanguageType, ThemeType, useAppStore } from '@/store/appStore'

// Mock window and document for Node.js environment
const mockMediaQuery = {
  matches: false,
  addEventListener: jest.fn(),
  removeEventListener: jest.fn(),
}
const mockMatchMedia = jest.fn().mockReturnValue(mockMediaQuery)
const mockSetAttribute = jest.fn()
const mockClassList = {
  add: jest.fn(),
  remove: jest.fn(),
}

Object.defineProperty(globalThis, 'window', {
  value: {
    matchMedia: mockMatchMedia,
  },
  writable: true,
})

Object.defineProperty(globalThis, 'document', {
  value: {
    documentElement: {
      setAttribute: mockSetAttribute,
      classList: mockClassList,
    },
  },
  writable: true,
})

// Mock storeApi for backend persistence
const mockGetSecret = jest.fn<Promise<unknown>, [string, unknown]>().mockResolvedValue(undefined)
const mockSetSecret = jest.fn().mockResolvedValue(undefined)
jest.mock('@/datasources/storeApi', () => ({
  storeApi: {
    getSecret: (...args: unknown[]) => mockGetSecret(...args as [string, unknown]),
    setSecret: (...args: unknown[]) => mockSetSecret(...args as [string, unknown]),
  },
}))

describe('appStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    jest.clearAllMocks()
  })

  describe('initial state', () => {
    it('should have correct default values', () => {
      const store = useAppStore()

      expect(store.themeType).toBe(ThemeType.AUTO)
      expect(store.languageType).toBe(LanguageType.AUTO)
      expect(store.uiThemeType).toBe(ThemeType.LIGHT)
      expect(store.queryTimeout).toBe(30000)
      expect(store.defaultLimit).toBe(1000)
      expect(store.autoSave).toBe(true)
    })

    it('should have correct default editor config', () => {
      const store = useAppStore()

      expect(store.editorConfig).toEqual({
        fontSize: 14,
        tabSize: 2,
        wordWrap: true,
        showLineNumbers: true,
        showMinimap: false,
        indentWidth: 2,
        lineWidth: 120,
      })
    })

    it('should have correct default query config', () => {
      const store = useAppStore()

      expect(store.queryConfig).toEqual({
        defaultLimit: 1000,
        queryTimeout: 30000,
        autoSave: true,
      })
    })

    it('should have sidebar not collapsed by default', () => {
      const store = useAppStore()

      expect(store.sidebarCollapsed).toBe(false)
    })
  })

  describe('setThemeType', () => {
    it('should set theme to LIGHT', () => {
      const store = useAppStore()
      store.setThemeType(ThemeType.LIGHT)

      expect(store.themeType).toBe(ThemeType.LIGHT)
      expect(store.uiThemeType).toBe(ThemeType.LIGHT)
      expect(mockSetAttribute).toHaveBeenCalledWith('theme', ThemeType.LIGHT)
      expect(mockClassList.remove).toHaveBeenCalledWith('dark')
    })

    it('should set theme to DARK', () => {
      const store = useAppStore()
      store.setThemeType(ThemeType.DARK)

      expect(store.themeType).toBe(ThemeType.DARK)
      expect(store.uiThemeType).toBe(ThemeType.DARK)
      expect(mockSetAttribute).toHaveBeenCalledWith('theme', ThemeType.DARK)
      expect(mockClassList.add).toHaveBeenCalledWith('dark')
    })

    it('should set theme to AUTO and detect system preference (dark)', () => {
      mockMatchMedia.mockReturnValue({ ...mockMediaQuery, matches: true })
      const store = useAppStore()
      store.setThemeType(ThemeType.AUTO)

      expect(store.themeType).toBe(ThemeType.AUTO)
      expect(store.uiThemeType).toBe(ThemeType.DARK)
    })

    it('should set theme to AUTO and detect system preference (light)', () => {
      mockMatchMedia.mockReturnValue({ ...mockMediaQuery, matches: false })
      const store = useAppStore()
      store.setThemeType(ThemeType.AUTO)

      expect(store.themeType).toBe(ThemeType.AUTO)
      expect(store.uiThemeType).toBe(ThemeType.LIGHT)
    })

    it('should cleanup theme listener on cleanup()', () => {
      mockMatchMedia.mockReturnValue({ ...mockMediaQuery, matches: true })
      const store = useAppStore()
      store.setThemeType(ThemeType.AUTO)

      store.cleanup()

      expect(mockMediaQuery.removeEventListener).toHaveBeenCalledWith('change', expect.any(Function))
    })

    it('should not error on cleanup() when no listener registered', () => {
      const store = useAppStore()
      store.setThemeType(ThemeType.LIGHT)

      expect(() => store.cleanup()).not.toThrow()
    })
  })

  describe('setLanguageType', () => {
    it('should set language type', () => {
      const store = useAppStore()
      store.setLanguageType(LanguageType.EN_US)

      expect(store.languageType).toBe(LanguageType.EN_US)
    })
  })

  describe('setEditorConfig', () => {
    it('should update partial editor config', () => {
      const store = useAppStore()
      store.setEditorConfig({ fontSize: 16 })

      expect(store.editorConfig.fontSize).toBe(16)
      expect(store.editorConfig.indentWidth).toBe(2) // unchanged
    })

    it('should update multiple editor config values', () => {
      const store = useAppStore()
      store.setEditorConfig({ fontSize: 18, showMinimap: true })

      expect(store.editorConfig.fontSize).toBe(18)
      expect(store.editorConfig.showMinimap).toBe(true)
    })
  })

  describe('setQueryTimeout', () => {
    it('should set query timeout', () => {
      const store = useAppStore()
      store.setQueryTimeout(60000)

      expect(store.queryTimeout).toBe(60000)
    })
  })

  describe('setDefaultLimit', () => {
    it('should set default limit', () => {
      const store = useAppStore()
      store.setDefaultLimit(500)

      expect(store.defaultLimit).toBe(500)
    })
  })

  describe('setAutoSave', () => {
    it('should set auto save', () => {
      const store = useAppStore()
      store.setAutoSave(false)

      expect(store.autoSave).toBe(false)
    })
  })

  describe('getEditorTheme', () => {
    it('should return vs-light for light theme', () => {
      const store = useAppStore()
      store.uiThemeType = ThemeType.LIGHT

      expect(store.getEditorTheme()).toBe('vs-light')
    })

    it('should return vs-dark for dark theme', () => {
      const store = useAppStore()
      store.uiThemeType = ThemeType.DARK

      expect(store.getEditorTheme()).toBe('vs-dark')
    })
  })

  describe('toggleSidebar', () => {
    it('should toggle sidebar collapsed state', () => {
      const store = useAppStore()
      expect(store.sidebarCollapsed).toBe(false)

      store.toggleSidebar()
      expect(store.sidebarCollapsed).toBe(true)

      store.toggleSidebar()
      expect(store.sidebarCollapsed).toBe(false)
    })
  })

  describe('setSidebarCollapsed', () => {
    it('should set sidebar collapsed state directly', () => {
      const store = useAppStore()

      store.setSidebarCollapsed(true)
      expect(store.sidebarCollapsed).toBe(true)

      store.setSidebarCollapsed(false)
      expect(store.sidebarCollapsed).toBe(false)
    })
  })

  describe('updateQueryConfig', () => {
    it('should update query config partially', () => {
      const store = useAppStore()
      store.updateQueryConfig({ defaultLimit: 500 })

      expect(store.queryConfig.defaultLimit).toBe(500)
      expect(store.queryConfig.queryTimeout).toBe(30000) // unchanged
    })

    it('should update multiple query config values', () => {
      const store = useAppStore()
      store.updateQueryConfig({ defaultLimit: 2000, autoSave: false })

      expect(store.queryConfig.defaultLimit).toBe(2000)
      expect(store.queryConfig.autoSave).toBe(false)
    })
  })

  describe('llm initial state', () => {
    it('should have default providers', () => {
      const store = useAppStore()

      expect(store.llmSettings.providers).toHaveLength(10)
      expect(store.llmSettings.providers[0].id).toBe('openai')
      expect(store.llmSettings.providers[0].name).toBe('OpenAI')
      expect(store.llmSettings.providers[0].enabled).toBe(true)
    })
  })

  describe('addProvider', () => {
    it('should add a provider to the list', () => {
      const store = useAppStore()
      const initialCount = store.llmSettings.providers.length
      const newProvider = {
        id: 'test-1',
        name: 'Test Provider',
        apiCompatibility: 'openai',
        apiKey: 'sk-test',
        baseUrl: 'https://api.openai.com/v1',
        enabled: true,
        models: ['gpt-4o'],
      }

      store.addProvider(newProvider)

      expect(store.llmSettings.providers).toHaveLength(initialCount + 1)
      expect(store.llmSettings.providers.find(p => p.id === 'test-1')).toBeTruthy()
    })

    it('should not mutate the original array reference', () => {
      const store = useAppStore()
      const originalProviders = store.llmSettings.providers
      const newProvider = {
        id: 'test-immutable',
        name: 'Immutability Test',
        apiCompatibility: 'anthropic',
        apiKey: '',
        baseUrl: '',
        enabled: true,
      }

      store.addProvider(newProvider)

      expect(store.llmSettings.providers).not.toBe(originalProviders)
    })
  })

  describe('removeProvider', () => {
    it('should remove a provider by id', () => {
      const store = useAppStore()
      store.addProvider({
        id: 'to-remove',
        name: 'To Remove',
        apiCompatibility: 'openai',
        apiKey: '',
        baseUrl: '',
        enabled: true,
      })
      expect(store.llmSettings.providers.find(p => p.id === 'to-remove')).toBeTruthy()

      store.removeProvider('to-remove')

      expect(store.llmSettings.providers.find(p => p.id === 'to-remove')).toBeUndefined()
    })

    it('should no-op when id is not found', () => {
      const store = useAppStore()
      const initialCount = store.llmSettings.providers.length

      store.removeProvider('non-existent-id')

      expect(store.llmSettings.providers).toHaveLength(initialCount)
    })

    it('should return a new array reference', () => {
      const store = useAppStore()
      const originalProviders = store.llmSettings.providers

      store.removeProvider('non-existent-id')

      // removeProvider should produce a new array even when no-op
      expect(store.llmSettings.providers).not.toBe(originalProviders)
    })
  })

  describe('updateProvider', () => {
    it('should update partial fields of a provider', () => {
      const store = useAppStore()

      store.updateProvider('openai', { name: 'Updated OpenAI' })

      const provider = store.llmSettings.providers.find(p => p.id === 'openai')
      expect(provider?.name).toBe('Updated OpenAI')
      expect(provider?.apiCompatibility).toBe('openai') // unchanged
    })

    it('should update apiKey and baseUrl', () => {
      const store = useAppStore()

      store.updateProvider('anthropic', { apiKey: 'sk-ant-new', baseUrl: 'https://custom.anthropic.com' })

      const provider = store.llmSettings.providers.find(p => p.id === 'anthropic')
      expect(provider?.apiKey).toBe('sk-ant-new')
      expect(provider?.baseUrl).toBe('https://custom.anthropic.com')
    })

    it('should no-op when id is not found', () => {
      const store = useAppStore()
      const original = [...store.llmSettings.providers]

      store.updateProvider('non-existent', { name: 'Changed' })

      expect(store.llmSettings.providers).toEqual(original)
    })

    it('should not mutate the original provider object', () => {
      const store = useAppStore()
      const originalProvider = store.llmSettings.providers.find(p => p.id === 'openai')

      store.updateProvider('openai', { name: 'Changed' })

      const updatedProvider = store.llmSettings.providers.find(p => p.id === 'openai')
      expect(updatedProvider).not.toBe(originalProvider)
    })
  })

  describe('toggleProviderEnabled', () => {
    it('should toggle enabled from true to false', () => {
      const store = useAppStore()
      expect(store.llmSettings.providers.find(p => p.id === 'openai')?.enabled).toBe(true)

      store.toggleProviderEnabled('openai')

      expect(store.llmSettings.providers.find(p => p.id === 'openai')?.enabled).toBe(false)
    })

    it('should toggle enabled from false to true', () => {
      const store = useAppStore()
      expect(store.llmSettings.providers.find(p => p.id === 'anthropic')?.enabled).toBe(false)

      store.toggleProviderEnabled('anthropic')

      expect(store.llmSettings.providers.find(p => p.id === 'anthropic')?.enabled).toBe(true)
    })

    it('should no-op when id is not found', () => {
      const store = useAppStore()
      const original = [...store.llmSettings.providers]

      store.toggleProviderEnabled('non-existent')

      expect(store.llmSettings.providers).toEqual(original)
    })
  })

  describe('reorderProviders', () => {
    it('should replace the providers array with new order', () => {
      const store = useAppStore()
      const reversed = [...store.llmSettings.providers].reverse()

      store.reorderProviders(reversed)

      expect(store.llmSettings.providers[0].id).toBe('lm-studio')
      expect(store.llmSettings.providers[1].id).toBe('ollama')
    })

    it('should accept empty array', () => {
      const store = useAppStore()

      store.reorderProviders([])

      expect(store.llmSettings.providers).toEqual([])
    })
  })

  describe('getFeatureModelConfig', () => {
    it('should return first enabled provider and its first model', async () => {
      const store = useAppStore()

      const config = await store.getFeatureModelConfig('dataStudio')

      expect(config.provider.id).toBe('openai')
      expect(config.model.label).toBeTruthy()
    })

    it('should resolve with default provider when no stored settings', async () => {
      const store = useAppStore()

      const config = await store.getFeatureModelConfig('dataStudio')

      expect(config.provider).toBeTruthy()
      expect(config.provider.enabled).toBe(true)
      expect(config.model.label).toBeTruthy()
      expect(config.model.providerConfigId).toBe(config.provider.id)
    })
  })

  describe('verifyModelAvailability', () => {
    it('should return true for an enabled provider model', async () => {
      const store = useAppStore()

      const available = await store.verifyModelAvailability('gpt-4o')

      expect(available).toBe(true)
    })

    it('should return false for a non-existent model', async () => {
      const store = useAppStore()

      const available = await store.verifyModelAvailability('non-existent-model')

      expect(available).toBe(false)
    })

    it('should return false when the provider is disabled', async () => {
      const store = useAppStore()
      store.toggleProviderEnabled('openai')

      const available = await store.verifyModelAvailability('gpt-4o')

      expect(available).toBe(false)
    })
  })
})
