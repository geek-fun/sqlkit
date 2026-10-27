/**
 * @jest-environment jsdom
 */

/**
 * SshTunnelConfigPanel — configure-as-intent journey tests.
 *
 * The tunnel has no explicit enable toggle: adding a hop / attaching the
 * system proxy / saving inline settings IS the intent (enabled is derived in
 * patch()), mirroring the sibling NoSQL client. These tests pin that semantic.
 */
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import Sortable from 'sortablejs'
import SshProfileDialog from '@/components/connections/SshProfileDialog.vue'
import SshTunnelConfigPanel from '@/components/connections/SshTunnelConfigPanel.vue'

jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(async (cmd: string) =>
    cmd === 'detect_system_proxy' ? 'http://127.0.0.1:7890' : []),
}))

jest.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: jest.fn(),
}))

jest.mock('vue-i18n', () => ({
  useI18n: () => ({
    // Plural calls pass a count (number); keyed calls pass params.
    t: (key: string, params?: Record<string, unknown> | number) =>
      typeof params === 'number'
        ? key
        : params && 'host' in params
          ? `${key}(${params.host})`
          : key,
  }),
}))

// sortablejs ships CJS/ESM mixed exports that break under ts-jest interop;
// the drag handle path is exercised in the real app, not here.
jest.mock('sortablejs', () => ({
  __esModule: true,
  default: { create: jest.fn(() => ({ destroy: jest.fn() })) },
}))

const mockSaveProfile = jest.fn(async (profile: { id: string, name: string }) => ({ ...profile, id: profile.id || 'new-1' }))

jest.mock('@/store/sshProfileStore', () => ({
  emptySshProfile: () => ({
    id: '',
    name: '',
    host: '',
    port: 22,
    username: '',
    authMethod: '',
    password: '',
    keyPath: '',
    keyPassphrase: '',
    useSshAgent: false,
    sshAgentSockPath: '',
    connectTimeoutSecs: 10,
    keepaliveIntervalSecs: 30,
    verifyHostKey: false,
    exposeLan: false,
  }),
  useSshProfileStore: () => ({
    fetch: jest.fn(),
    // eslint-disable-next-line ts/no-use-before-define
    byId: (id: string) => HOP_PROFILES.find(p => p.id === id),
    // eslint-disable-next-line ts/no-use-before-define
    profiles: HOP_PROFILES,
    save: mockSaveProfile,
  }),
  profileToTransportLayer: (profile: { host: string, port: number, username: string }) => ({
    type: 'ssh',
    enabled: true,
    host: profile.host,
    port: profile.port,
    username: profile.username,
    authMethod: 'password',
    password: '',
    keyPath: '',
    keyPassphrase: '',
    useSshAgent: false,
    sshAgentSockPath: '',
    useSystemProxy: false,
    connectTimeoutSecs: 10,
    keepaliveIntervalSecs: 30,
  }),
}))

jest.mock('@/store/entitlementStore', () => ({
  useEntitlementStore: () => ({ isLocalUltimate: true }),
}))

jest.mock('@/components/connections/SshProfileDialog.vue', () => ({
  __esModule: true,
  default: { name: 'SshProfileDialog', template: '<span />', methods: { show: jest.fn() } },
}))

jest.mock('@/components/ui/button', () => ({ Button: { template: '<button><slot /></button>' } }))
jest.mock('@/components/ui/label', () => ({ Label: { template: '<label><slot /></label>' } }))
jest.mock('@/components/ui/input', () => ({
  Input: {
    props: ['modelValue'],
    emits: ['update:modelValue'],
    template: '<input :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
  },
}))
jest.mock('@/components/ui/dialog', () => ({
  Dialog: { props: ['open'], template: '<div v-if="open" role="dialog"><slot /></div>' },
  DialogContent: { template: '<div><slot /></div>' },
  DialogHeader: { template: '<header><slot /></header>' },
  DialogTitle: { template: '<h2><slot /></h2>' },
  DialogDescription: { template: '<p><slot /></p>' },
  DialogFooter: { template: '<footer><slot /></footer>' },
}))
jest.mock('@/components/ui/select', () => ({
  Select: { template: '<select><slot /></select>' },
  SelectContent: { template: '<span><slot /></span>' },
  SelectGroup: { template: '<span><slot /></span>' },
  SelectItem: { template: '<option><slot /></option>' },
  SelectTrigger: { template: '<span><slot /></span>' },
  SelectValue: { template: '<span />' },
}))
jest.mock('@/components/ui/combobox', () => ({
  SearchableSelect: {
    name: 'SearchableSelect',
    props: ['modelValue', 'options', 'placeholder', 'searchThreshold', 'disabled'],
    emits: ['update:modelValue'],
    template: '<select class="stub-combobox" :disabled="disabled" :value="modelValue" @change="$emit(\'update:modelValue\', $event.target.value)"><option v-for="o in options" :key="o.value" :value="o.value">{{ o.label }}</option></select>',
  },
}))
jest.mock('@/components/upgrade', () => ({ ProBadge: { template: '<span />' } }))

