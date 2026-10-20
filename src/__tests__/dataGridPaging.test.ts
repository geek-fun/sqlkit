/** @jest-environment jsdom */
import { shallowMount } from '@vue/test-utils'
import { DataGrid } from '@/components/grid'

jest.mock('@tauri-apps/api/core', () => ({ invoke: jest.fn() }))
jest.mock('@vueuse/core', () => ({ useResizeObserver: jest.fn() }))
jest.mock('@/composables/useNotifications', () => ({ toast: { error: jest.fn() } }))
jest.mock('@/composables/useDataGridCopy', () => {
  const copyUtil = { copyRowsAs: jest.fn(), exportToFile: jest.fn() }
  return { useDataGridCopy: () => copyUtil }
})
jest.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string, params?: Record<string, unknown>) => `${key} ${JSON.stringify(params ?? {})}` }),
}))
jest.mock('@tanstack/vue-virtual', () => ({
  useVirtualizer: (options: { value: { count: number } }) => ({
    getTotalSize: () => options.value.count * 32,
    getVirtualItems: () => Array.from({ length: options.value.count }, (_, index) => ({ index, size: 32, start: index * 32 })),
  }),
}))
jest.mock('@/components/grid/DataGridPagination.vue', () => ({
  __esModule: true,
  default: { name: 'DataGridPagination', props: ['currentPage', 'pageCount', 'pageSize', 'disabled'], template: '<nav />' },
}))
jest.mock('@/components/grid/EditRowDialog.vue', () => ({ __esModule: true, default: { template: '<div />' } }))
jest.mock('@/components/grid/RowDetailsDialog.vue', () => ({
  __esModule: true,
  default: { name: 'RowDetailsDialog', props: ['row'], template: '<div />' },
}))
jest.mock('@/components/grid/JsonViewDialog.vue', () => ({ __esModule: true, default: { template: '<div />' } }))
jest.mock('@/components/grid/CellContextMenu.vue', () => ({ __esModule: true, default: { template: '<div />' } }))
jest.mock('@/components/grid/ColumnHeaderContextMenu.vue', () => ({ __esModule: true, default: { template: '<div />' } }))
jest.mock('@/components/grid/BatchActionBar.vue', () => ({ __esModule: true, default: { template: '<div />' } }))
jest.mock('@/components/grid/FilterBar.vue', () => ({ __esModule: true, default: { template: '<div />' } }))
jest.mock('@/components/ui/button', () => ({ Button: { name: 'Button', template: '<button><slot /></button>' } }))
jest.mock('@/components/ui/checkbox', () => ({ Checkbox: { name: 'Checkbox', template: '<input type="checkbox">' } }))
jest.mock('@/components/ui/dropdown-menu', () => ({}))
jest.mock('@/components/ui/alert-dialog', () => Object.fromEntries(
  ['AlertDialog', 'AlertDialogAction', 'AlertDialogCancel', 'AlertDialogContent', 'AlertDialogDescription', 'AlertDialogFooter', 'AlertDialogHeader', 'AlertDialogTitle']
    .map(name => [name, { name, template: '<div><slot /></div>' }]),
))

const rows = Array.from({ length: 235 }, (_, id) => ({ id: 235 - id }))
const mountGrid = () => shallowMount(DataGrid, {
  props: { columns: ['id'], rows, rowCount: rows.length, columnTypes: { id: 'integer' } },
  global: { renderStubDefaultSlot: true, mocks: { $t: (key: string) => key } },
})

