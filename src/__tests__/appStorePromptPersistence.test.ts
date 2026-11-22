import { createPinia, setActivePinia } from 'pinia'
import { storeApi } from '@/datasources/storeApi'
import { useAppStore } from '@/store/appStore'

jest.mock('@/datasources/storeApi', () => ({
  storeApi: {
    getSecret: jest.fn(),
    setSecret: jest.fn(),
  },
}))

describe('global system prompt persistence', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    jest.clearAllMocks()
  })

  it('round-trips the saved prompt when AI settings are loaded again', async () => {
    let persisted: unknown
    jest.mocked(storeApi.getSecret).mockImplementation(async () => persisted as never)
    jest.mocked(storeApi.setSecret).mockImplementation(async (_key, value) => {
      persisted = value
    })

    const appStore = useAppStore()
    await appStore.saveChatSettings({ systemPrompt: 'Our fiscal year starts in July.' })
    setActivePinia(createPinia())
    const reopenedAppStore = useAppStore()
    await reopenedAppStore.fetchLlmSettings()

    expect(reopenedAppStore.llmSettings.chat.systemPrompt).toBe('Our fiscal year starts in July.')
  })
})
