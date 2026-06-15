import type { Ref } from 'vue'
import { computed, ref } from 'vue'

export function useDataGridSelection() {
  const selectedRows: Ref<Set<number>> = ref(new Set())
  const lastClickedIndex: Ref<number | null> = ref(null)

  const toggleRow = (index: number, shiftKey = false) => {
    if (shiftKey && lastClickedIndex.value !== null) {
      // Range select
      const start = Math.min(lastClickedIndex.value, index)
      const end = Math.max(lastClickedIndex.value, index)
      const next = new Set<number>()
      for (let i = start; i <= end; i++)
        next.add(i)

      selectedRows.value = next
    }
    else {
      const next = new Set(selectedRows.value)
      if (next.has(index))
        next.delete(index)
      else
        next.add(index)

      selectedRows.value = next
    }
    lastClickedIndex.value = index
  }

  const toggleAll = (totalRows: number) => {
    if (selectedRows.value.size === totalRows) {
      selectedRows.value = new Set()
    }
    else {
      selectedRows.value = new Set(Array.from({ length: totalRows }, (_, i) => i))
    }
    lastClickedIndex.value = null
  }

  const isSelected = (index: number): boolean => {
    return selectedRows.value.has(index)
  }

  const isAllSelected = (totalRows: number): boolean => {
    return totalRows > 0 && selectedRows.value.size === totalRows
  }

  const selectedCount = computed(() => selectedRows.value.size)

  const clearSelection = () => {
    selectedRows.value = new Set()
    lastClickedIndex.value = null
  }

  const getSelectedRows = (rows: Record<string, unknown>[]): Record<string, unknown>[] => {
    return Array.from(selectedRows.value)
      .filter(i => i >= 0 && i < rows.length)
      .map(i => rows[i])
  }

  return {
    selectedRows,
    lastClickedIndex,
    toggleRow,
    toggleAll,
    isSelected,
    isAllSelected,
    selectedCount,
    clearSelection,
    getSelectedRows,
  }
}
