import { defineStore } from 'pinia'

export enum ThemeType {
  AUTO = 'auto',
  DARK = 'dark',
  LIGHT = 'light',
}

// System theme change listener state (managed outside Pinia for lifecycle control)
let systemThemeMediaQuery: MediaQueryList | null = null
let systemThemeHandler: ((e: MediaQueryListEvent) => void) | null = null

export enum LanguageType {
  AUTO = 'auto',
  ZH_CN = 'zhCN',
  EN_US = 'enUS',
}

export type EditorConfig = {
  fontSize: number
  fontWeight: string
  fontFamily: string
  tabSize: number
  wordWrap: boolean
  showLineNumbers: boolean
  showMinimap: boolean
}

export type QueryConfig = {
  defaultLimit: number
  queryTimeout: number
  autoSave: boolean
}

export type LlmProvider = {
  id: string
  name: string
  apiCompatibility: string
  apiKey: string
  baseUrl: string
  enabled: boolean
  proxy?: string
  proxyMode?: string
  contextWindowOverride?: number
  models?: string[]
}

type AppStoreState = {
  themeType: ThemeType
  languageType: LanguageType
  uiThemeType: Exclude<ThemeType, ThemeType.AUTO>
  editorConfig: EditorConfig
  queryConfig: QueryConfig
  sidebarCollapsed: boolean
  llmSettings: {
    providers: LlmProvider[]
  }
  featureModelRoutes: Record<string, { selectedModelId: string, useRecommendedModel: boolean }>
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
    llmSettings: {
      providers: [
        {
          id: 'openai',
          name: 'OpenAI',
          apiCompatibility: 'openai',
          apiKey: '',
          baseUrl: '',
          enabled: true,
          models: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo'],
        },
        {
          id: 'anthropic',
          name: 'Anthropic',
          apiCompatibility: 'anthropic',
          apiKey: '',
          baseUrl: '',
          enabled: false,
          models: ['claude-sonnet-4-20250514', 'claude-3-5-sonnet-20241022'],
        },
      ],
    },
    featureModelRoutes: {},
  }),
  persist: true,
  getters: {
    queryTimeout: (state): number => state.queryConfig.queryTimeout,
    defaultLimit: (state): number => state.queryConfig.defaultLimit,
    autoSave: (state): boolean => state.queryConfig.autoSave,
  },
  actions: {
    applyUiTheme(uiThemeType: Exclude<ThemeType, ThemeType.AUTO>) {
      document.documentElement.setAttribute('theme', uiThemeType)
      if (uiThemeType === ThemeType.DARK) {
        document.documentElement.classList.add('dark')
      }
      else {
        document.documentElement.classList.remove('dark')
      }
      this.uiThemeType = uiThemeType
    },

    updateSystemThemeWatcher() {
      if (systemThemeHandler && systemThemeMediaQuery) {
        systemThemeMediaQuery.removeEventListener('change', systemThemeHandler)
        systemThemeHandler = null
        systemThemeMediaQuery = null
      }

      if (this.themeType === ThemeType.AUTO) {
        systemThemeMediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
        systemThemeHandler = (e: MediaQueryListEvent) => {
          const uiThemeType = e.matches ? ThemeType.DARK : ThemeType.LIGHT
          this.applyUiTheme(uiThemeType)
        }
        systemThemeMediaQuery.addEventListener('change', systemThemeHandler)
      }
    },

    /**
     * Cleanup theme listener when store is destroyed
     * Call this on app unmount to prevent memory leaks
     */
    cleanup() {
      if (systemThemeHandler && systemThemeMediaQuery) {
        systemThemeMediaQuery.removeEventListener('change', systemThemeHandler)
        systemThemeHandler = null
        systemThemeMediaQuery = null
      }
    },

    setThemeType(themeType: ThemeType) {
      const uiThemeType = themeType === ThemeType.AUTO
        ? window.matchMedia('(prefers-color-scheme: dark)').matches
          ? ThemeType.DARK
          : ThemeType.LIGHT
        : themeType

      this.themeType = themeType
      this.applyUiTheme(uiThemeType)
      this.updateSystemThemeWatcher()
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

    // ── Provider CRUD ────────────────────────────────────────────────────

    addProvider(provider: LlmProvider) {
      this.llmSettings = {
        ...this.llmSettings,
        providers: [...this.llmSettings.providers, provider],
      }
    },

    removeProvider(providerId: string) {
      this.llmSettings = {
        ...this.llmSettings,
        providers: this.llmSettings.providers.filter(p => p.id !== providerId),
      }
    },

    updateProvider(providerId: string, updates: Partial<LlmProvider>) {
      this.llmSettings = {
        ...this.llmSettings,
        providers: this.llmSettings.providers.map(p =>
          p.id === providerId ? { ...p, ...updates } : p,
        ),
      }
    },

    toggleProviderEnabled(providerId: string) {
      this.llmSettings = {
        ...this.llmSettings,
        providers: this.llmSettings.providers.map(p =>
          p.id === providerId ? { ...p, enabled: !p.enabled } : p,
        ),
      }
    },

    reorderProviders(providers: LlmProvider[]) {
      this.llmSettings = {
        ...this.llmSettings,
        providers,
      }
    },

    // ── LLM/Agent configuration ──────────────────────────────────────────

    async getFeatureModelConfig(feature: string): Promise<{ provider: LlmProvider, model: { label: string } }> {
      const route = this.featureModelRoutes[feature]
      const enabled = this.llmSettings.providers.filter(p => p.enabled)

      // When a specific model is selected (not "recommended"), find the provider that owns it
      if (route?.selectedModelId && !route?.useRecommendedModel) {
        const owner = enabled.find(p => (p.models ?? []).includes(route.selectedModelId))
        if (owner) {
          return { provider: owner, model: { label: route.selectedModelId } }
        }
      }

      // Fallback: use first enabled provider's first model
      const provider = enabled[0] || this.llmSettings.providers[0]
      const modelId = (provider.models ?? ['gpt-4o'])[0]

      return {
        provider,
        model: { label: modelId },
      }
    },

    async setFeatureModelRoute(feature: string, route: { selectedModelId: string, useRecommendedModel: boolean }) {
      this.featureModelRoutes = {
        ...this.featureModelRoutes,
        [feature]: route,
      }
    },

    async syncProviderModels(providerId: string) {
      const idx = this.llmSettings.providers.findIndex(p => p.id === providerId)
      if (idx === -1)
        return
      const provider = this.llmSettings.providers[idx]
      if (!provider.apiKey)
        return

      try {
        const { invoke } = await import('@tauri-apps/api/core')
        const models = await invoke<string[]>('list_llm_models', {
          provider: provider.apiCompatibility,
          apiKey: provider.apiKey,
          baseUrl: provider.baseUrl || null,
        })
        if (models.length > 0) {
          const updated = [...this.llmSettings.providers]
          updated[idx] = { ...updated[idx], models }
          this.llmSettings = { ...this.llmSettings, providers: updated }
        }
      }
      catch {
        // Silently fail - models remain as defaults
      }
    },

    async testProviderConnection(providerId: string): Promise<{ success: boolean, error?: string }> {
      const provider = this.llmSettings.providers.find(p => p.id === providerId)
      if (!provider) {
        return { success: false, error: 'Provider not found' }
      }
      if (!provider.apiKey) {
        return { success: false, error: 'API key is not configured' }
      }

      try {
        const { invoke } = await import('@tauri-apps/api/core')
        const valid = await invoke<boolean>('validate_llm_config', {
          provider: provider.apiCompatibility,
          apiKey: provider.apiKey,
          model: (provider.models ?? ['gpt-4o'])[0],
          httpProxy: provider.proxy || null,
          proxyMode: provider.proxyMode || 'none',
          baseUrl: provider.baseUrl || null,
        })
        return { success: valid }
      }
      catch (err) {
        return { success: false, error: String(err) }
      }
    },

    async verifyModelAvailability(modelId: string): Promise<boolean> {
      return this.llmSettings.providers.some(
        p => p.enabled && (p.models ?? []).includes(modelId),
      )
    },
  },
})
