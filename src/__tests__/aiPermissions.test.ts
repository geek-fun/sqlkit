/** @jest-environment jsdom */
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent, reactive } from 'vue'
import { AI_ACTIONS, aiPreset, getAiPolicy, resolveAiPermissions } from '@/common/aiPermissions'
import ConnectionPermissionEditor from '@/components/ai-permissions/ConnectionPermissionEditor.vue'
import DefaultPermissionEditor from '@/components/ai-permissions/DefaultPermissionEditor.vue'
import ToolPermissionReview from '@/components/ai-permissions/ToolPermissionReview.vue'
import SourcePermissionModal from '@/views/data-studio/components/modify-source-modal.vue'

jest.mock('@tauri-apps/api/core', () => ({ invoke: jest.fn() }))
jest.mock('@/common', () => ({ ...jest.requireActual('@/common/aiPermissions'), UPGRADE_URL: 'https://example.test/upgrade' }))
const mockEntitlement = reactive({ isLocalUltimate: true })
jest.mock('@/store', () => ({ useEntitlementStore: () => mockEntitlement, useConnectionStore: () => ({ getConnectionById: (id: string) => ({ id, name: 'Development database' }) }) }))
jest.mock('@tauri-apps/plugin-opener', () => ({ openUrl: jest.fn() }))
jest.mock('@/components/upgrade', () => ({ ProBadge: { template: '<span>Ultimate</span>' } }))
jest.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
jest.mock('@/components/ui/button', () => ({ Button: { template: '<button><slot /></button>' } }))
jest.mock('@/components/ui/label', () => ({ Label: { template: '<label><slot /></label>' } }))
jest.mock('@/components/ui/radio-group', () => ({
  RadioGroup: { props: ['modelValue', 'disabled'], emits: ['update:modelValue'], template: '<fieldset :data-value="modelValue" :disabled="disabled" @change="$emit(\'update:modelValue\', $event.target.value)"><slot /></fieldset>' },
  RadioGroupItem: { props: ['value'], template: '<input type="radio" :value="value">' },
}))
jest.mock('@/components/ui/switch', () => ({
  Switch: { props: ['checked', 'disabled'], emits: ['update:checked'], template: '<input type="checkbox" :checked="checked" :disabled="disabled" @change="$emit(\'update:checked\', $event.target.checked)">' },
}))
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
  it('persists Full access from the real recovery editor without changing the denying defaults', async () => {
    let saved = { permission_defaults: aiPreset('data'), connection_permissions: {} as Record<string, Record<string, boolean>> }
    mockEntitlement.isLocalUltimate = true
    mockInvoke.mockImplementation(async (command, args) => {
      if (command === 'save_ai_permissions') {
        const update = args as { connectionId: string, permissions: Record<string, boolean> }
        saved = { ...saved, connection_permissions: { ...saved.connection_permissions, [update.connectionId]: update.permissions } }
      }
      return saved
    })
    const wrapper = mount(ToolPermissionReview, { props: { open: true, denial: { kind: 'permission', source: 'system', connectionId: 'dev', action: 'manage', message: 'Manage denied' }, toolName: 'ddl', sessionId: 'session' } })
    await flushPromises()
    await wrapper.get('input[value="full"]').setValue()
    await wrapper.findAll('button').find(button => button.text() === 'dataStudio.modifySource.saveChanges')!.trigger('click')
    await flushPromises()
    const reloaded = await getAiPolicy()
    expect(reloaded.permission_defaults.manage).toBe(false)
    expect(reloaded.connection_permissions.dev).toEqual(aiPreset('full'))
    expect(resolveAiPermissions(reloaded.permission_defaults, reloaded.connection_permissions.dev).manage).toBe(true)
  })
  beforeEach(() => {
    mockEntitlement.isLocalUltimate = true
    mockStore.updateSessionSourcePermissions.mockReset()
    mockStore.detachSourceFromSession.mockReset()
    mockInvoke.mockReset()
    mockInvoke.mockResolvedValue(policy)
  })

  it('gates Community users without loading permissions or blocking ordinary connection saves', async () => {
    mockEntitlement.isLocalUltimate = false
    const wrapper = mount(ConnectionPermissionEditor, { props: { connectionId: 'dev' } })
    await flushPromises()
    expect(mockInvoke).not.toHaveBeenCalled()
    expect(wrapper.find('details').exists()).toBe(false)
    expect(wrapper.text()).toContain('aiPermissions.ultimateOnly')
    expect((wrapper.vm as unknown as { ready: boolean }).ready).toBe(true)
    await wrapper.get('button').trigger('click')
    expect(openUrl).toHaveBeenCalledWith('https://example.test/upgrade')
    mockEntitlement.isLocalUltimate = true
    await flushPromises()
    expect(mockInvoke).toHaveBeenCalledWith('get_ai_permissions')
    expect(wrapper.find('details').exists()).toBe(true)
  })

  it('discards an in-flight permission load when Ultimate access is lost', async () => {
    let finishLoad!: (value: typeof policy) => void
    mockInvoke.mockImplementationOnce(() => new Promise((resolve) => {
      finishLoad = resolve
    }))
    const wrapper = mount(ConnectionPermissionEditor, { props: { connectionId: 'dev' } })
    mockEntitlement.isLocalUltimate = false
    await flushPromises()
    finishLoad(policy)
    await flushPromises()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    expect(wrapper.find('details').exists()).toBe(false)
    expect((wrapper.vm as unknown as { ready: boolean }).ready).toBe(true)
  })

  it('resolves each action with explicit false distinct from inheritance', () => {
    expect(resolveAiPermissions(aiPreset('full'), { update: false })).toEqual({ read: true, create: true, update: false, delete: true, manage: true })
    expect(resolveAiPermissions({}, { read: true })).toEqual(aiPreset('read'))
    expect(aiPreset('inherit')).toEqual({})
    expect(AI_ACTIONS.every(action => aiPreset('none')[action] === false)).toBe(true)
  })

  it('describes the selected access level below the option group', async () => {
    const wrapper = mount(DefaultPermissionEditor)
    await flushPromises()
    ;['none', 'read', 'data', 'full', 'custom'].forEach((value) => {
      const option = wrapper.get(`label[for="ai-default-preset-${value}"]`)
      expect(option.text()).not.toContain(`${value}Help`)
      expect(option.get('input').attributes('aria-describedby')).toBe('ai-default-preset-description')
    })
    expect(wrapper.get('#ai-default-preset-description').text()).toMatch(/aiPermissions\.\w+Help/)
    await wrapper.get('input[value="full"]').setValue()
    expect(wrapper.get('#ai-default-preset-description').text()).toContain('aiPermissions.fullHelp')
  })

  it('loads saved decisions and displays effective access from the whole chain', async () => {
    const wrapper = await openEditor('dev')
    expect(wrapper.vm.rules).toEqual({ update: true, delete: false })
    expect(wrapper.text()).toContain('aiPermissions.effective: aiPermissions.read, aiPermissions.update')
    expect(wrapper.findAll('input[type="checkbox"]')).toHaveLength(5)
    expect(wrapper.text()).toContain('aiPermissions.fromConnection')
  })

  it('supports explicit allow and deny independently per action', async () => {
    const wrapper = await openEditor('dev')
    const updateSwitch = () => wrapper.get('input#ai-dev-update')
    await updateSwitch().setValue(false)
    expect(wrapper.vm.rules).toEqual({ update: false, delete: false })
    await updateSwitch().setValue(true)
    expect(wrapper.vm.rules).toEqual({ update: true, delete: false })
  })

  it('keeps custom decisions when opening the custom preset', async () => {
    const wrapper = await openEditor('dev')
    await wrapper.get('input[value="custom"]').setValue()
    expect(wrapper.vm.rules).toEqual({ update: true, delete: false })
    expect(wrapper.findAll('input[type="checkbox"]')).toHaveLength(5)
  })

  it('new connections inherit and presets preserve explicit decisions', async () => {
    const wrapper = await openEditor()
    expect(wrapper.vm.rules).toEqual({})
    await wrapper.get('input[value="none"]').setValue()
    expect(wrapper.vm.rules).toEqual(aiPreset('none'))
    await wrapper.get('input[value="inherit"]').setValue()
    expect(wrapper.vm.rules).toEqual({})
  })

  it('offers Custom from inherited access without changing stored decisions', async () => {
    const wrapper = await openEditor()
    await wrapper.get('input[value="custom"]').setValue()
    expect(wrapper.vm.rules).toEqual({})
    expect(wrapper.findAll('input[type="checkbox"]')).toHaveLength(5)
    await wrapper.findAll('input[type="checkbox"]')[1].setValue(true)
    expect(wrapper.vm.rules).toEqual({ create: true })
    await wrapper.get('input[value="inherit"]').setValue()
    expect(wrapper.vm.rules).toEqual({})
    expect(wrapper.findAll('input[type="checkbox"]')).toHaveLength(0)
  })

  it('shows custom defaults on load and keeps action editing open when values match a preset', async () => {
    mockInvoke.mockResolvedValueOnce({ permission_defaults: { ...aiPreset('read'), create: true }, connection_permissions: {} })
    const wrapper = mount(DefaultPermissionEditor)
    await flushPromises()
    expect(wrapper.findAll('input[type="checkbox"]')).toHaveLength(5)
    await wrapper.get('#ai-default-create').setValue(false)
    expect(wrapper.get('[data-value]').attributes('data-value')).toBe('custom')
    expect(wrapper.findAll('input[type="checkbox"]')).toHaveLength(5)
    await wrapper.get('button').trigger('click')
    await flushPromises()
    expect(mockInvoke).toHaveBeenLastCalledWith('save_ai_permissions', { defaults: aiPreset('read') })
  })

  it('entering Custom does not change default permissions or introduce a separate policy', async () => {
    const wrapper = mount(DefaultPermissionEditor)
    await flushPromises()
    await wrapper.get('input[value="custom"]').setValue()
    expect(wrapper.findAll('input[type="checkbox"]')).toHaveLength(5)
    await wrapper.get('button').trigger('click')
    await flushPromises()
    expect(mockInvoke).toHaveBeenLastCalledWith('save_ai_permissions', { defaults: aiPreset('read') })
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
    await wrapper.get('input[value="full"]').setValue()
    mockInvoke.mockRejectedValueOnce(new Error('Temporarily unavailable'))
    await wrapper.findAll('button').find(button => button.text() === 'aiPermissions.save')!.trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-value]').attributes('data-value')).toBe('full')
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
    await wrapper.get('input[value="none"]').setValue()
    await wrapper.findAll('button').find(button => button.text() === 'aiPermissions.save')!.trigger('click')
    await flushPromises()
    expect(mockInvoke).toHaveBeenLastCalledWith('save_ai_permissions', { defaults: aiPreset('none') })
    expect(wrapper.text()).toContain('aiPermissions.saved')
  })
})
