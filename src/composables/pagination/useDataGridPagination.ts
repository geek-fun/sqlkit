import type { Ref } from 'vue'
import { computed, watch } from 'vue'
import { usePagination } from './usePagination'

export const useDataGridPagination = (rows: Ref<Record<string, unknown>[]>) => {
  const pagination = usePagination(computed(() => rows.value.length), { allowAll: true })
  const { pageSize, offset } = pagination
  const pageRows = computed(() => pageSize.value === 'all'
    ? rows.value
    : rows.value.slice(offset.value, offset.value + pageSize.value))

  watch(rows, pagination.resetPage, { flush: 'sync' })

  return { ...pagination, pageRows }
}
