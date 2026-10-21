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
      class="text-xs text-secondary-foreground font-medium px-2 py-0.5 border border-border rounded bg-secondary inline-flex gap-1 items-center"
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
