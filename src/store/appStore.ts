import { defineStore } from 'pinia'
import { storeApi } from '@/datasources/storeApi'

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
  tabSize: number
  wordWrap: boolean
  showLineNumbers: boolean
  showMinimap: boolean
  indentWidth: number
  lineWidth: number
}

export type QueryConfig = {
  defaultLimit: number
  queryTimeout: number
  autoSave: boolean
}

export type LlmProvider = {
  id: string
  kind: string
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

export type ChatRuntimeConfig = {
  autoCompact: boolean
  maxIterations: number
  wallClockBudgetMin: number
  tokenBudget: number
}

export type ModelRef = {
  id: string
  label: string
  providerKind: string
  providerConfigId: string
}

export type FeatureModelRoute = {
  selectedModelId?: string | null
  preferredCategory?: 'general' | 'reasoning' | 'coding' | null
  useRecommendedModel?: boolean
}

const CHAT_RUNTIME_DEFAULTS: ChatRuntimeConfig = {
  autoCompact: true,
  maxIterations: 200,
  wallClockBudgetMin: 30,
  tokenBudget: 20_000_000,
}

function createModelRef(providerId: string, providerKind: string, modelLabel: string): ModelRef {
  return {
    id: `${providerId}::${modelLabel}`,
    label: modelLabel,
    providerKind,
    providerConfigId: providerId,
  }
}

function normalizeFeatureRoute(route: FeatureModelRoute | undefined, providers: LlmProvider[], fallbackCategory: 'general' | 'reasoning' | 'coding'): FeatureModelRoute {
  const selectedModelId = route?.selectedModelId ?? null
  const selectedExists = selectedModelId === null
    ? false
    : providers.some(p =>
        (p.models ?? []).some(modelId => `${p.id}::${modelId}` === selectedModelId),
      )
  return {
    selectedModelId: selectedExists ? selectedModelId : null,
    preferredCategory: route?.preferredCategory ?? fallbackCategory,
    useRecommendedModel: route?.useRecommendedModel ?? !selectedExists,
  }
}

function resolveRecommendedModelId(providers: LlmProvider[], _route: FeatureModelRoute): string | null {
  const enabled = providers.filter(p => p.enabled)
  for (const p of enabled) {
    const firstModel = (p.models ?? [])[0]
    if (firstModel)
      return `${p.id}::${firstModel}`
  }
  return null
}

function reconcileModelRoutes(settings: {
  providers: LlmProvider[]
  models: { sidebarAssistant: FeatureModelRoute, dataStudio: FeatureModelRoute }
}): { sidebarAssistant: FeatureModelRoute, dataStudio: FeatureModelRoute } {
  return {
    sidebarAssistant: normalizeFeatureRoute(settings.models.sidebarAssistant, settings.providers, 'general'),
    dataStudio: normalizeFeatureRoute(settings.models.dataStudio, settings.providers, 'reasoning'),
  }
}

function mergeLlmSettings(stored: {
  providers?: LlmProvider[]
  models?: { sidebarAssistant?: FeatureModelRoute, dataStudio?: FeatureModelRoute }
  chat?: ChatRuntimeConfig
}): {
  providers: LlmProvider[]
  models: { sidebarAssistant: FeatureModelRoute, dataStudio: FeatureModelRoute }
  chat: ChatRuntimeConfig
} {
  const providers = stored.providers ?? []
  return {
    providers,
    models: {
      sidebarAssistant: normalizeFeatureRoute(
        stored.models?.sidebarAssistant,
        providers,
        'general',
      ),
      dataStudio: normalizeFeatureRoute(
        stored.models?.dataStudio,
        providers,
        'reasoning',
      ),
    },
    chat: {
      autoCompact: stored.chat?.autoCompact ?? CHAT_RUNTIME_DEFAULTS.autoCompact,
      maxIterations: stored.chat?.maxIterations ?? CHAT_RUNTIME_DEFAULTS.maxIterations,
      wallClockBudgetMin: stored.chat?.wallClockBudgetMin ?? CHAT_RUNTIME_DEFAULTS.wallClockBudgetMin,
      tokenBudget: stored.chat?.tokenBudget ?? CHAT_RUNTIME_DEFAULTS.tokenBudget,
    },
  }
}

function defaultModelRoutes(): { sidebarAssistant: FeatureModelRoute, dataStudio: FeatureModelRoute } {
  return {
    sidebarAssistant: {
      selectedModelId: null,
      preferredCategory: 'general',
      useRecommendedModel: true,
    },
    dataStudio: {
      selectedModelId: null,
      preferredCategory: 'reasoning',
      useRecommendedModel: true,
    },
  }
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
    models: {
      sidebarAssistant: FeatureModelRoute
      dataStudio: FeatureModelRoute
    }
    chat: ChatRuntimeConfig
  }
}

