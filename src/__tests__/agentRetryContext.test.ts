import type { ChatSession } from '@/types/chat'
import { computed, reactive, ref } from 'vue'
import { useChatAgent } from '@/composables/useChatAgent'
import { agentApi } from '@/datasources/agentApi'

jest.mock('@/common', () => jest.requireActual('@/common/agentPrompt'))
jest.mock('@/datasources/agentApi', () => ({ agentApi: { getAvailableTools: jest.fn(), getAiContext: jest.fn(), runAgentLoop: jest.fn() } }))
jest.mock('@/store/connectionStore', () => ({ useConnectionStore: () => ({ getConnectionById: () => undefined }) }))
jest.mock('@/store/entitlementStore', () => ({ useEntitlementStore: () => ({ isLocalUltimate: false }) }))
jest.mock('@/store/appStore', () => ({ useAppStore: () => ({
  llmSettings: { chat: {} },
  getFeatureModelConfig: async () => ({ provider: { name: 'synthetic', apiKey: 'synthetic', apiCompatibility: 'openai-compatible' }, model: { label: 'synthetic' } }),
}) }))
jest.mock('@/store/dataStudioStore', () => ({ useDataStudioStore: () => ({ clearSessionError: jest.fn(), insertPreparingPlaceholder: jest.fn(), setSessionProgress: jest.fn(), getSessionError: () => undefined }) }))

describe('agent retry uses current backend permissions', () => {
  it.each([true, false])('uses current Manage=%s instead of the previous frontend snapshot', async (manage) => {
    const session = reactive<ChatSession>({ id: 'chat', messages: [], status: 'blocked', maxIterations: 3 })
    jest.mocked(agentApi.getAvailableTools).mockResolvedValue({ tools: [], metadata: {} })
    jest.mocked(agentApi.getAiContext).mockResolvedValue({
      readableConnectionIds: ['42'],
      schema: '',
      effectivePermissions: { example: { read: true, create: true, update: true, delete: true, manage } },
    })
    jest.mocked(agentApi.runAgentLoop).mockClear()
    jest.mocked(agentApi.runAgentLoop).mockResolvedValue(undefined)
    const agent = useChatAgent({
      feature: 'dataStudio',
      autoMode: ref(true),
      sessionStore: {
        sessions: ref([session]),
        activeSession: computed(() => session),
        activeSessionId: ref('chat'),
        addMessage: jest.fn(),
        updateStreamingContent: jest.fn(),
        updateStreamingThinking: jest.fn(),
        setMessageStatus: jest.fn(),
        setMessageToolCalls: jest.fn(),
        removeOrphanedStreamingMessages: jest.fn(),
        updateToolCallStatus: jest.fn(),
        setSessionStatus: (_id, status) => { session.status = status },
        setSessionSchema: jest.fn(),
        clearSession: jest.fn(),
        getOrCreateSession: async () => 'chat',
        reloadSessionMessages: jest.fn(),
      },
    })
    await agent.sendMessage({ content: 'retry', context: { connections: { example: {
      connectionId: '42',
      dbType: 'POSTGRESQL',
      permissions: { read: true, create: false, update: false, delete: false, manage: !manage },
    } } } })
    expect(agentApi.runAgentLoop).toHaveBeenCalledTimes(1)
    const settings = jest.mocked(agentApi.runAgentLoop).mock.calls[0][2] as Record<string, unknown>
    expect(settings.systemPrompt).toContain(manage ? 'allowed actions: read, create, update, delete, manage; denied actions: none' : 'allowed actions: read, create, update, delete; denied actions: manage')
    expect(settings.systemPrompt).toContain('Never claim a new attempt failed without a new tool result from this turn')
  })
})
