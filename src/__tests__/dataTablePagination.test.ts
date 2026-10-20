/** @jest-environment jsdom */
import { invoke } from '@tauri-apps/api/core'
import { flushPromises, shallowMount } from '@vue/test-utils'
import DataTableView from '@/components/database-browser/DataTableView.vue'

jest.mock('@tauri-apps/api/core', () => ({ invoke: jest.fn() }))
jest.mock('@tauri-apps/plugin-dialog', () => ({ save: jest.fn() }))
jest.mock('@/components/pagination', () => ({
  PaginationControls: {
    name: 'PaginationControls',
    props: { currentPage: Number, pageCount: Number, pageSize: [Number, String], disabled: Boolean, allowAll: Boolean },
    template: '<footer><slot /></footer>',
  },
}))
jest.mock('@/components/ui/button', () => ({ Button: { name: 'Button', template: '<button><slot /></button>' } }))
jest.mock('@/components/ui/input', () => ({ Input: { name: 'Input', template: '<input>' } }))
jest.mock('@/components/ui/label', () => ({ Label: { template: '<label><slot /></label>' } }))
jest.mock('@/components/ui/spinner', () => ({ Spinner: { template: '<span />' } }))
jest.mock('@/components/ui/destructive-confirm-dialog', () => ({ DestructiveConfirmDialog: { template: '<div />' } }))
jest.mock('@/components/ui/dialog', () => Object.fromEntries(
  ['Dialog', 'DialogContent', 'DialogTitle'].map(name => [name, { template: '<div><slot /></div>' }]),
))
jest.mock('@/store', () => ({
  ConnectionStatus: { CONNECTED: 'connected' },
  useConnectionStore: () => ({ getConnectionStatus: () => 'connected' }),
}))
jest.mock('@/composables/useMinLoadingTime', () => ({
  useMinLoadingTime: () => ({ withMinLoadingTime: (operation: () => Promise<unknown>) => operation() }),
}))
jest.mock('@/composables/useNotifications', () => ({
  toast: { success: jest.fn(), warning: jest.fn(), error: jest.fn() },
}))
jest.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string, params?: Record<string, unknown>) => `${key} ${JSON.stringify(params ?? {})}` }),
}))

const mockInvoke = invoke as jest.Mock
let total = 235
const mountTable = () => shallowMount(DataTableView, {
  props: { connectionId: 'test', database: 'sample', tableName: 'items' },
  global: { renderStubDefaultSlot: true, mocks: { $t: (key: string) => key } },
})
const lastQuery = () => mockInvoke.mock.calls.filter(([command]) => command === 'get_table_data').slice(-1)[0][1].query

beforeEach(() => {
  total = 235
  mockInvoke.mockReset().mockImplementation(async (command, args) => {
    if (command === 'get_table_count')
      return total
    if (command === 'get_table_data') {
      const { limit, offset } = args.query
      return {
        columns: ['id'],
        columnTypes: ['integer'],
        rows: Array.from({ length: Math.max(0, Math.min(limit, total - offset)) }, (_, i) => ({ id: offset + i + 1 })),
      }
    }
    return []
  })
})

describe('table browsing with shared pagination', () => {
  it('keeps returned rows visible without claiming a total when the count fails', async () => {
    const normalInvoke = mockInvoke.getMockImplementation()!
    mockInvoke.mockImplementation((command, args) => command === 'get_table_count'
      ? Promise.reject(new Error('Count unavailable'))
      : normalInvoke(command, args))
    const wrapper = mountTable()
    await flushPromises()
    expect(wrapper.get('[role="status"]').text()).toContain('components.pagination.showingWithoutTotal {"start":1,"end":100}')
    wrapper.unmount()
  })

  it('fetches only the requested page and resets to page one for custom sizes', async () => {
    const wrapper = mountTable()
    await flushPromises()
    const controls = wrapper.findComponent({ name: 'PaginationControls' })
    expect(controls.props('pageCount')).toBe(3)
    expect(controls.props('allowAll')).toBe(false)
    expect(lastQuery()).toMatchObject({ limit: 100, offset: 0 })
    controls.vm.$emit('pageChange', 3)
    await flushPromises()
    expect(lastQuery()).toMatchObject({ limit: 100, offset: 200 })
    expect(wrapper.get('[role="status"]').text()).toContain('"start":201,"end":235,"total":"235"')
    controls.vm.$emit('pageSizeChange', 37)
    await flushPromises()
    expect(lastQuery()).toMatchObject({ limit: 37, offset: 0 })
    expect(controls.props('currentPage')).toBe(1)
    expect(controls.props('pageCount')).toBe(7)
    wrapper.unmount()
  })

  it('refreshes a shrinking final page using a valid offset and handles empty tables', async () => {
    const wrapper = mountTable()
    await flushPromises()
    const controls = wrapper.findComponent({ name: 'PaginationControls' })
    controls.vm.$emit('pageChange', 3)
    await flushPromises()
    total = 101
    const refresh = wrapper.findAll('button-stub').find(button => button.attributes('title')?.includes('refresh'))!
    await refresh.trigger('click')
    await flushPromises()
    expect(controls.props('currentPage')).toBe(2)
    expect(lastQuery()).toMatchObject({ offset: 100 })
    total = 0
    await refresh.trigger('click')
    await flushPromises()
    expect(controls.props('pageCount')).toBe(1)
    expect(controls.props('currentPage')).toBe(1)
    expect(wrapper.get('[role="status"]').text()).toContain('"start":0,"end":0,"total":"0"')
    wrapper.unmount()
  })

  it('prevents a second page fetch while a request is pending and disables controls on errors', async () => {
    const normalInvoke = mockInvoke.getMockImplementation()!
    let finish: (value: unknown) => void = () => {}
    mockInvoke.mockImplementation((command, args) => command === 'get_table_data'
      ? new Promise((resolve) => { finish = resolve })
      : normalInvoke(command, args))
    const wrapper = mountTable()
    await flushPromises()
    const controls = wrapper.findComponent({ name: 'PaginationControls' })
    expect(controls.props('disabled')).toBe(true)
    controls.vm.$emit('pageChange', 2)
    controls.vm.$emit('pageSizeChange', 50)
    await flushPromises()
    expect(mockInvoke.mock.calls.filter(([command]) => command === 'get_table_data')).toHaveLength(1)
    finish({ columns: ['id'], columnTypes: ['integer'], rows: [{ id: 1 }] })
    await flushPromises()
    expect(controls.props('disabled')).toBe(false)
    mockInvoke.mockRejectedValueOnce(new Error('Data unavailable'))
    controls.vm.$emit('pageChange', 2)
    await flushPromises()
    expect(controls.props('disabled')).toBe(true)
    wrapper.unmount()
  })
})
