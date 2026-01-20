import { defineStore } from 'pinia'

export enum ThemeType {
  AUTO = 'auto',
  DARK = 'dark',
  LIGHT = 'light',
}

export enum LanguageType {
  AUTO = 'auto',
  ZH_CN = 'zhCN',
  EN_US = 'enUS',
}

export interface EditorConfig {
  fontSize: number
  fontWeight: string
  fontFamily: string
  tabSize: number
  wordWrap: boolean
  showLineNumbers: boolean
  showMinimap: boolean
}

export interface QueryConfig {
  defaultLimit: number
  queryTimeout: number
  autoSave: boolean
}

interface AppStoreState {
  themeType: ThemeType
  languageType: LanguageType
  uiThemeType: Exclude<ThemeType, ThemeType.AUTO>
  editorConfig: EditorConfig
  queryConfig: QueryConfig
  sidebarCollapsed: boolean
}

export const useAppStore = defineStore('app', {
  state: (): AppStoreState => ({
    themeType: ThemeType.AUTO,
    languageType: LanguageType.AUTO,
    uiThemeType: ThemeType.LIGHT,
    editorConfig: {
      fontSize: 14,
      fontWeight: 'normal',
      fontFamily: 'Monaco, Menlo, Consolas, monospace',
      tabSize: 2,
      wordWrap: true,
      showLineNumbers: true,
      showMinimap: false,
    },
    queryConfig: {
      defaultLimit: 1000,
      queryTimeout: 30000,
      autoSave: true,
    },
    sidebarCollapsed: false,
  }),
  persist: true,
  getters: {
    queryTimeout: (state): number => state.queryConfig.queryTimeout,
    defaultLimit: (state): number => state.queryConfig.defaultLimit,
    autoSave: (state): boolean => state.queryConfig.autoSave,
  },
  actions: {
    setThemeType(themeType: ThemeType) {
      const uiThemeType = themeType === ThemeType.AUTO
        ? window.matchMedia('(prefers-color-scheme: light)').matches
          ? ThemeType.LIGHT
          : ThemeType.DARK
        : themeType
      document.documentElement.setAttribute('theme', uiThemeType)

      // Also update the dark class for UnoCSS dark mode support
      if (uiThemeType === ThemeType.DARK) {
        document.documentElement.classList.add('dark')
      }
      else {
        document.documentElement.classList.remove('dark')
      }

      this.uiThemeType = uiThemeType
      this.themeType = themeType
    },
    setLanguageType(languageType: LanguageType) {
      this.languageType = languageType
    },
    setEditorConfig(config: Partial<EditorConfig>) {
      this.editorConfig = { ...this.editorConfig, ...config }
    },
    setQueryTimeout(timeout: number) {
      this.queryConfig.queryTimeout = timeout
    },
    setDefaultLimit(limit: number) {
      this.queryConfig.defaultLimit = limit
    },
    setAutoSave(autoSave: boolean) {
      this.queryConfig.autoSave = autoSave
    },
    updateQueryConfig(config: Partial<QueryConfig>) {
      this.queryConfig = { ...this.queryConfig, ...config }
    },
    toggleSidebar() {
      this.sidebarCollapsed = !this.sidebarCollapsed
    },
    setSidebarCollapsed(collapsed: boolean) {
      this.sidebarCollapsed = collapsed
    },
    getEditorTheme() {
      return this.uiThemeType === ThemeType.DARK ? 'vs-dark' : 'vs-light'
    },
  },
})
