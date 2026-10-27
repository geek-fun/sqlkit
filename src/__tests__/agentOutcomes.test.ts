/** @jest-environment jsdom */
import type { AgentToolCall } from '@/store/dataStudioStore'
import { mount } from '@vue/test-utils'
import AgentMessageBubble from '@/components/agent-message-bubble.vue'
import { enUS } from '@/lang/enUS'
import { zhCN } from '@/lang/zhCN'

let mockMessages: Record<string, any> = enUS
jest.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string, values: Record<string, string> = {}) => {
    const text = key.split('.').reduce<unknown>((value, key) => value && typeof value === 'object' ? (value as Record<string, unknown>)[key] : undefined, mockMessages)
    return typeof text === 'string' ? text.replace(/\{(\w+)\}/g, (_match, name) => values[name] ?? name) : key
  } }),
}))

jest.mock('@/components/markdown-render.vue', () => ({ template: '<div />' }))
jest.mock('@/store/dataStudioStore', () => ({ useDataStudioStore: () => ({ toolResultFullBodies: {} }) }))
jest.mock('@/store/connectionStore', () => ({ useConnectionStore: () => ({ getConnectionById: () => ({ name: 'Development database' }) }) }))
jest.mock('@/components/ui/button', () => ({ Button: { template: '<button><slot /></button>' } }))

const call: AgentToolCall = {
  id: 'call',
  toolName: 'sqlkit__execute_ddl',
  args: '{"connection_id":"dev","sql":"CREATE TABLE example(id INT)"}',
  status: 'denied',
  riskLevel: 'destructive',
  requiresConfirmation: false,
  result: 'Manage denied',
  denial: { kind: 'permission', source: 'connection', connectionId: 'db', action: 'manage', message: 'Manage denied' },
}
const render = (tool = call, locale = 'enUS', canRetry = true) => {
  mockMessages = locale === 'zhCN' ? zhCN : enUS
  return mount(AgentMessageBubble, {
    props: { message: { role: 'assistant', status: 'done', toolCalls: [tool] }, canRetry },
  })
}

describe('tool outcome notices', () => {
  it('keeps blocked action, connection, required permission and source visible outside details', async () => {
    const wrapper = render()
    const notice = wrapper.get('[role="alert"]')
    expect(notice.text()).toContain('Blocked by permissions — action not executed')
    expect(notice.text()).toContain('Development database')
    expect(notice.text()).toContain('Required:')
    expect(notice.text()).toContain('connection’s AI permissions')
    expect(wrapper.get('.tool-verb-label').text()).toBe('Blocked')
    expect(notice.element.closest('details')).toBeNull()
    await notice.findAll('button')[0].trigger('click')
    expect(wrapper.emitted('reviewPermissions')?.[0]).toEqual([call])
    await notice.findAll('button')[1].trigger('click')
    expect(wrapper.emitted('retryToolCall')?.[0]).toEqual([call])
  })

  it('distinguishes user rejection without offering to change permissions', () => {
    const wrapper = render({ ...call, denial: { ...call.denial!, kind: 'user' } })
    expect(wrapper.get('[role="alert"]').text()).toContain('Rejected by you — action not executed')
    expect(wrapper.get('.tool-verb-label').text()).toBe('Rejected')
    expect(wrapper.text()).not.toContain('Review permissions')
  })

  it('localizes rejection and disables recovery while the previous run is still settling', () => {
    const wrapper = render(call, 'zhCN', false)
    expect(wrapper.get('[role="alert"]').text()).toContain('权限不足，操作未执行')
    expect(wrapper.findAll('button').every(button => button.attributes('disabled') !== undefined)).toBe(true)
  })

  it.each(['error', 'interrupted'] as const)('never labels a %s call as executed', (status) => {
    const wrapper = render({ ...call, status, denial: undefined })
    expect(wrapper.find('[role="alert"]').exists()).toBe(true)
    expect(wrapper.get('.tool-verb-label').text()).not.toContain('Executed')
    expect(wrapper.find('.activity-status-icon.done').exists()).toBe(false)
  })

  it('does not mislabel unavailable sources or invalid requests as permission failures', () => {
    const wrapper = render({ ...call, denial: { kind: 'unavailable', source: 'unknown', message: 'Saved connection is no longer available' } })
    expect(wrapper.get('[role="alert"]').text()).toContain('Action unavailable — action not executed')
    expect(wrapper.get('[role="alert"]').text()).toContain('Saved connection is no longer available')
    expect(wrapper.text()).not.toContain('Review permissions')
  })
})
