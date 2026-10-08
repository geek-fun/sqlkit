import type { Ref } from 'vue'
import type { SortDirection, SortState } from '@/types/grid'
import { ref } from 'vue'

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

  /** Set an explicit direction for a column, keeping other sort rules intact. */
  const setSort = (column: string, direction: SortDirection) => {
    const existing = sortState.value.find(s => s.column === column)
    sortState.value = existing
      ? sortState.value.map(s => (s.column === column ? { ...s, direction } : s))
      : [...sortState.value, { column, direction }]
  }

  /** Drop sort rules for columns that no longer exist or are no longer sortable. */
  const pruneSort = (validColumns: readonly string[]) => {
    const pruned = sortState.value.filter(s => validColumns.includes(s.column))
    if (pruned.length !== sortState.value.length)
      sortState.value = pruned
  }

  const clearSort = () => {
    sortState.value = []
  }

  const getSortDirection = (column: string): SortDirection | null => {
    const s = sortState.value.find(ss => ss.column === column)
    return s ? s.direction : null
  }

  const getSortPriority = (column: string): number | null => {
    const idx = sortState.value.findIndex(s => s.column === column)
    return idx >= 0 ? idx + 1 : null
  }

  return {
    sortState,
    toggleSort,
    setSort,
    pruneSort,
    clearSort,
    getSortDirection,
    getSortPriority,
  }
}
