import type { Ref } from 'vue'
import type { ColumnFilter, FilterState } from '@/types/grid'
import { computed, ref } from 'vue'

const escapeSqlValue = (value: string): string => value.replace(/'/g, '\'\'')

export function useDataGridFilter() {
  const filters: Ref<FilterState> = ref([])

  const addFilter = (filter: ColumnFilter) => {
    const existing = filters.value.findIndex(f => f.column === filter.column)
    if (existing >= 0) {
      const next = [...filters.value]
      next[existing] = filter
      filters.value = next
    }
    else {
      filters.value = [...filters.value, filter]
    }
  }

  const removeFilter = (column: string) => {
    filters.value = filters.value.filter(f => f.column !== column)
  }

  const clearAllFilters = () => {
    filters.value = []
  }

  const hasActiveFilters = computed(() => filters.value.length > 0)

  const hasFilter = (column: string): boolean => {
    return filters.value.some(f => f.column === column)
  }

  const getFilter = (column: string): ColumnFilter | null => {
    return filters.value.find(f => f.column === column) ?? null
  }

  const buildWhereClause = (validColumns: string[]): string | null => {
    const valid = filters.value.filter(f => validColumns.includes(f.column))
    if (valid.length === 0)
      return null

    const clauses = valid.map((f) => {
      const escaped = escapeSqlValue(f.value)
      switch (f.operator) {
        case 'eq':
          return `${f.column} = '${escaped}'`
        case 'neq':
          return `${f.column} != '${escaped}'`
        case 'like':
          return `${f.column} LIKE '%${escaped}%'`
        case 'gt':
          return `${f.column} > '${escaped}'`
        case 'lt':
          return `${f.column} < '${escaped}'`
        case 'gte':
          return `${f.column} >= '${escaped}'`
        case 'lte':
          return `${f.column} <= '${escaped}'`
        case 'between':
          return `${f.column} >= '${escaped}' AND ${f.column} <= '${escapeSqlValue(f.value2 ?? '')}'`
        default:
          return ''
      }
    }).filter(Boolean)

    return clauses.length > 0 ? clauses.join(' AND ') : null
  }

  return {
    filters,
    addFilter,
    removeFilter,
    clearAllFilters,
    hasActiveFilters,
    buildWhereClause,
    hasFilter,
    getFilter,
  }
}
