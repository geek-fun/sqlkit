import type { Ref } from 'vue'
import { computed, ref, watch } from 'vue'

export type PageSize = number | 'all'

export const PAGE_SIZE_OPTIONS = [50, 100, 200, 500, 1000] as const
export const MAX_FETCH_PAGE_SIZE = 0xFFFF_FFFF

export const isValidPageSize = (size: PageSize, allowAll = false, maximum = Number.MAX_SAFE_INTEGER) =>
  size === 'all' ? allowAll : Number.isSafeInteger(size) && size > 0 && size <= maximum

export const usePagination = (
  total: Ref<number>,
  options: { allowAll?: boolean, maxPageSize?: number } = {},
) => {
  const currentPage = ref(1)
  const pageSize = ref<PageSize>(100)
  const pageCount = computed(() => pageSize.value === 'all'
    ? 1
    : Math.max(1, Math.ceil(total.value / pageSize.value)))
  const offset = computed(() => pageSize.value === 'all' ? 0 : (currentPage.value - 1) * pageSize.value)

  const setPage = (page: number) => {
    if (Number.isSafeInteger(page))
      currentPage.value = Math.max(1, Math.min(pageCount.value, page))
  }

  const resetPage = () => {
    currentPage.value = 1
  }

  const setPageSize = (size: PageSize) => {
    if (size === pageSize.value || !isValidPageSize(size, options.allowAll, options.maxPageSize))
      return
    pageSize.value = size
    resetPage()
  }

  watch(pageCount, () => setPage(currentPage.value), { flush: 'sync' })

  return { currentPage, pageSize, pageCount, offset, setPage, setPageSize, resetPage }
}
