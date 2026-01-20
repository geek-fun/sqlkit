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
  showLineNumbers: boolean
  showMinimap: boolean
}

export const useAppStore = defineStore('app', {
  state: () => ({
    themeType: ThemeType.AUTO,
    languageType: LanguageType.AUTO,
    uiThemeType: ThemeType.LIGHT as Exclude<ThemeType, ThemeType.AUTO>,
    editorConfig: {
      fontSize: 14,
      fontWeight: 'normal',
      showLineNumbers: true,
      showMinimap: false,
    } as EditorConfig,
    queryTimeout: 30000,
    defaultLimit: 1000,
    autoSave: true,
  }),
  persist: true,
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
      this.queryTimeout = timeout
    },
    setDefaultLimit(limit: number) {
      this.defaultLimit = limit
    },
    setAutoSave(autoSave: boolean) {
      this.autoSave = autoSave
    },
    getEditorTheme() {
      return this.uiThemeType === ThemeType.DARK ? 'vs-dark' : 'vs-light'
    },
  },
})
