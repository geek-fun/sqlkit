import { effectScope, ref } from 'vue'
import { MAX_FETCH_PAGE_SIZE, usePagination } from '@/composables/pagination'

describe('shared pagination state', () => {
  it.each([
    [0, 100, 1, 1, 0],
    [200, 100, 2, 2, 100],
    [1432, 100, 15, 15, 1400],
    [501, 500, 2, 2, 500],
  ])('pages %i rows at size %i consistently', (total, size, page, count, offset) => {
    const scope = effectScope()
    const pages = scope.run(() => usePagination(ref(total)))!
    pages.setPageSize(size)
    pages.setPage(page)
    expect(pages.pageCount.value).toBe(count)
    expect(pages.offset.value).toBe(offset)
    scope.stop()
  })

  it('clamps a shrinking table to its last valid page and keeps empty tables on page one', () => {
    const total = ref(235)
    const scope = effectScope()
    const pages = scope.run(() => usePagination(total))!
    pages.setPage(3)
    expect(pages.offset.value).toBe(200)
    total.value = 101
    expect(pages.currentPage.value).toBe(2)
    total.value = 0
    expect(pages.currentPage.value).toBe(1)
    expect(pages.pageCount.value).toBe(1)
    expect(pages.offset.value).toBe(0)
    scope.stop()
  })

  it('shares numeric sizes and resets across local and fetched data while reserving all for local data', () => {
    const scope = effectScope()
    const local = scope.run(() => usePagination(ref(235), { allowAll: true }))!
    const remote = scope.run(() => usePagination(ref(235), { maxPageSize: MAX_FETCH_PAGE_SIZE }))!
    ;[local, remote].forEach((pages) => {
      pages.setPage(3)
      pages.setPageSize(100)
      expect(pages.currentPage.value).toBe(3)
      pages.setPageSize(37)
      expect(pages.currentPage.value).toBe(1)
      expect(pages.pageCount.value).toBe(7)
      pages.setPage(999)
      expect(pages.currentPage.value).toBe(7)
      pages.setPageSize(0)
      expect(pages.pageSize.value).toBe(37)
    })
    remote.setPageSize(MAX_FETCH_PAGE_SIZE + 1)
    remote.setPageSize('all')
    expect(remote.pageSize.value).toBe(37)
    local.setPageSize('all')
    expect(local.pageCount.value).toBe(1)
    expect(local.offset.value).toBe(0)
    scope.stop()
  })
})