const HOP_PROFILES = [
  { id: 'p1', name: 'Bastion A', host: 'bastion-a', port: 22, username: 'ops', authMethod: 'password', password: '', keyPath: '', keyPassphrase: '', useSshAgent: false, sshAgentSockPath: '', connectTimeoutSecs: 10, keepaliveIntervalSecs: 30, verifyHostKey: false, exposeLan: false },
  { id: 'p2', name: 'Bastion B', host: 'bastion-b', port: 22, username: 'ops', authMethod: 'password', password: '', keyPath: '', keyPassphrase: '', useSshAgent: false, sshAgentSockPath: '', connectTimeoutSecs: 10, keepaliveIntervalSecs: 30, verifyHostKey: false, exposeLan: false },
]

const mountPanel = (modelValue?: Record<string, unknown>) =>
  mount(SshTunnelConfigPanel, {
    props: {
      modelValue: modelValue as never,
      remoteHost: 'db.example.com',
      remotePort: 5432,
    },
  })

const lastPayload = (wrapper: ReturnType<typeof mountPanel>) => {
  const emitted = wrapper.emitted('update:modelValue') ?? []
  return emitted[emitted.length - 1]?.[0] as {
    enabled?: boolean
    profileIds?: string[]
    useSystemProxy?: boolean
    host?: string
  }
}

/** Mirror the parent dialog's v-model: write the last emitted payload back into props. */
async function syncProps(wrapper: ReturnType<typeof mountPanel>) {
  await wrapper.vm.$nextTick()
  const payload = lastPayload(wrapper)
  if (payload !== undefined)
    await wrapper.setProps({ modelValue: payload as never })
  await wrapper.vm.$nextTick()
}

