/** @jest-environment jsdom */
import { flushPromises, mount } from '@vue/test-utils'
import { saveAiPermissions } from '@/common'
import ToolPermissionReview from '@/components/ai-permissions/ToolPermissionReview.vue'
import { agentApi } from '@/datasources/agentApi'

const mockRemoveRule = jest.fn().mockResolvedValue(undefined)
jest.mock('@/common', () => ({ saveAiPermissions: jest.fn() }))
jest.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
jest.mock('@/store/dataStudioStore', () => ({ useDataStudioStore: () => ({ activeSession: { id: 'chat', sources: [] }, removeConfirmationRule: mockRemoveRule }) }))
jest.mock('@/datasources/agentApi', () => ({ agentApi: { loadConfirmationRules: jest.fn() } }))
jest.mock('@/components/upgrade', () => ({ PaidGate: { props: ['feature'], template: '<slot />' } }))
jest.mock('@/components/ui/button', () => ({ Button: { template: '<button><slot /></button>' } }))
jest.mock('@/components/ui/dialog', () => Object.fromEntries(['Dialog', 'DialogContent', 'DialogDescription', 'DialogFooter', 'DialogHeader', 'DialogTitle'].map(name => [name, { template: '<section><slot /></section>' }])))
jest.mock('@/views/data-studio/components/modify-source-modal.vue', () => ({ __esModule: true, default: { template: '<div />' } }))
jest.mock('@/components/ai-permissions/DefaultPermissionEditor.vue', () => ({ __esModule: true, default: { template: '<div>System defaults</div>' } }))
jest.mock('@/components/ai-permissions/ConnectionPermissionEditor.vue', () => ({
  __esModule: true,
  default: {
    props: ['connectionId'],
    emits: ['update:modelValue'],
    data: () => ({ ready: true }),
    mounted() { (this as any).$emit('update:modelValue', { manage: true }) },
    template: '<div>Connection {{ connectionId }}</div>',
  },
}))

const denial = { kind: 'permission' as const, source: 'connection' as const, connectionId: 'db', action: 'manage' as const, message: 'Manage denied' }
const openReview = (source = denial.source as 'connection' | 'system' | 'rule') => mount(ToolPermissionReview, {
  props: { open: true, denial: { ...denial, source }, toolName: 'ddl', sessionId: 'chat' },
})

describe('permission recovery', () => {
  beforeEach(() => {
    jest.clearAllMocks()
    jest.mocked(saveAiPermissions).mockResolvedValue({ permission_defaults: {}, connection_permissions: {} })
  })

  it('saves only the blocked connection and returns without executing the action', async () => {
    const wrapper = openReview()
    await flushPromises()
    expect(wrapper.text()).toContain('Connection db')
    expect(wrapper.text()).not.toContain('System defaults')
    const save = wrapper.findAll('button').find(button => button.text() === 'dataStudio.modifySource.saveChanges')!
    await save.trigger('click')
    await flushPromises()
    expect(saveAiPermissions).toHaveBeenCalledWith('db', { manage: true })
    expect(wrapper.emitted('update:open')).toEqual([[false]])
    expect(wrapper.emitted('retryToolCall')).toBeUndefined()
  })

  it('opens system defaults when inheritance caused the denial', () => {
    const wrapper = openReview('system')
    expect(wrapper.text()).toContain('System defaults')
    expect(wrapper.text()).not.toContain('Connection db')
    expect(saveAiPermissions).not.toHaveBeenCalled()
  })

  it('removes only the matching saved deny rule in the current conversation', async () => {
    jest.mocked(agentApi.loadConfirmationRules).mockResolvedValue([
      { id: 'other', session_id: 'chat', tool_name: 'read', action: 'deny', created_at: '0' },
      { id: 'target', session_id: 'chat', tool_name: 'ddl', action: 'deny', created_at: '0' },
    ])
    const wrapper = openReview('rule')
    await wrapper.findAll('button').find(button => button.text() === 'dataStudio.agent.message.removeDenyRule')!.trigger('click')
    await flushPromises()
    expect(agentApi.loadConfirmationRules).toHaveBeenCalledWith('chat')
    expect(mockRemoveRule).toHaveBeenCalledWith('target')
    expect(saveAiPermissions).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('dataStudio.agent.message.ruleRemoved')
  })
})
