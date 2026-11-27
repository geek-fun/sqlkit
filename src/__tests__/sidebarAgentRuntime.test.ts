import type { EventCallback } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { createPinia, setActivePinia } from 'pinia'
import { disposeAgentRuntime } from '@/composables/agentRuntime'
import { useSidebarChatAgent } from '@/composables/useSidebarChatAgent'
import { useDataStudioStore } from '@/store/dataStudioStore'

jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}))
jest.mock('@tauri-apps/api/event', () => ({
  listen: jest.fn(),
}))
jest.mock('@/common', () => ({
  ...jest.requireActual('@/common/agentPrompt'),
  ...jest.requireActual('@/common/toolCallPresentation'),
}))
jest.mock('@/store/connectionStore', () => ({
  useConnectionStore: () => ({
    getConnectionById: () => undefined,
  }),
}))
jest.mock('@/store/entitlementStore', () => ({
  useEntitlementStore: () => ({ isLocalUltimate: true }),
}))
jest.mock('@/store/tabStore', () => ({
  useTabStore: () => ({}),
}))
const mockGetFeatureModelConfig = jest.fn(async () => ({
  provider: { name: 'test-provider' },
  model: { label: 'test-model' },
}))
jest.mock('@/store/appStore', () => ({
  useAppStore: () => ({
    llmSettings: {
      chat: { maxIterations: 3 },
    },
    getFeatureModelConfig: mockGetFeatureModelConfig,
  }),
}))

const sessionRow = {
  id: 'sidebar',
  title: 'AI Assistant',
  status: 'idle',
  created_at: '0',
  updated_at: '0',
}
const listeners = new Map<string, Set<EventCallback<unknown>>>()
const emit = (event: string, payload: object) => {
  listeners.get(event)?.forEach(handler => handler({ event, id: 0, payload }))
}

describe('sidebar assistant event delivery', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    jest.clearAllMocks()
    jest.mocked(listen).mockImplementation(async (event, handler) => {
      const handlers = listeners.get(event) ?? new Set()
      listeners.set(event, handlers)
      handlers.add(handler)
      return () => handlers.delete(handler)
    })
  })

  afterEach(() => {
    disposeAgentRuntime()
    listeners.clear()
  })

  it.each(['reply', 'error'] as const)('handles a model %s on the first turn without opening Data Studio', async (outcome) => {
    jest.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'create_agent_session')
        return sessionRow as never
      if (command === 'get_available_tools')
        return { tools: [], metadata: {} } as never
      if (command === 'get_ai_context')
        return { schema: '', effectivePermissions: {}, readableConnectionIds: [] } as never
      if (command === 'confirm_tool_call')
        return undefined as never
      if (command === 'run_agent_loop') {
        const session_id = sessionRow.id
        emit('agent-loop-waiting-llm', { session_id, iter_count: 1 })
        if (outcome === 'error') {
          emit('agent-loop-error', { session_id, error: 'Model request failed' })
        }
        else {
          emit('agent-loop-tool-call', {
            session_id,
            tool_call_id: 'read',
            tool_name: 'sqlkit__list_tables',
            confirmation_required: false,
          })
          expect(invoke).toHaveBeenCalledWith('confirm_tool_call', {
            toolCallId: 'read',
            allowed: true,
          })
          emit('agent-loop-tool-result', {
            session_id,
            tool_call_id: 'read',
            envelope: { summary: 'Synthetic result' },
          })
          emit('agent-loop-delta', { session_id, content: 'The answer is 42.' })
          emit('agent-loop-done', { session_id })
        }
        return undefined as never
      }
      throw new Error(`Unexpected command ${command}`)
    })

    const assistant = useSidebarChatAgent()
    await assistant.sendMessage('Summarize the data')
    expect(useDataStudioStore().activeSessionId).toBeUndefined()
    expect(assistant.isLoading.value).toBe(false)
    expect(assistant.progress.value).toBeUndefined()
    expect(assistant.messages.value.some(message => message.preparingInProgress)).toBe(false)
    if (outcome === 'reply') {
      expect(assistant.messages.value).toContainEqual(expect.objectContaining({
        role: 'assistant',
        content: 'The answer is 42.',
        status: 'done',
      }))
    }
    else {
      expect(assistant.error.value).toBe('Model request failed')
      expect(assistant.activeSession.value?.status).toBe('error')
    }
  })

  it.each(['modelConfig', 'aiContext'] as const)(
    'clears preparing placeholder and progress when %s fails before the loop starts',
    async (failurePoint) => {
      if (failurePoint === 'modelConfig')
        mockGetFeatureModelConfig.mockRejectedValueOnce(new Error('No model configured'))
      jest.mocked(invoke).mockImplementation(async (command) => {
        if (command === 'create_agent_session')
          return sessionRow as never
        if (command === 'get_available_tools')
          return { tools: [], metadata: {} } as never
        if (command === 'get_ai_context')
          throw new Error('Context unavailable')
        throw new Error(`Unexpected command ${command}`)
      })

      const assistant = useSidebarChatAgent()
      await assistant.sendMessage('Summarize the data')
      expect(assistant.progress.value).toBeUndefined()
      expect(assistant.messages.value.some(message => message.preparingInProgress)).toBe(false)
      expect(assistant.error.value).toBe(
        failurePoint === 'modelConfig'
          ? 'No model configured'
          : 'Context unavailable',
      )
    },
  )
})
