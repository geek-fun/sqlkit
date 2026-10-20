import { effectScope, nextTick, ref } from 'vue'
import { useDataGridPagination } from '@/composables/pagination'

const makeRows = (count: number) => Array.from({ length: count }, (_, id) => ({ id }))

describe('result pagination', () => {
  it('covers every row exactly once, including a partial last page', () => {
    const rows = ref(makeRows(235))
    const scope = effectScope()
    const pagination = scope.run(() => useDataGridPagination(rows))!
    expect(pagination.pageCount.value).toBe(3)
    expect(pagination.pageRows.value).toEqual(rows.value.slice(0, 100))
    pagination.setPage(2)
    expect(pagination.offset.value).toBe(100)
    expect(pagination.pageRows.value).toEqual(rows.value.slice(100, 200))
    pagination.setPage(3)
    expect(pagination.pageRows.value).toEqual(rows.value.slice(200))
    pagination.setPage(999)
    expect(pagination.currentPage.value).toBe(3)
    pagination.setPage(-1)
    expect(pagination.currentPage.value).toBe(1)
    scope.stop()
  })

  it('supports custom sizes and all rows, resetting the page when size changes', () => {
    const rows = ref(makeRows(200))
    const scope = effectScope()
    const pagination = scope.run(() => useDataGridPagination(rows))!
    pagination.setPage(2)
    pagination.setPageSize(37)
    expect(pagination.currentPage.value).toBe(1)
    expect(pagination.pageCount.value).toBe(6)
    expect(pagination.pageRows.value).toHaveLength(37)
    pagination.setPageSize('all')
    expect(pagination.pageCount.value).toBe(1)
    expect(pagination.pageRows.value).toEqual(rows.value)
    pagination.setPageSize(100)
    expect(pagination.pageCount.value).toBe(2)
    expect(pagination.pageRows.value).toHaveLength(100)
    scope.stop()
  })

  it('ignores unsafe and invalid sizes and page numbers', () => {
    const scope = effectScope()
    const pagination = scope.run(() => useDataGridPagination(ref(makeRows(235))))!
    ;[0, -1, 1.5, Number.NaN, Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1].forEach((size) => {
      pagination.setPageSize(size)
      expect(pagination.pageSize.value).toBe(100)
    })
    ;[1.5, Number.NaN, Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1].forEach((page) => {
      pagination.setPage(page)
      expect(pagination.currentPage.value).toBe(1)
    })
    scope.stop()
  })

  it('resets when results are replaced and handles zero rows or shrinking results', async () => {
    const rows = ref(makeRows(235))
    const scope = effectScope()
    const pagination = scope.run(() => useDataGridPagination(rows))!
    pagination.setPage(3)
    rows.value = makeRows(235)
    expect(pagination.currentPage.value).toBe(1)
    pagination.setPage(3)
    rows.value = []
    expect(pagination.currentPage.value).toBe(1)
    expect(pagination.pageCount.value).toBe(1)
    expect(pagination.pageRows.value).toEqual([])
    rows.value = makeRows(235)
    pagination.setPage(3)
    rows.value.length = 50
    await nextTick()
    expect(pagination.currentPage.value).toBe(1)
    expect(pagination.pageRows.value).toHaveLength(50)
    scope.stop()
  })
})
