/**
 * @jest-environment jsdom
 */

/**
 * SshTunnelConfigPanel bindings — needs jsdom for radix-vue mounting.
 */
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import SshTunnelConfigPanel from '@/components/connections/SshTunnelConfigPanel.vue'

jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn().mockResolvedValue([]),
}))

jest.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: jest.fn(),
}))

jest.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}))

// Radix-vue CheckboxRoot wires PointerEvent-based behaviour that jsdom does not
// implement; stub Checkbox with the same contract radix v1 actually exposes
// (checked prop / update:checked emit) so the test documents the binding shape.
jest.mock('@/components/ui/checkbox', () => ({
  Checkbox: {
    name: 'Checkbox',
    props: ['checked', 'id'],
    emits: ['update:checked'],
    template:
      '<input type="checkbox" :id="id" :checked="checked === true" @change="$emit(\'update:checked\', $event.target.checked)" />',
  },
}))

jest.mock('@/store/sshProfileStore', () => ({
  useSshProfileStore: () => ({
    fetch: jest.fn(),
    byId: () => undefined,
    profiles: [],
  }),
}))

// Simulate an Ultimate user so the checkbox branch renders (non-Ultimate gets
// the gated strip instead).
jest.mock('@/store/entitlementStore', () => ({
  useEntitlementStore: () => ({ isLocalUltimate: true }),
}))

jest.mock('@/components/connections/SshProfileDialog.vue', () => ({
  default: { template: '<span />' },
}))

jest.mock('@/components/ui/button', () => ({ Button: { template: '<button><slot /></button>' } }))
jest.mock('@/components/ui/label', () => ({ Label: { template: '<label><slot /></label>' } }))
jest.mock('@/components/ui/input', () => ({ Input: { template: '<input />' } }))
jest.mock('@/components/ui/select', () => ({
  Select: { template: '<select><slot /></select>' },
  SelectContent: { template: '<span><slot /></span>' },
  SelectGroup: { template: '<span><slot /></span>' },
  SelectItem: { template: '<option><slot /></option>' },
  SelectTrigger: { template: '<span><slot /></span>' },
  SelectValue: { template: '<span />' },
}))
jest.mock('@/components/upgrade', () => ({ ProBadge: { template: '<span />' } }))

const mountPanel = (modelValue?: Record<string, unknown>) =>
  mount(SshTunnelConfigPanel, {
    props: {
      modelValue: modelValue as never,
      remoteHost: 'db.example.com',
      remotePort: 5432,
    },
  })

describe('sshTunnelConfigPanel checkbox bindings', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('toggling "Use SSH Tunnel" emits enabled config (radix v1: update:checked)', async () => {
    const wrapper = mountPanel()
    const checkbox = wrapper.find('#use-ssh')

    await checkbox.setValue(true)

    const emitted = wrapper.emitted('update:modelValue') ?? []
    const payload = emitted[emitted.length - 1]?.[0] as { enabled?: boolean }
    expect(payload.enabled).toBe(true)
  })

  it('unchecking clears profileIds and proxy flag', async () => {
    const wrapper = mountPanel({ enabled: true, profileIds: ['p1'], useSystemProxy: true })
    await wrapper.find('#use-ssh').setValue(false)

    const emitted = wrapper.emitted('update:modelValue') ?? []
    const payload = emitted[emitted.length - 1]?.[0] as {
      enabled?: boolean
      profileIds?: string[]
      useSystemProxy?: boolean
    }
    expect(payload.enabled).toBe(false)
    expect(payload.profileIds).toEqual([])
    expect(payload.useSystemProxy).toBe(false)
  })

  it('toggling "Use system proxy" patches useSystemProxy', async () => {
    const wrapper = mountPanel({ enabled: true })
    await wrapper.find('#ssh-system-proxy').setValue(true)

    const emitted = wrapper.emitted('update:modelValue') ?? []
    const payload = emitted[emitted.length - 1]?.[0] as {
      useSystemProxy?: boolean
    }
    expect(payload.useSystemProxy).toBe(true)
  })
})
