/** @jest-environment jsdom */
import type { QueryResult } from '@/store/tabStore'
import { flushPromises, shallowMount } from '@vue/test-utils'
import QueryResultPanel from '@/components/database-browser/QueryResultPanel.vue'

jest.mock('@tauri-apps/api/core', () => ({ invoke: jest.fn() }))
jest.mock('@/store', () => ({ useConnectionStore: () => ({ getConnectionById: () => ({ type: 'postgresql' }) }) }))
jest.mock('@/composables/useDataGridCopy', () => ({
  useDataGridCopy: () => ({ copyRowsAs: jest.fn(), exportToFile: jest.fn() }),
}))
jest.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
jest.mock('@/components/explain-plan', () => ({ ExplainPlanPanel: { template: '<div />' } }))
jest.mock('@/components/ui/button', () => ({ Button: { template: '<button><slot /></button>' } }))
jest.mock('@/components/ui/spinner', () => ({ Spinner: { template: '<span />' } }))
jest.mock('@/components/ui/tooltip', () => ({
  Tooltip: { template: '<div><slot /></div>' },
  TooltipContent: { template: '<div><slot /></div>' },
  TooltipProvider: { template: '<div><slot /></div>' },
  TooltipTrigger: { template: '<div><slot /></div>' },
}))
jest.mock('@/components/ui/select', () => ({
  Select: { name: 'Select', props: ['modelValue'], emits: ['update:modelValue', 'update:open'], template: '<div><slot /></div>' },
  SelectContent: { template: '<div><slot /></div>' },
  SelectItem: { template: '<div><slot /></div>' },
  SelectTrigger: { template: '<div><slot /></div>' },
  SelectValue: { template: '<span />' },
}))
jest.mock('@/components/ui/dropdown-menu', () => ({
  DropdownMenu: { template: '<div><slot /></div>' },
  DropdownMenuContent: { template: '<div><slot /></div>' },
  DropdownMenuItem: { template: '<button><slot /></button>' },
  DropdownMenuSeparator: { template: '<hr />' },
  DropdownMenuTrigger: { template: '<div><slot /></div>' },
}))
jest.mock('@/components/grid/DataGrid.vue', () => ({
  __esModule: true,
  default: {
    name: 'DataGrid',
    props: ['columns', 'rows', 'columnTypes', 'filterable', 'tableName'],
    emits: ['filter-change'],
    template: '<div class="test-grid">{{ columns }} {{ rows }}</div>',
  },
}))

const resultSet = (column: string, rows: Record<string, unknown>[]): QueryResult => ({
  columns: [column],
  columnTypes: ['text'],
  rows,
})

const first = resultSet('email', [{ email: 'test@example.com' }])
const empty = resultSet('refund_id', [])
const last = resultSet('balance', [{ balance: '2700' }])
const scriptResult = { ...last, resultSets: [first, empty, last] }

const mountPanel = (results: QueryResult | null = scriptResult) => shallowMount(QueryResultPanel, {
  props: { results, sql: 'BEGIN; SELECT 1; COMMIT;', connectionId: 'test' },
  global: { stubs: { DataGrid: false } },
})

describe('script query results', () => {
  it('switches between every result, including empty results with column metadata', async () => {
    const wrapper = mountPanel()
    await flushPromises()
    expect(wrapper.findAll('[role=tab]')).toHaveLength(0)
    expect(wrapper.find('.result-toolbar').findComponent({ name: 'Select' }).exists()).toBe(true)
    expect(wrapper.find('.query-result-panel').element.children).toHaveLength(3)
    const grid = () => wrapper.findComponent({ name: 'DataGrid' })
    expect(grid().props('columns')).toEqual(['email'])
    expect(grid().props('filterable')).toBe(false)
    expect(grid().props('tableName')).toBeUndefined()

    wrapper.findComponent({ name: 'Select' }).vm.$emit('update:modelValue', '1')
    await wrapper.vm.$nextTick()
    expect(grid().props('columns')).toEqual(['refund_id'])
    expect(grid().props('rows')).toEqual([])

    wrapper.findComponent({ name: 'Select' }).vm.$emit('update:modelValue', '2')
    await wrapper.vm.$nextTick()
    expect(grid().props('rows')).toEqual([{ balance: '2700' }])
    expect(grid().props('columnTypes')).toEqual({ balance: 'text' })
  })

  it('resets selection for a new execution and removes stale results on failure', async () => {
    const wrapper = mountPanel()
    wrapper.findComponent({ name: 'Select' }).vm.$emit('update:modelValue', '2')
    await wrapper.vm.$nextTick()
    await wrapper.setProps({ results: { ...scriptResult } })
    expect(wrapper.findComponent({ name: 'DataGrid' }).props('columns')).toEqual(['email'])
    await wrapper.setProps({ results: null, error: 'division by zero' })
    expect(wrapper.findComponent({ name: 'DataGrid' }).exists()).toBe(false)
    expect(wrapper.text()).toContain('division by zero')
  })

  it('never re-executes a script when a filter event is emitted', async () => {
    const { invoke } = jest.requireMock('@tauri-apps/api/core')
    const wrapper = mountPanel()
    await flushPromises()
    invoke.mockClear()
    wrapper.findComponent({ name: 'DataGrid' }).vm.$emit('filter-change', [{ column: 'email', operator: 'eq', value: 'test' }])
    await flushPromises()
    expect(invoke).not.toHaveBeenCalled()
  })

  it('returns from Explain to the selected result when the selector opens', async () => {
    const wrapper = mountPanel()
    await wrapper.setProps({ explainPlan: {} as never })
    expect(wrapper.findComponent({ name: 'DataGrid' }).exists()).toBe(false)
    wrapper.findComponent({ name: 'Select' }).vm.$emit('update:open', true)
    await wrapper.vm.$nextTick()
    expect(wrapper.findComponent({ name: 'DataGrid' }).props('columns')).toEqual(['email'])
  })

  it('keeps ordinary single-query results filterable without a result selector', () => {
    const wrapper = mountPanel(last)
    expect(wrapper.findComponent({ name: 'Select' }).exists()).toBe(false)
    expect(wrapper.findComponent({ name: 'DataGrid' }).props('rows')).toEqual(last.rows)
    expect(wrapper.findComponent({ name: 'DataGrid' }).props('filterable')).toBe(true)
  })
})
