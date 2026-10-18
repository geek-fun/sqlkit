/**
 * @jest-environment jsdom
 * @jest-environment-options {"customExportConditions": ["node", "node-addons"]}
 */
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { storeApi } from '@/datasources/storeApi'
import { enUS } from '@/lang/enUS'
import AiSettings from '@/views/setting/ai-settings.vue'

jest.mock('@tauri-apps/plugin-opener', () => ({ openUrl: jest.fn() }))
jest.mock('@/store/entitlementStore', () => ({
  useEntitlementStore: () => ({ isLocalUltimate: true }),
}))
jest.mock('@/datasources/storeApi', () => ({
  storeApi: { getSecret: jest.fn(), setSecret: jest.fn() },
}))
jest.mock('@/components/upgrade', () => ({ ProBadge: { template: '<span />' } }))
jest.mock('@/components/ui/button', () => ({
  Button: { template: '<button><slot /></button>' },
}))
jest.mock('@/components/ui/card', () => Object.fromEntries(
  ['Card', 'CardContent', 'CardDescription', 'CardHeader', 'CardTitle']
    .map(name => [name, { template: '<div><slot /></div>' }]),
))
jest.mock('@/components/ui/checkbox', () => ({
  Checkbox: { template: '<input type="checkbox" />' },
}))
jest.mock('@/components/ui/input', () => ({ Input: { template: '<input />' } }))
jest.mock('@/views/setting/provider-list.vue', () => ({
  default: { template: '<div data-testid="provider-list" />' },
}))
jest.mock('@/views/setting/provider-form-dialog.vue', () => ({
  default: { template: '<div data-testid="provider-form" />' },
}))

const savedSettings = () => ({
  providers: [{
    id: 'provider',
    kind: 'openai',
    name: 'OpenAI',
    apiCompatibility: 'openai',
    apiKey: 'saved-api-key',
    baseUrl: 'https://example.com/v1',
    enabled: true,
    models: ['model'],
  }],
  models: {
    sidebarAssistant: { selectedModelId: 'provider::model', preferredCategory: 'general', useRecommendedModel: false },
    dataStudio: { selectedModelId: 'provider::model', preferredCategory: 'reasoning', useRecommendedModel: false },
  },
  chat: {
    autoCompact: false,
    maxIterations: 57,
    wallClockBudgetMin: 12,
    tokenBudget: 42_000,
    systemPrompt: 'Revenue is stored in cents.',
  },
})

let persisted: ReturnType<typeof savedSettings>
const render = () => mount(AiSettings, {
  global: {
    plugins: [createI18n({ legacy: false, locale: 'enUS', messages: { enUS } })],
  },
})
const button = (wrapper: ReturnType<typeof render>, label: string) => {
  const found = wrapper.findAll('button').find(button => button.text() === label)
  expect(found).toBeDefined()
  return found!
}

beforeEach(() => {
  setActivePinia(createPinia())
  jest.resetAllMocks()
  persisted = savedSettings()
  jest.mocked(storeApi.getSecret).mockImplementation(async () => persisted as never)
  jest.mocked(storeApi.setSecret).mockImplementation(async (_key, value) => {
    persisted = JSON.parse(JSON.stringify(value))
  })
})

it('loads the saved prompt when settings is opened before either chat surface', async () => {
  const wrapper = render()
  await flushPromises()

  expect(storeApi.getSecret).toHaveBeenCalledWith('llmSettings', undefined)
  expect(wrapper.text()).toContain(persisted.chat.systemPrompt)
  await button(wrapper, enUS.pages.settings.ai.chat.systemPromptEdit).trigger('click')
  expect(wrapper.get('textarea').element.value).toBe(persisted.chat.systemPrompt)
  expect(storeApi.setSecret).not.toHaveBeenCalled()
})

it.each(['', 'Revenue is stored in cents.'])(
  'preserves providers, credentials, model routes, and preferences when saving over %p',
  async (systemPrompt) => {
    persisted.chat.systemPrompt = systemPrompt
    const original = savedSettings()
    const wrapper = render()
    await flushPromises()

    await button(wrapper, systemPrompt
      ? enUS.pages.settings.ai.chat.systemPromptEdit
      : enUS.pages.settings.ai.chat.systemPromptAdd).trigger('click')
    await wrapper.get('textarea').setValue('Our fiscal year starts in July.')
    await button(wrapper, enUS.pages.settings.ai.chat.systemPromptSave).trigger('click')
    await flushPromises()

    expect(persisted).toEqual({
      ...original,
      chat: { ...original.chat, systemPrompt: 'Our fiscal year starts in July.' },
    })
  },
)

it('withholds editing controls until settings finish loading', async () => {
  let finishLoading!: (settings: typeof persisted) => void
  jest.mocked(storeApi.getSecret).mockReturnValueOnce(new Promise((resolve) => {
    finishLoading = resolve
  }) as never)
  const wrapper = render()

  expect(wrapper.get('[role="status"]').text()).toContain(enUS.pages.settings.feedback.loading)
  expect(wrapper.find('button').exists()).toBe(false)
  expect(wrapper.find('input').exists()).toBe(false)
  expect(wrapper.find('[data-testid="provider-list"]').exists()).toBe(false)
  expect(wrapper.find('[data-testid="provider-form"]').exists()).toBe(false)
  expect(storeApi.setSecret).not.toHaveBeenCalled()

  finishLoading(persisted)
  await flushPromises()
  expect(wrapper.text()).toContain(persisted.chat.systemPrompt)
  expect(wrapper.find('[data-testid="provider-list"]').exists()).toBe(true)
})

it('keeps settings unavailable after a load failure and lets the user retry', async () => {
  jest.mocked(storeApi.getSecret).mockRejectedValueOnce(new Error('Store unavailable'))
  const wrapper = render()
  await flushPromises()

  expect(wrapper.get('[role="alert"]').text()).toContain('Store unavailable')
  expect(wrapper.findAll('button')).toHaveLength(1)
  expect(wrapper.find('input').exists()).toBe(false)
  expect(wrapper.find('[data-testid="provider-list"]').exists()).toBe(false)
  expect(storeApi.setSecret).not.toHaveBeenCalled()

  await button(wrapper, enUS.pages.settings.feedback.retry).trigger('click')
  await flushPromises()
  expect(storeApi.getSecret).toHaveBeenCalledTimes(2)
  expect(wrapper.find('[role="alert"]').exists()).toBe(false)
  expect(wrapper.text()).toContain(persisted.chat.systemPrompt)
  expect(wrapper.find('[data-testid="provider-list"]').exists()).toBe(true)
  expect(storeApi.setSecret).not.toHaveBeenCalled()
})
