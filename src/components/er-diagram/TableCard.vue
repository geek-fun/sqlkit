<script setup lang="ts">
import type { RenderNode } from './types'

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

function isFKColumn(colName: string): boolean {
  return props.node.table.foreignKeys.some(
    fk => fk.columns.includes(colName) || fk.referenced_columns.includes(colName),
  )
}
</script>

<template>
  <div
    class="er-table-card"
    :class="{ 'er-table-card--selected': isSelected }"
    @click.stop="emit('cardClick')"
    @dblclick.stop="emit('cardDblclick')"
  >
    <!-- Header: only this triggers drag -->
    <div
      class="er-table-header"
      :class="{ 'er-table-header--dragging': headerDragging }"
      @mousedown="emit('headerMousedown', $event)"
    >
      {{ node.id }}
    </div>

    <!-- Columns list -->
    <div class="er-table-columns">
      <div
        v-for="col in node.visibleColumns"
        :key="col.name"
        class="er-table-column"
      >
        <span class="er-column-markers">
          <span v-if="col.is_primary_key" class="er-pk">
            <!-- key SVG -->
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z" /><circle cx="16.5" cy="7.5" r=".5" fill="currentColor" /></svg>
          </span>
          <span v-else-if="isFKColumn(col.name)" class="er-fk">
            <!-- link SVG -->
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" /><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" /></svg>
          </span>
          <span v-else class="er-col-spacer" />
        </span>
        <span class="er-column-name">{{ col.name }}</span>
        <span class="er-column-type">{{ col.data_type }}</span>
      </div>
    </div>

    <!-- Show more / less toggle -->
    <button
      v-if="node.showExpandButton"
      class="er-expand-btn"
      @click.stop="emit('toggleExpand')"
    >
      {{ node.isExpanded ? $t('components.databaseBrowser.erDiagram.hideExtraColumns') : $t('components.databaseBrowser.erDiagram.showAllColumns', { count: node.table.columns.length }) }}
    </button>
  </div>
</template>

<style scoped>
.er-table-card {
  @apply bg-card border border-border rounded-lg shadow-sm;
  width: 220px;
  font-size: 12px;
  user-select: none;
}
.er-table-card--selected {
  @apply border-primary ring-1 ring-primary shadow-md;
}
.er-table-header {
  @apply bg-muted px-3 py-2 font-semibold text-sm border-b border-border;
  cursor: grab;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.er-table-header--dragging {
  cursor: grabbing;
}
.er-table-columns {
  @apply divide-y divide-border/50;
}
.er-table-column {
  @apply px-3 py-1.5 flex gap-1 items-center text-xs;
}
.er-column-name {
  @apply font-mono flex-1 truncate;
}
.er-column-type {
  @apply text-muted-foreground text-[10px] flex-shrink-0;
  max-width: 80px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.er-column-markers {
  @apply flex gap-0.5 flex-shrink-0 w-4 items-center justify-center;
}
.er-pk {
  @apply text-amber-500 flex-shrink-0;
}
.er-fk {
  @apply text-blue-500 flex-shrink-0;
}
.er-col-spacer {
  @apply block w-3;
}
.er-expand-btn {
  @apply w-full text-xs text-muted-foreground hover:text-foreground py-1.5 border-t border-border bg-muted/30 hover:bg-muted/50 transition-colors;
}
</style>
