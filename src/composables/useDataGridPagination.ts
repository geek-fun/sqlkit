import type { Ref } from 'vue'
import { computed, ref, watch } from 'vue'

export type ResultPageSize = number | 'all'

export const useDataGridPagination = (rows: Ref<Record<string, unknown>[]>) => {
  const currentPage = ref(1)
  const pageSize = ref<ResultPageSize>(100)
  const pageCount = computed(() => pageSize.value === 'all'
    ? 1
    : Math.max(1, Math.ceil(rows.value.length / pageSize.value)))
  const offset = computed(() => pageSize.value === 'all' ? 0 : (currentPage.value - 1) * pageSize.value)
  const pageRows = computed(() => pageSize.value === 'all'
    ? rows.value
    : rows.value.slice(offset.value, offset.value + pageSize.value))

  const setPage = (page: number) => {
    if (Number.isSafeInteger(page))
      currentPage.value = Math.max(1, Math.min(pageCount.value, page))
  }

  const setPageSize = (size: ResultPageSize) => {
    if (size !== 'all' && (!Number.isSafeInteger(size) || size < 1))
      return
    pageSize.value = size
    currentPage.value = 1
  }

  watch(rows, () => {
    currentPage.value = 1
  }, { flush: 'sync' })

  watch(pageCount, () => {
    setPage(currentPage.value)
  }, { flush: 'sync' })

  return { currentPage, pageSize, pageCount, offset, pageRows, setPage, setPageSize }
}
