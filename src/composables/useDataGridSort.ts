import type { Ref } from 'vue'
import type { SortDirection, SortState } from '@/types/grid'
import { computed, ref } from 'vue'

export function useDataGridSort() {
  const sortState: Ref<SortState> = ref([])

  const toggleSort = (column: string, shiftKey = false) => {
    const current = sortState.value
    const existing = current.find(s => s.column === column)

    if (shiftKey) {
      // Multi-column sort
      if (existing) {
        // Cycle ASC → DESC → remove
        if (existing.direction === 'ASC') {
          sortState.value = current.map(s =>
            s.column === column ? { ...s, direction: 'DESC' as SortDirection } : s,
          )
        }
        else {
          sortState.value = current.filter(s => s.column !== column)
        }
      }
      else {
        sortState.value = [...current, { column, direction: 'ASC' }]
      }
    }
    else {
      // Single-column sort
      if (existing) {
        if (existing.direction === 'ASC') {
          sortState.value = [{ column, direction: 'DESC' }]
        }
        else {
          sortState.value = []
        }
      }
      else {
        sortState.value = [{ column, direction: 'ASC' }]
      }
    }
  }

  const clearSort = () => {
    sortState.value = []
  }

  const buildOrderByClause = (validColumns: string[]): string | null => {
    const valid = sortState.value.filter(s => validColumns.includes(s.column))
    if (valid.length === 0)
      return null
    return valid.map(s => `${s.column} ${s.direction}`).join(', ')
  }

  const getSortDirection = (column: string): SortDirection | null => {
    const s = sortState.value.find(ss => ss.column === column)
    return s ? s.direction : null
  }

  const getSortPriority = (column: string): number | null => {
    const idx = sortState.value.findIndex(s => s.column === column)
    return idx >= 0 ? idx + 1 : null
  }

  const hasActiveSort = computed(() => sortState.value.length > 0)

  return {
    sortState,
    toggleSort,
    clearSort,
    buildOrderByClause,
    getSortDirection,
    getSortPriority,
    hasActiveSort,
  }
}
