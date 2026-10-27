import { createPinia, setActivePinia } from 'pinia'
import { agentApi } from '@/datasources/agentApi'
import { useDataStudioStore } from '@/store/dataStudioStore'

jest.mock('@/datasources/agentApi', () => ({ agentApi: { loadSessionMessages: jest.fn(), loadAgentSessions: jest.fn() } }))

const denial = { kind: 'permission' as const, source: 'connection' as const, connectionId: 'db', action: 'manage' as const, message: 'Manage denied' }
const rows = [
  {
    id: 'assistant',
    session_id: 'chat',
    role: 'assistant',
    created_at: '2026-10-27T00:00:00Z',
    content: JSON.stringify({ content: '', tool_calls: [] }),
    tool_calls: [
      { id: 'success', tool_name: 'query', arguments: '{}', status: 'completed', result: 'One row' },
      { id: 'failure', tool_name: 'write', arguments: '{}', status: 'failed', result: 'Database error' },
      { id: 'blocked', tool_name: 'ddl', arguments: '{}', status: 'denied', result: denial.message, denial },
      { id: 'rejected', tool_name: 'delete', arguments: '{}', status: 'denied', result: 'Rejected', denial: { ...denial, kind: 'user' as const } },
      { id: 'unfinished', tool_name: 'write', arguments: '{}', status: 'approved' },
    ],
  },
  { id: 'tool-result', session_id: 'chat', role: 'tool', created_at: '2026-10-27T00:00:01Z', content: JSON.stringify({ tool_call_id: 'blocked', content: denial.message }) },
]

describe('agent history outcomes', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    jest.mocked(agentApi.loadSessionMessages).mockResolvedValue(rows)
    jest.mocked(agentApi.loadAgentSessions).mockResolvedValue([{
      id: 'chat',
      title: 'Chat',
      status: 'blocked',
      sources: '[]',
      permissions_mode: 'Auto',
      model_id: '',
      created_at: '2026-10-27T00:00:00Z',
      updated_at: '2026-10-27T00:00:01Z',
    }])
  })

  it('restores successes, failures, rejections and blocked state after reopening', async () => {
    const store = useDataStudioStore()
    await store.loadSessions()
    const session = store.sessions[0]
    expect(session.status).toBe('blocked')
    expect(session.messages[0].toolCalls?.map(call => call.status)).toEqual(['done', 'error', 'denied', 'denied', 'interrupted'])
    expect(session.messages[0].toolCalls?.[2]).toMatchObject({ result: denial.message, denial, requiresConfirmation: false })
    expect(session.messages[0].toolCalls?.[3].denial?.kind).toBe('user')
    expect(session.messages[1].toolCallId).toBe('blocked')
    expect(session.messages[1].content).toBe(denial.message)
    await store.reloadSessionMessages('chat')
    expect(store.sessions[0].messages[0].toolCalls?.[2].denial).toEqual(denial)
  })

  it('never presents a legacy call with no recorded outcome as successful', async () => {
    jest.mocked(agentApi.loadSessionMessages).mockResolvedValue([{
      ...rows[0],
      tool_calls: undefined,
      content: JSON.stringify({ content: '', tool_calls: [{ id: 'unknown', function: { name: 'write', arguments: '{}' } }] }),
    }])
    const store = useDataStudioStore()
    await store.loadSessions()
    expect(store.sessions[0].messages[0].toolCalls?.[0].status).toBe('interrupted')
  })
})
