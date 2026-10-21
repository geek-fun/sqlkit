<script setup lang="ts">
import type { RenderNode } from './types'
import type { ColumnInfo } from '@/types/connection'
import { computed } from 'vue'

const props = defineProps<{
  node: RenderNode
  isSelected: boolean
  isHighlighted: boolean
  headerDragging: boolean
}>()

const emit = defineEmits<{
  headerMousedown: [e: MouseEvent]
  cardDblclick: []
  cardClick: []
  toggleExpand: []
}>()

const fkColumnNames = computed(() => {
  const names = new Set<string>()
  for (const fk of props.node.table.foreignKeys) {
    for (const col of fk.columns)
      names.add(col)
    for (const col of fk.referenced_columns)
      names.add(col)
  }
  return names
})

function isFKColumn(colName: string): boolean {
  return fkColumnNames.value.has(colName)
}

function pillClass(col: ColumnInfo): string {
  if (col.is_primary_key)
    return 'er-entity-pill--pk'
  if (isFKColumn(col.name))
    return 'er-entity-pill--fk'
  return ''
}

defineExpose({})
</script>

<template>
  <div
    class="er-entity"
    :class="{ 'er-entity--selected': isSelected }"
    @click.stop="emit('cardClick')"
    @dblclick.stop="emit('cardDblclick')"
  >
    <!-- Header (draggable) -->
    <div
      class="er-entity-header"
      :class="{ 'er-entity-header--dragging': headerDragging }"
      @mousedown="emit('headerMousedown', $event)"
    >
      <svg
        class="er-entity-icon"
        xmlns="http://www.w3.org/2000/svg"
        width="14" height="14"
        viewBox="0 0 24 24"
        fill="none" stroke="currentColor" stroke-width="2"
      >
        <path d="M3 3h18v18H3z" />
        <path d="M21 9H3" />
        <path d="M9 21V9" />
      </svg>
      {{ node.id }}
    </div>

    <!-- Column pills -->
    <div class="er-entity-pills">
      <span
        v-for="col in node.table.columns"
        :key="col.name"
        class="er-entity-pill" :class="[pillClass(col)]"
      >
        <svg
          v-if="col.is_primary_key"
          xmlns="http://www.w3.org/2000/svg"
          width="10" height="10"
          viewBox="0 0 24 24"
          fill="none" stroke="currentColor" stroke-width="2"
          class="er-entity-pill-icon"
        >
          <path d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z" />
          <circle cx="16.5" cy="7.5" r=".5" fill="currentColor" />
        </svg>
        <svg
          v-else-if="isFKColumn(col.name)"
          xmlns="http://www.w3.org/2000/svg"
          width="10" height="10"
          viewBox="0 0 24 24"
          fill="none" stroke="currentColor" stroke-width="2"
          class="er-entity-pill-icon"
        >
          <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
          <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
        </svg>
        <span class="er-entity-pill-name">{{ col.name }}</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.er-entity {
  @apply bg-card border-2 border-blue-500/50 rounded-xl shadow-sm;
  width: 260px;
  font-size: 12px;
  user-select: none;
  overflow: hidden;
}
.er-entity--selected {
  @apply border-primary ring-2 ring-primary/30;
}
.er-entity-header {
  @apply bg-blue-50 dark:bg-blue-950/30 px-3 py-2 font-semibold text-sm border-b border-blue-500/20 flex items-center gap-2;
  cursor: grab;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.er-entity-header--dragging {
  cursor: grabbing;
}
.er-entity-icon {
  @apply text-blue-500 shrink-0;
}
.er-entity-pills {
  @apply p-2 flex flex-wrap gap-1.5;
}
.er-entity-pill {
  @apply inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono border;
  max-width: 100%;
  @apply bg-muted/50 border-border/60 text-muted-foreground;
}
.er-entity-pill--pk {
  @apply bg-amber-50 dark:bg-amber-950/30 border-amber-300 dark:border-amber-700 text-amber-700 dark:text-amber-300;
}
.er-entity-pill--fk {
  @apply bg-blue-50 dark:bg-blue-950/30 border-blue-300 dark:border-blue-700 text-blue-700 dark:text-blue-300;
}
.er-entity-pill-name {
  @apply truncate;
}
.er-entity-pill-icon {
  @apply shrink-0;
}
</style>
