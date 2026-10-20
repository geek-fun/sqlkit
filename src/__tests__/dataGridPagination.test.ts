/** @jest-environment jsdom */
import { mount } from '@vue/test-utils'
import { PaginationControls } from '@/components/pagination'

jest.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
jest.mock('@/components/ui/button', () => ({
  Button: { template: '<button><slot /></button>' },
}))
jest.mock('@/components/ui/input', () => ({
  Input: {
    props: ['modelValue'],
    emits: ['update:modelValue'],
    template: '<input :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)">',
  },
}))
jest.mock('@/components/ui/dropdown-menu', () => ({
  DropdownMenu: { template: '<div><slot /></div>' },
  DropdownMenuContent: { template: '<div><slot /></div>' },
  DropdownMenuItem: { template: '<button @click="$emit(\'select\')"><slot /></button>' },
  DropdownMenuSeparator: { template: '<hr />' },
  DropdownMenuTrigger: { template: '<div><slot /></div>' },
}))

const mountPagination = () => mount(PaginationControls, {
  props: { currentPage: 2, pageCount: 5, pageSize: 100, disabled: false },
})

describe('result pagination controls', () => {
  it('navigates in all four directions and disables boundary buttons', async () => {
    const wrapper = mountPagination()
    const action = (name: string) => wrapper.get(`button[aria-label="components.pagination.${name}"]`)
    await action('firstPage').trigger('click')
    await action('previousPage').trigger('click')
    await action('nextPage').trigger('click')
    await action('lastPage').trigger('click')
    expect(wrapper.emitted('pageChange')).toEqual([[1], [1], [3], [5]])
    await wrapper.setProps({ currentPage: 1 })
    expect(action('firstPage').attributes('disabled')).toBeDefined()
    expect(action('previousPage').attributes('disabled')).toBeDefined()
    await wrapper.setProps({ currentPage: 5 })
    expect(action('nextPage').attributes('disabled')).toBeDefined()
    expect(action('lastPage').attributes('disabled')).toBeDefined()
  })

  it('validates direct page entry, clamps to the last page, and restores invalid input', async () => {
    const wrapper = mountPagination()
    const input = wrapper.get('input[aria-label="components.pagination.goToPage"]')
    await input.setValue('4')
    await input.trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('pageChange')).toEqual([[4]])
    await input.setValue('999')
    await input.trigger('blur')
    expect(wrapper.emitted('pageChange')?.slice(-1)[0]).toEqual([5])
    await input.setValue('1.5')
    await input.trigger('keydown', { key: 'Enter' })
    expect((input.element as HTMLInputElement).value).toBe('2')
    expect(wrapper.emitted('pageChange')).toHaveLength(2)
    await input.setValue('')
    await input.trigger('blur')
    expect((input.element as HTMLInputElement).value).toBe('2')
    await input.setValue('4')
    await input.trigger('keydown', { key: 'Escape' })
    expect((input.element as HTMLInputElement).value).toBe('2')
    await wrapper.setProps({ currentPage: 3 })
    expect((input.element as HTMLInputElement).value).toBe('3')
  })

  it('applies valid custom sizes and prevents interaction while disabled', async () => {
    const wrapper = mountPagination()
    const input = wrapper.get('input[aria-label="components.pagination.customSize"]')
    await input.setValue('37')
    await wrapper.get('form').trigger('submit')
    expect(wrapper.emitted('pageSizeChange')).toEqual([[37]])
    await input.setValue('0')
    await wrapper.get('form').trigger('submit')
    expect(wrapper.emitted('pageSizeChange')).toHaveLength(1)
    await wrapper.setProps({ disabled: true })
    await input.setValue('50')
    await wrapper.get('form').trigger('submit')
    expect(wrapper.emitted('pageSizeChange')).toHaveLength(1)
    expect(wrapper.findAll('nav > button').every(button => button.attributes('disabled') !== undefined)).toBe(true)
  })

  it('offers all returned rows only where supported and respects fetch size limits', async () => {
    const wrapper = mountPagination()
    expect(wrapper.text()).not.toContain('components.pagination.showAll')
    await wrapper.setProps({ allowAll: true })
    const all = wrapper.findAll('button').find(button => button.text().includes('components.pagination.showAll'))!
    await all.trigger('click')
    expect(wrapper.emitted('pageSizeChange')).toEqual([['all']])
    await wrapper.setProps({ maxPageSize: 1000 })
    const input = wrapper.get('input[aria-label="components.pagination.customSize"]')
    await input.setValue('1001')
    await wrapper.get('form').trigger('submit')
    expect(wrapper.emitted('pageSizeChange')).toHaveLength(1)
    await input.setValue('100')
    await wrapper.get('form').trigger('submit')
    expect(wrapper.emitted('pageSizeChange')).toHaveLength(1)
  })
})
