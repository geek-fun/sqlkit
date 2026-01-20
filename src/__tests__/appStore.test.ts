import { createPinia, setActivePinia } from 'pinia'
import { LanguageType, ThemeType, useAppStore } from '../store/appStore'

// Mock window and document for Node.js environment
const mockMatchMedia = jest.fn().mockReturnValue({ matches: false })
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
        fontWeight: 'normal',
        showLineNumbers: true,
        showMinimap: false,
      })
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

    it('should set theme to AUTO and detect system preference (light)', () => {
      mockMatchMedia.mockReturnValue({ matches: true }) // prefers light
      const store = useAppStore()
      store.setThemeType(ThemeType.AUTO)

      expect(store.themeType).toBe(ThemeType.AUTO)
      expect(store.uiThemeType).toBe(ThemeType.LIGHT)
    })

    it('should set theme to AUTO and detect system preference (dark)', () => {
      mockMatchMedia.mockReturnValue({ matches: false }) // prefers dark
      const store = useAppStore()
      store.setThemeType(ThemeType.AUTO)

      expect(store.themeType).toBe(ThemeType.AUTO)
      expect(store.uiThemeType).toBe(ThemeType.DARK)
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
      expect(store.editorConfig.fontWeight).toBe('normal') // unchanged
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
})
