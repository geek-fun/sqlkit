/** @jest-environment jsdom */
import { flushPromises, mount } from '@vue/test-utils'
import { reactive } from 'vue'
import { saveAiPermissions } from '@/common'
import ToolPermissionReview from '@/components/ai-permissions/ToolPermissionReview.vue'
import { agentApi } from '@/datasources/agentApi'

const mockRemoveRule = jest.fn().mockResolvedValue(undefined)
const mockConnection = jest.fn()
const mockEntitlement = reactive({ isLocalUltimate: true })
jest.mock('@/common', () => ({ saveAiPermissions: jest.fn() }))
jest.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
jest.mock('@/store/dataStudioStore', () => ({ useDataStudioStore: () => ({ activeSession: { id: 'chat', sources: [] }, removeConfirmationRule: mockRemoveRule }) }))
jest.mock('@/datasources/agentApi', () => ({ agentApi: { loadConfirmationRules: jest.fn() } }))
jest.mock('@/store', () => ({ useConnectionStore: () => ({ getConnectionById: mockConnection }), useEntitlementStore: () => mockEntitlement }))
jest.mock('@/components/ui/button', () => ({ Button: { template: '<button><slot /></button>' } }))
jest.mock('@/components/ui/dialog', () => Object.fromEntries(['Dialog', 'DialogContent', 'DialogDescription', 'DialogFooter', 'DialogHeader', 'DialogTitle'].map(name => [name, { template: '<section><slot /></section>' }])))
jest.mock('@/views/data-studio/components/modify-source-modal.vue', () => ({ __esModule: true, default: { template: '<div />' } }))
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
    mockConnection.mockReturnValue({ id: 'db', name: 'Development database' })
    mockEntitlement.isLocalUltimate = true
    jest.mocked(saveAiPermissions).mockResolvedValue({ permission_defaults: {}, connection_permissions: {} })
  })

  it('saves only the blocked connection and returns without executing the action', async () => {
    const wrapper = openReview()
    await flushPromises()
    expect(wrapper.text()).toContain('Connection db')
    expect(wrapper.text()).not.toContain('System defaults')
    const save = wrapper.findAll('button').find(button => button.text() === 'dataStudio.modifySource.saveChanges')!
    const back = wrapper.findAll('button').find(button => button.text() === 'dataStudio.agent.message.backToChat')!
    expect(save.element.parentElement).toBe(back.element.parentElement)
    await save.trigger('click')
    await flushPromises()
    expect(saveAiPermissions).toHaveBeenCalledWith('db', { manage: true })
    expect(wrapper.emitted('update:open')).toEqual([[false]])
    expect(wrapper.emitted('retryToolCall')).toBeUndefined()
  })

  it('edits only the blocked connection when inheritance caused the denial', async () => {
    const wrapper = openReview('system')
    await flushPromises()
    expect(wrapper.text()).toContain('Connection db')
    expect(wrapper.text()).not.toContain('System defaults')
    expect(wrapper.text()).toContain('dataStudio.agent.message.reviewConnectionHelp')
    const save = wrapper.findAll('button').find(button => button.text() === 'dataStudio.modifySource.saveChanges')!
    await save.trigger('click')
    await flushPromises()
    expect(saveAiPermissions).toHaveBeenCalledWith('db', { manage: true })
  })

  it('never falls back to editing defaults when a connection cannot be identified', () => {
    const wrapper = mount(ToolPermissionReview, { props: { open: true, denial: { ...denial, source: 'system', connectionId: undefined }, toolName: 'ddl', sessionId: 'chat' } })
    expect(wrapper.text()).toContain('dataStudio.agent.message.connectionUnavailable')
    expect(wrapper.text()).not.toContain('dataStudio.modifySource.saveChanges')
    expect(saveAiPermissions).not.toHaveBeenCalled()
  })

  it('does not offer edits for a deleted connection', () => {
    mockConnection.mockReturnValue(undefined)
    const wrapper = openReview()
    expect(wrapper.text()).toContain('dataStudio.agent.message.connectionUnavailable')
    expect(wrapper.text()).not.toContain('dataStudio.modifySource.saveChanges')
    expect(saveAiPermissions).not.toHaveBeenCalled()
  })

  it('keeps the connection permission entitlement gate on recovery', () => {
    mockEntitlement.isLocalUltimate = false
    const wrapper = openReview()
    expect(wrapper.text()).not.toContain('dataStudio.modifySource.saveChanges')
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