export const useAppStore = defineStore('app', {
  state: (): AppStoreState => ({
    themeType: ThemeType.AUTO,
    languageType: LanguageType.AUTO,
    uiThemeType: ThemeType.LIGHT,
    editorConfig: {
      fontSize: 14,
      tabSize: 2,
      wordWrap: true,
      showLineNumbers: true,
      showMinimap: false,
      indentWidth: 2,
      lineWidth: 120,
    },
    queryConfig: {
      defaultLimit: 1000,
      queryTimeout: 30000,
      autoSave: true,
    },
    sidebarCollapsed: false,
    llmSettings: {
      providers: [],
      models: defaultModelRoutes(),
      chat: { ...CHAT_RUNTIME_DEFAULTS },
    },
  }),
  persist: true,
  getters: {
    queryTimeout: (state): number => state.queryConfig.queryTimeout,
    defaultLimit: (state): number => state.queryConfig.defaultLimit,
    autoSave: (state): boolean => state.queryConfig.autoSave,
    availableModels(state): ModelRef[] {
      return state.llmSettings.providers.flatMap(p =>
        (p.models ?? []).map(modelId => createModelRef(p.id, p.kind, modelId)),
      )
    },
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

    async fetchLlmSettings() {
      const storedSettings = await storeApi.getSecret<{
        providers?: LlmProvider[]
        models?: { sidebarAssistant?: FeatureModelRoute, dataStudio?: FeatureModelRoute }
        chat?: ChatRuntimeConfig
      } | undefined>('llmSettings', undefined)
      if (storedSettings) {
        this.llmSettings = mergeLlmSettings(storedSettings)
        return
      }
      await storeApi.setSecret('llmSettings', this.llmSettings)
    },

    async persistLlmSettings() {
      this.llmSettings.models = reconcileModelRoutes(this.llmSettings)
      await storeApi.setSecret('llmSettings', this.llmSettings)
    },

    async setFeatureModelRoute(
      feature: 'sidebarAssistant' | 'dataStudio',
      route: Partial<FeatureModelRoute>,
    ) {
      this.llmSettings.models[feature] = {
        ...this.llmSettings.models[feature],
        ...route,
      }
      await this.persistLlmSettings()
    },

    getResolvedFeatureModel(feature: string): { provider: LlmProvider, model: ModelRef } | undefined {
      const route = this.llmSettings.models[feature as keyof typeof this.llmSettings.models]
      if (!route)
        return undefined
      const selectedModel = route.selectedModelId
        ? this.availableModels.find(m => m.id === route.selectedModelId)
        : undefined

      const modelId = route.useRecommendedModel || !selectedModel
        ? resolveRecommendedModelId(this.llmSettings.providers, route)
        : selectedModel.id

      if (!modelId)
        return undefined

      const resolvedModel = this.availableModels.find(m => m.id === modelId)
      if (!resolvedModel)
        return undefined

      const provider = this.llmSettings.providers.find(
        p => p.id === resolvedModel.providerConfigId && p.enabled,
      )
      if (!provider)
        return undefined

      return { provider, model: resolvedModel }
    },

    async getFeatureModelConfig(feature: 'sidebarAssistant' | 'dataStudio'): Promise<{ provider: LlmProvider, model: ModelRef }> {
      await this.fetchLlmSettings()
      const resolved = this.getResolvedFeatureModel(feature)
      if (!resolved)
        throw new Error('No LLM provider configured. Please configure an AI provider in Settings.')
      return resolved
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
          httpProxy: provider.proxy || null,
          proxyMode: provider.proxyMode || 'none',
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

    // ── Agent Chat Config ───────────────────────────────────────────────

    setAutoCompact(autoCompact: boolean) {
      this.llmSettings = {
        ...this.llmSettings,
        chat: { ...this.llmSettings.chat, autoCompact },
      }
    },

    setMaxIterations(maxIterations: number) {
      this.llmSettings = {
        ...this.llmSettings,
        chat: { ...this.llmSettings.chat, maxIterations },
      }
    },

    setWallClockBudgetMin(wallClockBudgetMin: number) {
      this.llmSettings = {
        ...this.llmSettings,
        chat: { ...this.llmSettings.chat, wallClockBudgetMin },
      }
    },

    setTokenBudget(tokenBudget: number) {
      this.llmSettings = {
        ...this.llmSettings,
        chat: { ...this.llmSettings.chat, tokenBudget },
      }
    },
  },
})
