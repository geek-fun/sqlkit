<script setup lang="ts">
import type { ColumnFilter, FilterBarEmits, FilterOperator } from '@/types/grid'
import { useI18n } from 'vue-i18n'

defineProps<{
  filters: ColumnFilter[]
}>()

const emit = defineEmits<FilterBarEmits>()

const { t } = useI18n()

function operatorLabel(op: FilterOperator): string {
  const labels: Record<FilterOperator, string> = {
    eq: '=',
    neq: '≠',
    like: 'LIKE',
    gt: '>',
    lt: '<',
    gte: '≥',
    lte: '≤',
    between: '∈',
  }
  return labels[op]
}

function chipColor(op: FilterOperator): string {
  const colors: Record<FilterOperator, string> = {
    eq: 'bg-blue-500/15 text-blue-700 dark:text-blue-300',
    neq: 'bg-orange-500/15 text-orange-700 dark:text-orange-300',
    like: 'bg-purple-500/15 text-purple-700 dark:text-purple-300',
    gt: 'bg-green-500/15 text-green-700 dark:text-green-300',
    gte: 'bg-green-500/15 text-green-700 dark:text-green-300',
    lt: 'bg-yellow-600/15 text-yellow-700 dark:text-yellow-300',
    lte: 'bg-yellow-600/15 text-yellow-700 dark:text-yellow-300',
    between: 'bg-teal-500/15 text-teal-700 dark:text-teal-300',
  }
  return colors[op]
}
</script>

<template>
  <div
    v-if="filters.length > 0"
    class="px-3 py-1.5 border-b bg-muted/30 flex flex-wrap gap-1.5 min-h-[32px] items-center"
  >
    <span class="text-xs text-muted-foreground mr-1 whitespace-nowrap">{{ t('components.dataGrid.filter.activeFilters') }}:</span>
    <span
      v-for="f in filters"
      :key="f.column"
      class="text-xs font-medium px-2 py-0.5 rounded inline-flex gap-1 items-center"
      :class="chipColor(f.operator)"
    >
      <span class="font-semibold">{{ f.column }}</span>
      <span>{{ operatorLabel(f.operator) }}</span>
      <span class="max-w-[200px] truncate">{{ f.value }}<template v-if="f.value2"> – {{ f.value2 }}</template></span>
      <button
        class="text-sm leading-none ml-0.5 hover:opacity-70"
        :title="t('components.dataGrid.filter.clearFilter')"
        @click="emit('removeFilter', f.column)"
      >×</button>
    </span>
    <button
      class="text-xs text-muted-foreground ml-2 underline underline-offset-2 hover:text-foreground"
      @click="emit('clearAll')"
    >
      {{ t('components.dataGrid.filter.clearAll') }}
    </button>
  </div>
</template>
