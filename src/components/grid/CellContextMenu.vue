<script setup lang="ts">
import type { CellContextMenuEmits, ColumnFilter } from '@/types/grid'
import { onClickOutside } from '@vueuse/core'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useDataGridCopy } from '@/composables/useDataGridCopy'

const props = defineProps<{
  show: boolean
  x: number
  y: number
  value: unknown
  column: string
  columnType?: string
  row: Record<string, unknown> | null
  columns: string[]
  tableName?: string
}>()

const emit = defineEmits<CellContextMenuEmits & {
  (e: 'close'): void
  (e: 'filter', filter: ColumnFilter): void
}>()

const { t } = useI18n()
const { copyCellValue, copyRowsAs } = useDataGridCopy()

const menuRef = ref<HTMLElement>()

onClickOutside(menuRef, () => emit('close'))

const adjustedX = Math.min(props.x, window.innerWidth - 220)
const adjustedY = Math.min(props.y, window.innerHeight - 340)

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape')
    emit('close')
}

function closeAnd(fn: () => void) {
  fn()
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="show"
      ref="menuRef"
      class="fixed z-50"
      :style="{ left: `${adjustedX}px`, top: `${adjustedY}px` }"
      @keydown="handleKeydown"
    >
      <DropdownMenu :open="true">
        <DropdownMenuTrigger />
        <DropdownMenuContent align="start" class="min-w-44" @keydown.escape="emit('close')">
          <DropdownMenuItem @click="closeAnd(() => copyCellValue(value))">
            <span class="i-carbon-copy mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.cell.copyValue') }}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem @click="closeAnd(() => { if (row) copyRowsAs([row], columns, 'csv', tableName) })">
            <span class="i-carbon-table-split mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.row.copyAsCsv') }}
          </DropdownMenuItem>
          <DropdownMenuItem @click="closeAnd(() => { if (row) copyRowsAs([row], columns, 'json', tableName) })">
            <span class="i-carbon-code mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.row.copyAsJson') }}
          </DropdownMenuItem>
          <DropdownMenuItem @click="closeAnd(() => { if (row) copyRowsAs([row], columns, 'insert', tableName) })">
            <span class="i-carbon-sql mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.row.copyAsInsert') }}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            @click="closeAnd(() => emit('filter', { column, operator: 'eq', value: String(value) }))"
          >
            <span class="i-carbon-filter mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.cell.filterByValue') }}
          </DropdownMenuItem>
          <DropdownMenuItem
            @click="closeAnd(() => emit('filter', { column, operator: 'neq', value: String(value) }))"
          >
            <span class="i-carbon-filter-remove mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.cell.excludeValue') }}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  </Teleport>
</template>
