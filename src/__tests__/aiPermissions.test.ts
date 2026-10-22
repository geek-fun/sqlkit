/** @jest-environment jsdom */
import { invoke } from '@tauri-apps/api/core'
import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { AI_ACTIONS, aiPreset, resolveAiPermissions } from '@/common/aiPermissions'
import ConnectionPermissionEditor from '@/components/ai-permissions/ConnectionPermissionEditor.vue'
import DefaultPermissionEditor from '@/components/ai-permissions/DefaultPermissionEditor.vue'
import SourcePermissionModal from '@/views/data-studio/components/modify-source-modal.vue'

jest.mock('@tauri-apps/api/core', () => ({ invoke: jest.fn() }))
jest.mock('@/common', () => jest.requireActual('@/common/aiPermissions'))
jest.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
jest.mock('@/components/ui/button', () => ({ Button: { template: '<button><slot /></button>' } }))
jest.mock('@/components/ui/label', () => ({ Label: { template: '<label><slot /></label>' } }))
jest.mock('@/components/ui/select', () => ({
  Select: { props: ['modelValue'], emits: ['update:modelValue'], template: '<select :value="modelValue" @change="$emit(\'update:modelValue\', $event.target.value)"><slot /></select>' },
  SelectTrigger: { template: '<span />' },
  SelectValue: { template: '<span />' },
  SelectContent: { template: '<slot />' },
  SelectItem: { props: ['value', 'disabled'], template: '<option :value="value" :disabled="disabled"><slot /></option>' },
}))

jest.mock('@/components/ui/dialog', () => Object.fromEntries(
  ['Dialog', 'DialogContent', 'DialogDescription', 'DialogFooter', 'DialogHeader', 'DialogTitle']
    .map(name => [name, { template: '<section><slot /></section>' }]),
))
jest.mock('@/components/ui/checkbox', () => ({
  Checkbox: {
    props: ['checked', 'disabled'],
    emits: ['update:checked'],
    template: '<input type="checkbox" :checked="checked" :disabled="disabled" @change="$emit(\'update:checked\', $event.target.checked)">',
  },
}))

const mockStore = {
  activeSession: { id: 'session', sources: [{ sourceId: 'source', alias: 'Example', permissionsMode: 'custom', permissions: { read: true, update: true } }] },
  attachedSources: [{ sourceId: 'source', kind: 'database', connectionId: 'dev' }],
  updateSessionSourcePermissions: jest.fn(),
  updateSessionSourceMode: jest.fn(),
  detachSourceFromSession: jest.fn(),
}

jest.mock('@/store/dataStudioStore', () => ({ useDataStudioStore: () => mockStore }))

const mockInvoke = jest.mocked(invoke)
const policy = { permission_defaults: aiPreset('read'), connection_permissions: { dev: { update: true, delete: false } } }
const openEditor = async (id?: string) => {
  const wrapper = mount(defineComponent({
    components: { ConnectionPermissionEditor },
    data: () => ({ rules: {}, id }),
    template: '<ConnectionPermissionEditor v-model="rules" :connection-id="id" />',
  }))
  await flushPromises()
  await wrapper.get('summary').trigger('click')
  return wrapper
}