describe('paged result grid', () => {
  it('renders the requested page and sorts only its rows without mutating the result', async () => {
    const wrapper = mountGrid()
    const pagination = wrapper.findComponent({ name: 'DataGridPagination' })
    expect(wrapper.findAll('[data-index]')).toHaveLength(100)
    expect(pagination.props('pageCount')).toBe(3)
    pagination.vm.$emit('pageChange', 3)
    await wrapper.vm.$nextTick()
    expect(wrapper.findAll('[data-index]')).toHaveLength(35)
    expect(wrapper.find('[data-index="0"]').text()).toContain('35')
    expect(wrapper.find('[role="status"]').text()).toContain('"start":201,"end":235,"total":"235"')
    const sortHeader = wrapper.findAll('button').find(button => button.text().includes('id'))!
    await sortHeader.trigger('click')
    expect(wrapper.find('[data-index="0"]').text()).toContain('1')
    expect(rows[0].id).toBe(235)
    expect(rows[200].id).toBe(35)
    const { invoke } = jest.requireMock('@tauri-apps/api/core')
    expect(invoke).not.toHaveBeenCalled()
  })

  it('resets selection and scroll when changing pages', async () => {
    const wrapper = mountGrid()
    const firstRow = wrapper.get('[data-index="0"]')
    firstRow.findComponent({ name: 'Checkbox' }).vm.$emit('update:checked', true)
    await wrapper.vm.$nextTick()
    expect(wrapper.get('[data-index="0"]').classes()).toContain('bg-primary/[0.08]')
    const scroller = wrapper.find('.overflow-auto').element as HTMLDivElement
    scroller.scrollTop = 320
    wrapper.findComponent({ name: 'DataGridPagination' }).vm.$emit('pageChange', 2)
    await wrapper.vm.$nextTick()
    expect(scroller.scrollTop).toBe(0)
    expect(wrapper.get('[data-index="0"]').classes()).not.toContain('bg-primary/[0.08]')
    expect(wrapper.get('[data-index="0"]').text()).toContain('135')
  })

  it('copies and exports every returned row while viewing a later page', async () => {
    const wrapper = mountGrid()
    wrapper.findComponent({ name: 'DataGridPagination' }).vm.$emit('pageChange', 3)
    await wrapper.vm.$nextTick()
    const buttons = wrapper.findAllComponents({ name: 'Button' })
    buttons[0].vm.$emit('click')
    buttons[3].vm.$emit('click')
    const copyUtil = jest.requireMock('@/composables/useDataGridCopy').useDataGridCopy()
    expect(copyUtil.copyRowsAs).toHaveBeenCalledWith(rows, ['id'], 'csv', undefined)
    expect(copyUtil.exportToFile).toHaveBeenCalledWith(rows, ['id'], 'csv', undefined)
    await wrapper.setProps({ truncated: true })
    expect(wrapper.get('[role="status"]').text()).toContain('status.showingLimited')
    expect(wrapper.get('[role="status"]').text()).toContain('"start":201,"end":235,"loaded":"235"')
  })

  it('shows all rows and resets to the first page when results are replaced', async () => {
    const wrapper = mountGrid()
    const pagination = wrapper.findComponent({ name: 'DataGridPagination' })
    pagination.vm.$emit('pageSizeChange', 'all')
    await wrapper.vm.$nextTick()
    expect(wrapper.findAll('[data-index]')).toHaveLength(235)
    pagination.vm.$emit('pageSizeChange', 50)
    pagination.vm.$emit('pageChange', 4)
    await wrapper.vm.$nextTick()
    expect(pagination.props('currentPage')).toBe(4)
    await wrapper.setProps({ rows: [{ id: 7 }], rowCount: 1 })
    expect(pagination.props('currentPage')).toBe(1)
    expect(pagination.props('pageSize')).toBe(50)
    expect(wrapper.findAll('[data-index]')).toHaveLength(1)
    await wrapper.setProps({ rows: [], rowCount: 0 })
    expect(pagination.exists()).toBe(true)
    expect(wrapper.get('[role="status"]').text()).toContain('"start":0,"end":0,"total":"0"')
    await wrapper.setProps({ loading: true })
    expect(pagination.props('disabled')).toBe(true)
    await wrapper.setProps({ loading: false, error: 'Failed' })
    expect(pagination.props('disabled')).toBe(true)
  })
})