describe('sshTunnelConfigPanel — configure-as-intent', () => {
  beforeEach(() => {
    jest.clearAllMocks()
    setActivePinia(createPinia())
  })

  it('renders the empty state and NO enable toggle when nothing is configured', async () => {
    const wrapper = mountPanel()
    await flushPromises()
    expect(wrapper.find('.stub-combobox').exists()).toBe(true)
    expect(wrapper.text()).toContain('components.serverForm.ssh.notConfigured')
    // The explicit enable toggle is gone; with a proxy detected the Add row
    // is the only proxy affordance — no checkbox fallback.
    expect(wrapper.find('input[type="checkbox"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('components.serverForm.ssh.systemProxyDetected')
  })

  it('offers Add for the detected system proxy from the empty state (DockIt parity)', async () => {
    const wrapper = mountPanel()
    await flushPromises()
    const addButton = wrapper.findAll('button').find(b => b.text() === 'components.serverForm.ssh.systemProxyAdd')
    expect(addButton).toBeTruthy()
  })

  it('adding a hop derives enabled=true without any explicit toggle', async () => {
    const wrapper = mountPanel()
    await flushPromises()
    await wrapper.find('.stub-combobox').setValue('p1')
    await wrapper.find('.stub-combobox').trigger('change')
    await syncProps(wrapper)

    const payload = lastPayload(wrapper)
    expect(payload.profileIds).toEqual(['p1'])
    expect(payload.enabled).toBe(true)
  })

  it('detaching the last hop removes it from the list and derives enabled=false', async () => {
    const wrapper = mountPanel({ enabled: true, profileIds: ['p1'] })
    await flushPromises()
    const detachButton = wrapper.findAll('button').find(b => b.attributes('title') === 'components.serverForm.ssh.detachAction')
    expect(detachButton).toBeTruthy()
    await detachButton?.trigger('click')
    await syncProps(wrapper)

    const payload = lastPayload(wrapper)
    expect(payload.profileIds).toEqual([])
    expect(payload.enabled).toBe(false)
  })

  it('attaching the detected system proxy alone enables the tunnel — no hop needed', async () => {
    const wrapper = mountPanel()
    await flushPromises()
    const addButton = wrapper.findAll('button').find(b => b.text() === 'components.serverForm.ssh.systemProxyAdd')
    expect(addButton).toBeTruthy()
    await addButton?.trigger('click')
    await syncProps(wrapper)

    const payload = lastPayload(wrapper)
    expect(payload.enabled).toBe(true)
    expect(payload.useSystemProxy).toBe(true)
  })

  it('an attached system proxy renders pinned and detaches without dropping hops', async () => {
    const wrapper = mountPanel({ enabled: true, profileIds: ['p1'], useSystemProxy: true })
    await flushPromises()
    expect(wrapper.text()).not.toContain('components.serverForm.ssh.systemProxyAdd')
    const detachButton = wrapper.findAll('button').find(b => b.attributes('title') === 'components.serverForm.ssh.systemProxyDetach')
    expect(detachButton).toBeTruthy()
    await detachButton?.trigger('click')
    await syncProps(wrapper)

    const payload = lastPayload(wrapper)
    expect(payload.useSystemProxy).toBe(false)
    expect(payload.profileIds).toEqual(['p1'])
    expect(payload.enabled).toBe(true)
  })

  const legacyConfig = {
    enabled: true,
    host: 'jump.example.com',
    port: 2222,
    username: 'ops',
    authMethod: 'password',
    password: 'synthetic-password',
    useSystemProxy: true,
  }

  it('shows a compact inline tunnel without converting or revealing credentials', async () => {
    const wrapper = mountPanel(legacyConfig)
    await flushPromises()
    expect(wrapper.find('#ssh-host').exists()).toBe(false)
    expect(wrapper.text()).toContain('ops@jump.example.com:2222')
    expect(wrapper.text()).not.toContain('synthetic-password')
    expect(mockSaveProfile).not.toHaveBeenCalled()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    expect(wrapper.find('.stub-combobox').attributes('disabled')).toBeDefined()
    wrapper.findComponent({ name: 'SearchableSelect' }).vm.$emit('update:modelValue', 'p1')
    await wrapper.vm.$nextTick()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  })

  it('edits tunnel details in a draft and keeps Cancel separate from Save', async () => {
    const wrapper = mountPanel(legacyConfig)
    const edit = () => wrapper.get('button[title="components.serverForm.ssh.editTunnel"]')
    await edit().trigger('click')
    expect(wrapper.find('[role="dialog"]').exists()).toBe(true)
    await wrapper.get('#ssh-host').setValue('cancelled.example.com')
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    await wrapper.get('footer button').trigger('click')
    expect(wrapper.find('#ssh-host').exists()).toBe(false)
    await edit().trigger('click')
    expect((wrapper.get('#ssh-host').element as HTMLInputElement).value).toBe('jump.example.com')
    await wrapper.get('#ssh-host').setValue('new-jump.example.com')
    await wrapper.findAll('footer button')[1]!.trigger('click')
    expect(lastPayload(wrapper)).toMatchObject({ ...legacyConfig, host: 'new-jump.example.com' })
    expect(mockSaveProfile).not.toHaveBeenCalled()
    expect(wrapper.find('#ssh-host').exists()).toBe(false)
  })

  it.each(['password', 'privateKey', 'agent'])('converts %s only on click and preserves authentication and proxy', async (authMethod) => {
    const config = { ...legacyConfig, authMethod, privateKey: '/synthetic/key', privateKeyPassphrase: 'synthetic-passphrase' }
    const wrapper = mountPanel(config)
    await flushPromises()
    const convert = wrapper.findAll('button').find(b => b.text() === 'components.serverForm.ssh.convertToProfile')!
    await convert.trigger('click')
    await flushPromises()
    expect(mockSaveProfile).toHaveBeenCalledTimes(1)
    expect(mockSaveProfile).toHaveBeenCalledWith(expect.objectContaining({
      host: config.host,
      port: config.port,
      username: config.username,
      authMethod: authMethod === 'privateKey' ? 'key' : authMethod,
      password: config.password,
      keyPath: config.privateKey,
      keyPassphrase: config.privateKeyPassphrase,
      useSshAgent: authMethod === 'agent',
    }))
    expect(lastPayload(wrapper)).toMatchObject({
      enabled: true,
      profileIds: ['new-1'],
      useSystemProxy: true,
      host: '',
      username: '',
      password: '',
      privateKey: '',
      privateKeyPassphrase: '',
    })
  })

  it('keeps inline settings on conversion failure and allows a retry', async () => {
    mockSaveProfile.mockRejectedValueOnce(new Error('Profile storage unavailable'))
    const wrapper = mountPanel(legacyConfig)
    await flushPromises()
    const convert = wrapper.findAll('button').find(b => b.text() === 'components.serverForm.ssh.convertToProfile')!
    await convert.trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('Profile storage unavailable')
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    await convert.trigger('click')
    await flushPromises()
    expect(lastPayload(wrapper).profileIds).toEqual(['new-1'])
  })

  it('detaches the inline tunnel while retaining its attached proxy', async () => {
    const wrapper = mountPanel(legacyConfig)
    await wrapper.get('button[title="components.serverForm.ssh.detachAction"]').trigger('click')
    expect(lastPayload(wrapper)).toMatchObject({ enabled: true, useSystemProxy: true, host: '', password: '' })
  })

  it('reorders saved hops with keyboard-accessible buttons', async () => {
    const wrapper = mountPanel({ enabled: true, profileIds: ['p1', 'p2'] })
    await wrapper.findAll('button[title="components.serverForm.ssh.moveDown"]')[0]!.trigger('click')
    await syncProps(wrapper)
    expect(lastPayload(wrapper).profileIds).toEqual(['p2', 'p1'])
    expect(wrapper.findAll('.ssh-hop-handle').map(el => el.element.parentElement?.textContent)).toEqual([
      expect.stringContaining('Bastion B'),
      expect.stringContaining('Bastion A'),
    ])
  })

  it('restores the sortable DOM before applying a drag reorder and reinitializes after detach', async () => {
    const wrapper = mountPanel({ enabled: true, profileIds: ['p1', 'p2'] })
    await flushPromises()
    const create = Sortable.create as jest.Mock
    const options = create.mock.calls[create.mock.calls.length - 1]![1]
    const rows = wrapper.findAll('.ssh-hop-handle').map(el => el.element.parentElement!)
    rows[0]!.parentElement!.appendChild(rows[0]!)
    options.onEnd({ item: rows[0], oldIndex: 0, newIndex: 1 })
    await syncProps(wrapper)
    expect(lastPayload(wrapper).profileIds).toEqual(['p2', 'p1'])
    expect(wrapper.findAll('.ssh-hop-handle')[0]!.element.parentElement?.textContent).toContain('Bastion B')
    const instance = create.mock.results[create.mock.results.length - 1]!.value
    await wrapper.setProps({ modelValue: { enabled: false, profileIds: [] } as never })
    expect(instance.destroy).toHaveBeenCalled()
    const previous = create.mock.calls.length
    await wrapper.setProps({ modelValue: { enabled: true, profileIds: ['p1'] } as never })
    expect(create.mock.calls.length).toBe(previous + 1)
  })

  it('saving a NEW profile appends it to the chain', async () => {
    const wrapper = mountPanel({ enabled: true, profileIds: ['p1'] })
    await wrapper.getComponent(SshProfileDialog).vm.$emit('save', { id: 'p2', name: 'Bastion B' })
    await wrapper.vm.$nextTick()

    expect(lastPayload(wrapper).profileIds).toEqual(['p1', 'p2'])
  })

  it('saving an EXISTING hop does not duplicate it', async () => {
    const wrapper = mountPanel({ enabled: true, profileIds: ['p1'] })
    const before = wrapper.emitted('update:modelValue')?.length ?? 0
    await wrapper.getComponent(SshProfileDialog).vm.$emit('save', { id: 'p1', name: 'Bastion A' })
    await wrapper.vm.$nextTick()

    // No state change → no new emit; the chain keeps exactly one hop.
    expect(wrapper.emitted('update:modelValue')?.length ?? 0).toBe(before)
  })
})