describe('aI permission chain', () => {
  beforeEach(() => {
    mockStore.updateSessionSourcePermissions.mockReset()
    mockStore.detachSourceFromSession.mockReset()
    mockInvoke.mockReset()
    mockInvoke.mockResolvedValue(policy)
  })

  it('resolves each action with explicit false distinct from inheritance', () => {
    expect(resolveAiPermissions(aiPreset('full'), { update: false })).toEqual({ read: true, create: true, update: false, delete: true, manage: true })
    expect(resolveAiPermissions({}, { read: true })).toEqual(aiPreset('read'))
    expect(aiPreset('inherit')).toEqual({})
    expect(AI_ACTIONS.every(action => aiPreset('none')[action] === false)).toBe(true)
  })

  it('loads saved decisions and displays effective access from the whole chain', async () => {
    const wrapper = await openEditor('dev')
    expect(wrapper.vm.rules).toEqual({ update: true, delete: false })
    expect(wrapper.text()).toContain('aiPermissions.effective: aiPermissions.read, aiPermissions.update')
    await wrapper.get('button').trigger('click')
    expect(wrapper.text()).toContain('aiPermissions.fromConnection')
    expect(wrapper.text()).toContain('aiPermissions.fromSystem')
  })

  it('supports inherit, explicit allow and explicit deny independently', async () => {
    const wrapper = await openEditor('dev')
    await wrapper.get('button').trigger('click')
    const actionSelect = () => wrapper.findAll('select')[3]
    await actionSelect().setValue('deny')
    expect(wrapper.vm.rules).toEqual({ update: false, delete: false })
    await actionSelect().setValue('inherit')
    expect(wrapper.vm.rules).toEqual({ delete: false })
    await actionSelect().setValue('allow')
    expect(wrapper.vm.rules).toEqual({ update: true, delete: false })
  })

  it('keeps custom decisions when opening the custom preset', async () => {
    const wrapper = await openEditor('dev')
    await wrapper.get('select').setValue('custom')
    expect(wrapper.vm.rules).toEqual({ update: true, delete: false })
    expect(wrapper.findAll('select')).toHaveLength(6)
  })

  it('new connections inherit and presets preserve explicit decisions', async () => {
    const wrapper = await openEditor()
    expect(wrapper.vm.rules).toEqual({})
    await wrapper.get('select').setValue('none')
    expect(wrapper.vm.rules).toEqual(aiPreset('none'))
    await wrapper.get('select').setValue('inherit')
    expect(wrapper.vm.rules).toEqual({})
  })

  it('blocks readiness after load failure and recovers through Retry', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Unavailable'))
    const wrapper = await openEditor('dev')
    const editor = wrapper.findComponent(ConnectionPermissionEditor)
    expect((editor.vm as unknown as { ready: boolean }).ready).toBe(false)
    expect(wrapper.text()).toContain('aiPermissions.loadFailed')
    await wrapper.get('button').trigger('click')
    await flushPromises()
    expect((editor.vm as unknown as { ready: boolean }).ready).toBe(true)
  })

  it('retries failed default saves without discarding the edited permissions', async () => {
    const wrapper = mount(DefaultPermissionEditor)
    await flushPromises()
    await wrapper.get('select').setValue('full')
    mockInvoke.mockRejectedValueOnce(new Error('Temporarily unavailable'))
    await wrapper.findAll('button').find(button => button.text() === 'aiPermissions.save')!.trigger('click')
    await flushPromises()
    expect(wrapper.get('select').element.value).toBe('full')
    await wrapper.findAll('button').find(button => button.text() === 'aiPermissions.retry')!.trigger('click')
    await flushPromises()
    expect(mockInvoke).toHaveBeenLastCalledWith('save_ai_permissions', { defaults: aiPreset('full') })
    expect(wrapper.text()).toContain('aiPermissions.saved')
  })

  it('keeps conversation edits after a failed save and retries the same draft', async () => {
    const wrapper = mount(SourcePermissionModal, { props: { open: true, sourceIdx: 0 } })
    await flushPromises()
    expect(wrapper.get('#conversation-create').attributes('disabled')).toBeDefined()
    await wrapper.get('#conversation-update').setValue(false)
    mockStore.updateSessionSourcePermissions.mockRejectedValueOnce(new Error('Unavailable'))
    await wrapper.findAll('button').find(button => button.text() === 'dataStudio.modifySource.saveChanges')!.trigger('click')
    await flushPromises()
    expect(wrapper.emitted('update:open')).toBeUndefined()
    expect((wrapper.get('#conversation-update').element as HTMLInputElement).checked).toBe(false)
    await wrapper.findAll('button').find(button => button.text() === 'aiPermissions.retry')!.trigger('click')
    await flushPromises()
    expect(mockStore.updateSessionSourcePermissions).toHaveBeenLastCalledWith('source', { read: true, create: false, update: false, delete: false, manage: false })
    expect(wrapper.emitted('update:open')).toEqual([[false]])
  })

  it('requires a separate detach confirmation and lets the user cancel it', async () => {
    const wrapper = mount(SourcePermissionModal, { props: { open: true, sourceIdx: 0 } })
    await flushPromises()
    await wrapper.findAll('button').find(button => button.text() === 'dataStudio.detachSource.title')!.trigger('click')
    expect(wrapper.text()).toContain('dataStudio.detachSource.message')
    expect(mockStore.detachSourceFromSession).not.toHaveBeenCalled()
    await wrapper.findAll('button').find(button => button.text() === 'aiPermissions.cancel')!.trigger('click')
    expect(wrapper.text()).not.toContain('dataStudio.detachSource.message')
    expect(wrapper.emitted('update:open')).toBeUndefined()
  })

  it('saves system defaults without overwriting connection decisions', async () => {
    const wrapper = mount(DefaultPermissionEditor)
    await flushPromises()
    await wrapper.get('select').setValue('none')
    await wrapper.findAll('button').find(button => button.text() === 'aiPermissions.save')!.trigger('click')
    await flushPromises()
    expect(mockInvoke).toHaveBeenLastCalledWith('save_ai_permissions', { defaults: aiPreset('none') })
    expect(wrapper.text()).toContain('aiPermissions.saved')
  })
})
