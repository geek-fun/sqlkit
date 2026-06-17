<script setup lang="ts">
import type { ColumnFilter, DataGridEmits } from '@/types/grid'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useDataGridCopy } from '@/composables/useDataGridCopy'
import { useDataGridFilter } from '@/composables/useDataGridFilter'
import { useDataGridSelection } from '@/composables/useDataGridSelection'
import { useDataGridSort } from '@/composables/useDataGridSort'
import { toast } from '@/composables/useNotifications'
import BatchActionBar from './BatchActionBar.vue'
import CellContextMenu from './CellContextMenu.vue'
import ColumnHeaderContextMenu from './ColumnHeaderContextMenu.vue'
import EditRowDialog from './EditRowDialog.vue'
import FilterBar from './FilterBar.vue'
import JsonViewDialog from './JsonViewDialog.vue'
import RowDetailsDialog from './RowDetailsDialog.vue'

type Props = {
  columns: string[]
  rows: Record<string, unknown>[]
  rowCount: number
  executionTimeMs?: number
  columnTypes?: Record<string, string>
  primaryKeys?: string[]
  loading?: boolean
  error?: string | null
  tableName?: string
  database?: string
  schema?: string
  connectionId?: string
  hideToolbar?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  executionTimeMs: undefined,
  columnTypes: () => ({}),
  primaryKeys: () => [],
  loading: false,
  error: null,
  tableName: undefined,
  database: undefined,
  schema: undefined,
  connectionId: undefined,
  hideToolbar: false,
})

const emit = defineEmits<DataGridEmits>()

const { t } = useI18n({ useScope: 'global' })
const sort = useDataGridSort()
const filter = useDataGridFilter()
const selection = useDataGridSelection()
const copyUtil = useDataGridCopy()

// ── Virtual Scroller ──
const scrollContainer = ref<HTMLDivElement | null>(null)

const rowVirtualizer = useVirtualizer(
  computed(() => ({
    count: props.rows.length,
    getScrollElement: () => scrollContainer.value,
    estimateSize: () => 32,
    overscan: 10,
    enabled: props.rows.length > 0,
  })),
)

// ── Column Widths ──
const columnWidths = ref<Map<string, number>>(new Map())
const defaultColumnWidth = 150

function getColumnWidth(col: string): number {
  return columnWidths.value.get(col) ?? defaultColumnWidth
}

let resizeStartX = 0
let resizeStartWidth = 0
let resizeColumn = ''

function startColumnResize(e: MouseEvent, col: string) {
  resizeStartX = e.clientX
  resizeStartWidth = getColumnWidth(col)
  resizeColumn = col
  document.addEventListener('mousemove', handleColumnResize)
  document.addEventListener('mouseup', stopColumnResize)
  e.preventDefault()
}

function handleColumnResize(e: MouseEvent) {
  if (!resizeColumn)
    return
  const diff = e.clientX - resizeStartX
  const newWidth = Math.max(30, resizeStartWidth + diff)
  const next = new Map(columnWidths.value)
  next.set(resizeColumn, newWidth)
  columnWidths.value = next
}

function stopColumnResize() {
  resizeColumn = ''
  document.removeEventListener('mousemove', handleColumnResize)
  document.removeEventListener('mouseup', stopColumnResize)
}

// ── Dialogs ──
const editDialogOpen = ref(false)
const editingRowIndex = ref<number | null>(null)
const isDuplicateRow = ref(false)

const detailsDialogOpen = ref(false)
const detailsRowIndex = ref<number | null>(null)

const jsonDialogOpen = ref(false)
const jsonDialogValue = ref<unknown>(null)
const jsonDialogColumn = ref('')

// Delete dialog
const deleteDialogOpen = ref(false)
const deletingRowIndex = ref<number | null>(null)
const isDeleting = ref(false)

// ── Context Menus ──
const cellMenu = ref({
  show: false,
  x: 0,
  y: 0,
  value: null as unknown,
  column: '',
  columnType: '',
  row: null as Record<string, unknown> | null,
  rowIndex: -1,
})
const headerMenu = ref({
  show: false,
  x: 0,
  y: 0,
  column: '',
  columnType: '',
  hasActiveFilter: false,
  currentSortDirection: null as import('@/types/grid').SortDirection | null,
})

function openCellContextMenu(e: MouseEvent, rowIndex: number, col: string) {
  cellMenu.value = {
    show: true,
    x: e.clientX,
    y: e.clientY,
    value: props.rows[rowIndex]?.[col] ?? null,
    column: col,
    columnType: props.columnTypes?.[col] ?? '',
    row: props.rows[rowIndex] ?? null,
    rowIndex,
  }
}

function openHeaderContextMenu(e: MouseEvent, col: string) {
  headerMenu.value = {
    show: true,
    x: e.clientX,
    y: e.clientY,
    column: col,
    columnType: props.columnTypes?.[col] ?? '',
    hasActiveFilter: filter.hasFilter(col),
    currentSortDirection: sort.getSortDirection(col),
  }
}

// ── Event Handlers ──
function handleSortFromHeader(column: string, direction: import('@/types/grid').SortDirection) {
  sort.clearSort()
  sort.toggleSort(column)
  if (sort.getSortDirection(column) !== direction)
    sort.toggleSort(column)
  emit('sortChange', sort.sortState.value)
}

function handleClearSort() {
  sort.clearSort()
  emit('sortChange', [])
}

function handleAddFilter(f: ColumnFilter) {
  filter.addFilter(f)
  emit('filterChange', filter.filters.value)
}

function handleRemoveFilter(column: string) {
  filter.removeFilter(column)
  emit('filterChange', filter.filters.value)
}

function handleClearAllFilters() {
  filter.clearAllFilters()
  emit('filterChange', [])
}

function handleClearFilter(column: string) {
  filter.removeFilter(column)
  emit('filterChange', filter.filters.value)
}

// ── Row Operations ──
function openEditDialog(rowIndex: number) {
  editingRowIndex.value = rowIndex
  isDuplicateRow.value = false
  editDialogOpen.value = true
}

function openDuplicateDialog(rowIndex: number) {
  editingRowIndex.value = rowIndex
  isDuplicateRow.value = true
  editDialogOpen.value = true
}

function openDetailsDialog(rowIndex: number) {
  detailsRowIndex.value = rowIndex
  detailsDialogOpen.value = true
}

function openJsonDialog(value: unknown, column: string) {
  jsonDialogValue.value = value
  jsonDialogColumn.value = column
  jsonDialogOpen.value = true
}

function openDeleteDialog(rowIndex: number) {
  deletingRowIndex.value = rowIndex
  deleteDialogOpen.value = true
}

async function confirmDelete() {
  if (deletingRowIndex.value === null)
    return
  isDeleting.value = true
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const row = props.rows[deletingRowIndex.value]
    const pkValues = Object.fromEntries(
      props.primaryKeys.map(col => [col, row?.[col] ?? null]),
    )
    await invoke('delete_table_row', {
      connectionId: props.connectionId,
      database: props.database ?? null,
      table: props.tableName,
      schema: props.schema ?? null,
      pkValues,
    })
    toast.success(`${t('components.dataGrid.delete.title')} → ${t('common.status.success')}`)
    deleteDialogOpen.value = false
    deletingRowIndex.value = null
    emit('refresh')
  }
  catch (err) {
    toast.error(String(err))
  }
  finally {
    isDeleting.value = false
  }
}

function onEditSaved() {
  editDialogOpen.value = false
  emit('refresh')
}

const currentEditingRow = computed(() => {
  if (editingRowIndex.value === null)
    return null
  if (isDuplicateRow.value) {
    // Clone the row but remove PK values
    const row = props.rows[editingRowIndex.value]
    if (!row)
      return null
    const clone = { ...row }
    for (const pk of props.primaryKeys) delete clone[pk]
    return clone
  }
  return props.rows[editingRowIndex.value] ?? null
})

const currentDetailsRow = computed(() => {
  if (detailsRowIndex.value === null)
    return null
  return props.rows[detailsRowIndex.value] ?? null
})

// ── Type-aware Cell Formatting ──
const isNullValue = (v: unknown): boolean => v === null || v === undefined

function isNumericType(type?: string): boolean {
  if (!type)
    return false
  const t = type.toUpperCase()
  return /^(?:INT|BIGINT|SMALLINT|TINYINT|SERIAL|BIGSERIAL|FLOAT|DOUBLE|REAL|DECIMAL|NUMERIC|MONEY|NUMBER)/.test(t)
}

function isBooleanType(type?: string): boolean {
  if (!type)
    return false
  return /^(?:BOOL|BIT)/i.test(type)
}

function isDateType(type?: string): boolean {
  if (!type)
    return false
  const t = type.toUpperCase()
  return t === 'DATE' || /^(?:TIMESTAMP|DATETIME2?)/.test(t)
}

function isJsonType(type?: string): boolean {
  if (!type)
    return false
  return /^JSONB?/.test(type.toUpperCase())
}

function isBlobType(type?: string): boolean {
  if (!type)
    return false
  return /^(?:BYTEA|BLOB|BINARY|VARBINARY)/i.test(type)
}

function formatNumber(v: unknown, type?: string): string {
  if (isNullValue(v))
    return ''
  const num = Number(v)
  if (Number.isNaN(num))
    return typeof v === 'object' ? JSON.stringify(v) : String(v)
  const t = (type ?? '').toUpperCase()
  if (/^(?:INT|BIGINT|SMALLINT|TINYINT|SERIAL|BIGSERIAL)/.test(t)) {
    return new Intl.NumberFormat().format(num)
  }
  return num.toFixed(2)
}

function formatDateTime(v: unknown, type?: string): string {
  if (isNullValue(v))
    return ''
  const date = new Date(v as string | number)
  if (Number.isNaN(date.getTime()))
    return typeof v === 'object' ? JSON.stringify(v) : String(v)
  const t = (type ?? '').toUpperCase()
  if (t === 'DATE') {
    return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }).format(date)
  }
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'medium' }).format(date)
}

function jsonPreview(v: unknown): string {
  if (isNullValue(v))
    return ''
  const s = typeof v === 'string' ? v : JSON.stringify(v)
  return s.length > 60 ? `${s.slice(0, 60)}…` : s
}

function formatBlobSize(v: unknown): string {
  if (isNullValue(v))
    return ''
  const s = typeof v === 'string' ? v.length / 2 : (typeof v === 'object' ? JSON.stringify(v) : String(v)).length
  if (s < 1024)
    return `[BLOB ${s.toFixed(0)} B]`
  return `[BLOB ${(s / 1024).toFixed(1)} KB]`
}

function getCellClass(type?: string): string {
  if (!type)
    return 'text-xs'
  const t = type.toUpperCase()
  if (/^(?:INT|BIGINT|SMALLINT|TINYINT|SERIAL|BIGSERIAL|FLOAT|DOUBLE|REAL|DECIMAL|NUMERIC|MONEY)/.test(t)) {
    return 'text-xs tabular-nums justify-end'
  }
  if (/^(?:BOOL|BIT)/.test(t))
    return 'text-xs justify-center'
  return 'text-xs'
}

function getCellTooltip(v: unknown): string {
  if (isNullValue(v))
    return ''
  return typeof v === 'object' ? JSON.stringify(v) : String(v)
}

function formatCellText(v: unknown): string {
  if (isNullValue(v))
    return ''
  if (typeof v === 'object')
    return JSON.stringify(v)
  return String(v)
}

// ── Copy/Export ──
function copyAllAs(format: import('@/types/grid').CopyFormat) {
  copyUtil.copyRowsAs(props.rows, props.columns, format, props.tableName)
}

function exportAllAs(format: import('@/types/grid').CopyFormat) {
  copyUtil.exportToFile(props.rows, props.columns, format, props.tableName)
}

// ── Status Bar ──
const statusText = computed(() => {
  if (props.rows.length === 0)
    return ''
  const start = 1
  const end = props.rows.length
  return t('components.dataGrid.status.showing', { start, end, total: props.rowCount.toLocaleString() })
})

const formattedTime = computed(() => {
  if (props.executionTimeMs === undefined)
    return ''
  const t2 = props.executionTimeMs < 1000
    ? `${props.executionTimeMs}ms`
    : `${(props.executionTimeMs / 1000).toFixed(2)}s`
  return `${t('components.dataGrid.status.time', { time: t2 })}`
})

// ── Load more handling for virtual scroll ──
const allRowsLoaded = computed(() => props.rows.length >= props.rowCount)

// ── Clear selection on data change ──
watch(() => [props.rows, props.columns], () => {
  selection.clearSelection()
})
</script>

<template>
  <div class="data-grid bg-background flex flex-col h-full">
    <!-- Toolbar -->
    <div
      v-if="rows.length > 0 && !hideToolbar"
      class="px-3 py-1 border-b bg-muted/20 flex flex-shrink-0 gap-1 items-center"
    >
      <Button variant="ghost" size="sm" class="text-xs px-2 h-6" @click="copyAllAs('csv')">
        {{ $t('components.dataGrid.export.copyAllCsv') }}
      </Button>
      <Button variant="ghost" size="sm" class="text-xs px-2 h-6" @click="copyAllAs('json')">
        {{ $t('components.dataGrid.export.copyAllJson') }}
      </Button>
      <Button variant="ghost" size="sm" class="text-xs px-2 h-6" @click="copyAllAs('insert')">
        {{ $t('components.dataGrid.export.copyAllInsert') }}
      </Button>
      <span class="text-xs text-muted-foreground mx-1">|</span>
      <Button variant="ghost" size="sm" class="text-xs px-2 h-6" @click="exportAllAs('csv')">
        {{ $t('components.dataGrid.export.exportCsv') }}
      </Button>
      <Button variant="ghost" size="sm" class="text-xs px-2 h-6" @click="exportAllAs('json')">
        {{ $t('components.dataGrid.export.exportJson') }}
      </Button>
    </div>

    <!-- Filter Bar -->
    <FilterBar
      :filters="filter.filters.value"
      @remove-filter="handleRemoveFilter"
      @clear-all="handleClearAllFilters"
    />

    <!-- Loading State -->
    <div v-if="loading" class="px-2 py-1 flex-1 overflow-hidden">
      <div
        v-for="i in 10"
        :key="i"
        class="flex gap-3 h-8 items-center animate-pulse"
      >
        <div class="flex-shrink-0 w-8" />
        <div
          v-for="j in Math.min(columns.length, 6)"
          :key="j"
          class="rounded bg-muted/50 h-3"
          :style="{ width: `${40 + ((i * 7 + j * 13) % 60)}px` }"
        />
      </div>
    </div>

    <!-- Error State -->
    <div
      v-else-if="error"
      class="p-6 flex flex-1 items-center justify-center"
    >
      <div class="text-center space-y-3">
        <span class="i-carbon-warning-alt text-destructive mx-auto h-8 w-8 block" />
        <p class="text-sm text-destructive">
          {{ error }}
        </p>
        <Button variant="outline" size="sm" @click="emit('refresh')">
          {{ $t('components.dataGrid.error.retry') }}
        </Button>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-else-if="rows.length === 0 && !loading"
      class="flex flex-1 items-center justify-center"
    >
      <div class="text-muted-foreground text-center">
        <span class="i-carbon-data-blank mx-auto mb-2 opacity-40 h-8 w-8 block" />
        <p class="text-sm">
          {{ $t('components.dataGrid.empty') }}
        </p>
      </div>
    </div>

    <!-- Grid Content -->
    <div v-else class="flex flex-1 flex-col min-h-0">
      <!-- Scroll container -->
      <div
        ref="scrollContainer"
        class="flex-1 overflow-auto"
        @scroll="() => {}"
      >
        <div
          :style="{ height: `${rowVirtualizer.getTotalSize()}px` }"
          class="relative"
        >
          <!-- Sticky Header -->
          <div
            class="border-b bg-muted flex top-0 sticky z-20"
          >
            <!-- Select-All Checkbox -->
            <div class="flex flex-shrink-0 h-8 w-10 items-center justify-center">
              <Checkbox
                :checked="selection.isAllSelected(rows.length)"
                @update:checked="selection.toggleAll(rows.length)"
              />
            </div>
            <!-- Column Headers -->
            <div
              v-for="col in columns"
              :key="col"
              class="group flex flex-shrink-0 items-center relative"
              :style="{ width: `${getColumnWidth(col)}px` }"
              @contextmenu.prevent="openHeaderContextMenu($event, col)"
            >
              <button
                class="px-3 py-1.5 text-left flex flex-1 gap-1 min-w-0 items-center"
                :class="sort.getSortDirection(col) ? 'text-foreground' : 'text-muted-foreground hover:text-foreground'"
                @click="sort.toggleSort(col, ($event as MouseEvent).shiftKey); emit('sortChange', sort.sortState.value)"
              >
                <span class="text-xs font-medium truncate">{{ col }}</span>
                <span
                  v-if="sort.getSortDirection(col) === 'ASC'"
                  class="i-carbon-arrow-up flex-shrink-0 h-3 w-3"
                />
                <span
                  v-else-if="sort.getSortDirection(col) === 'DESC'"
                  class="i-carbon-arrow-down flex-shrink-0 h-3 w-3"
                />
                <span
                  v-else
                  class="i-carbon-chevron-sort opacity-0 flex-shrink-0 h-3 w-3 transition-opacity group-hover:opacity-40"
                />
                <!-- Multi-sort priority -->
                <span
                  v-if="sort.getSortPriority(col)"
                  class="text-[10px] text-primary leading-tight font-bold px-1 rounded bg-primary/10 flex-shrink-0"
                >{{ sort.getSortPriority(col) }}</span>
                <!-- Filter indicator -->
                <span
                  v-if="filter.hasFilter(col)"
                  class="i-carbon-filter text-blue-500 flex-shrink-0 h-3 w-3"
                />
              </button>
              <!-- Resize Handle -->
              <div
                class="opacity-0 w-1 cursor-col-resize transition-opacity bottom-0 right-0 top-0 absolute z-10 hover:bg-primary/40 group-hover:opacity-100"
                @mousedown.stop="startColumnResize($event, col)"
              />
            </div>
            <!-- Actions Column Header -->
            <div
              v-if="connectionId && tableName"
              class="bg-muted flex-shrink-0 w-10 right-0 sticky z-10"
            >
              <div class="flex h-8 items-center justify-center">
                <span class="i-carbon-overflow-menu-vertical text-muted-foreground h-3.5 w-3.5" />
              </div>
            </div>
          </div>

          <!-- Virtual Rows -->
          <div
            v-for="virtualRow in rowVirtualizer.getVirtualItems()"
            :key="`r-${virtualRow.index}`"
            :data-index="virtualRow.index"
            class="border-b border-border/30 flex transition-colors duration-75 left-0 top-0 absolute hover:bg-muted/[0.08]"
            :class="[
              virtualRow.index % 2 === 0 ? 'bg-muted/[0.03]' : '',
              selection.isSelected(virtualRow.index) ? 'bg-primary/[0.08]' : '',
            ]"
            :style="{
              height: `${virtualRow.size}px`,
              transform: `translateY(${virtualRow.start}px)`,
            }"
          >
            <!-- Row Checkbox -->
            <div
              class="flex flex-shrink-0 w-10 items-center justify-center"
              @click.stop
            >
              <Checkbox
                :checked="selection.isSelected(virtualRow.index)"
                @update:checked="selection.toggleRow(virtualRow.index, false)"
              />
            </div>

            <!-- Data Cells -->
            <div
              v-for="col in columns"
              :key="`c-${virtualRow.index}-${col}`"
              class="px-3 py-1 flex flex-shrink-0 items-center overflow-hidden"
              :class="getCellClass(columnTypes?.[col])"
              :style="{ width: `${getColumnWidth(col)}px` }"
              :title="getCellTooltip(rows[virtualRow.index][col])"
              @click="selection.toggleRow(virtualRow.index, false)"
              @contextmenu.prevent="openCellContextMenu($event, virtualRow.index, col)"
            >
              <!-- NULL -->
              <span
                v-if="isNullValue(rows[virtualRow.index][col])"
                class="text-xs text-muted-foreground italic"
              >{{ $t('components.dataGrid.null') }}</span>

              <!-- Boolean -->
              <span
                v-else-if="isBooleanType(columnTypes?.[col])"
                :class="rows[virtualRow.index][col] ? 'text-green-600' : 'text-red-400'"
                class="inline-flex"
              >
                <span
                  v-if="rows[virtualRow.index][col]"
                  class="i-carbon-checkmark h-4 w-4"
                />
                <span v-else class="i-carbon-close h-4 w-4" />
              </span>

              <!-- Number -->
              <span
                v-else-if="isNumericType(columnTypes?.[col])"
                class="text-xs truncate tabular-nums"
              >{{ formatNumber(rows[virtualRow.index][col], columnTypes?.[col]) }}</span>

              <!-- Date/Time -->
              <span
                v-else-if="isDateType(columnTypes?.[col])"
                class="text-xs truncate"
              >{{ formatDateTime(rows[virtualRow.index][col], columnTypes?.[col]) }}</span>

              <!-- JSON -->
              <span
                v-else-if="isJsonType(columnTypes?.[col])"
                class="text-xs flex gap-1 min-w-0 items-center"
              >
                <span class="truncate">{{ jsonPreview(rows[virtualRow.index][col]) }}</span>
                <Button
                  variant="ghost"
                  size="sm"
                  class="text-[10px] text-primary px-1 flex-shrink-0 h-5"
                  @click.stop="openJsonDialog(rows[virtualRow.index][col], col)"
                >{{ $t('components.dataGrid.json.expand') }}</Button>
              </span>

              <!-- BLOB -->
              <span
                v-else-if="isBlobType(columnTypes?.[col])"
                class="text-xs text-muted-foreground"
              >{{ formatBlobSize(rows[virtualRow.index][col]) }}</span>

              <!-- Text (default) -->
              <span v-else class="text-xs truncate">{{ formatCellText(rows[virtualRow.index][col]) }}</span>
            </div>

            <!-- Row Actions -->
            <div
              v-if="connectionId && tableName"
              class="bg-background/80 opacity-0 flex flex-shrink-0 w-10 transition-opacity items-center right-0 justify-center sticky z-10 group-hover:opacity-100"
              @click.stop
            >
              <DropdownMenu>
                <DropdownMenuTrigger class="rounded flex h-6 w-6 items-center justify-center hover:bg-muted">
                  <span class="i-carbon-overflow-menu-vertical h-4 w-4" />
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" :side-offset="4">
                  <DropdownMenuItem @click="openEditDialog(virtualRow.index)">
                    <span class="i-carbon-edit mr-2 h-3.5 w-3.5" />{{ $t('components.dataGrid.row.edit') }}
                  </DropdownMenuItem>
                  <DropdownMenuItem @click="openDuplicateDialog(virtualRow.index)">
                    <span class="i-carbon-copy mr-2 h-3.5 w-3.5" />{{ $t('components.dataGrid.row.duplicate') }}
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem @click="openDetailsDialog(virtualRow.index)">
                    <span class="i-carbon-information mr-2 h-3.5 w-3.5" />{{ $t('components.dataGrid.row.details') }}
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem @click="copyUtil.copyRowsAs([rows[virtualRow.index]], columns, 'csv', tableName)">
                    <span class="i-carbon-table-split mr-2 h-3.5 w-3.5" />{{ $t('components.dataGrid.row.copyAsCsv') }}
                  </DropdownMenuItem>
                  <DropdownMenuItem @click="copyUtil.copyRowsAs([rows[virtualRow.index]], columns, 'json', tableName)">
                    <span class="i-carbon-code mr-2 h-3.5 w-3.5" />{{ $t('components.dataGrid.row.copyAsJson') }}
                  </DropdownMenuItem>
                  <DropdownMenuItem @click="copyUtil.copyRowsAs([rows[virtualRow.index]], columns, 'insert', tableName)">
                    <span class="i-carbon-sql mr-2 h-3.5 w-3.5" />{{ $t('components.dataGrid.row.copyAsInsert') }}
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    class="text-destructive focus:text-destructive"
                    @click="openDeleteDialog(virtualRow.index)"
                  >
                    <span class="i-carbon-trash-can mr-2 h-3.5 w-3.5" />{{ $t('components.dataGrid.row.delete') }}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>

          <!-- "Load more..." indicator if not all rows loaded -->
          <div
            v-if="!allRowsLoaded && rows.length > 0"
            class="text-xs text-muted-foreground py-2 flex items-center justify-center"
          >
            {{ $t('components.dataGrid.status.showingLimited', { start: 1, end: rows.length, loaded: rows.length }) }}
          </div>
        </div>
      </div>

      <!-- Batch Action Bar -->
      <BatchActionBar
        :selected-count="selection.selectedCount.value"
        :selected-rows="selection.getSelectedRows(rows)"
        :columns="columns"
        :table-name="tableName"
        @export-selected="(fmt) => copyUtil.copyRowsAs(selection.getSelectedRows(rows), columns, fmt, tableName)"
      />

      <!-- Status Bar -->
      <div class="text-xs text-muted-foreground px-3 py-1.5 border-t bg-muted/20 flex flex-shrink-0 gap-3 items-center">
        <span class="tabular-nums">{{ statusText }}</span>
        <span v-if="formattedTime" class="text-muted-foreground/70">{{ formattedTime }}</span>
        <div class="flex-1" />
        <span v-if="sort.hasActiveSort.value" class="text-primary tabular-nums">
          {{ $t('components.dataGrid.sort.asc') }}/{{ $t('components.dataGrid.sort.desc') }}
        </span>
        <span v-if="filter.hasActiveFilters.value" class="text-blue-500 tabular-nums">
          {{ filter.filters.value.length }} {{ $t('components.dataGrid.filter.activeFilters') }}
        </span>
      </div>
    </div>

    <!-- Context Menus -->
    <CellContextMenu
      v-bind="cellMenu"
      :columns="columns"
      :table-name="tableName"
      @close="cellMenu.show = false"
      @filter="handleAddFilter"
    />
    <ColumnHeaderContextMenu
      v-bind="headerMenu"
      @close="headerMenu.show = false"
      @sort="handleSortFromHeader"
      @clear-sort="handleClearSort"
      @filter="handleAddFilter"
      @clear-filter="handleClearFilter"
    />

    <!-- Dialogs -->
    <JsonViewDialog
      :open="jsonDialogOpen"
      :value="jsonDialogValue"
      :column="jsonDialogColumn"
      @update:open="jsonDialogOpen = $event"
    />

    <EditRowDialog
      v-if="connectionId && tableName"
      :open="editDialogOpen"
      :row="currentEditingRow"
      :columns="columns"
      :column-types="columnTypes ?? {}"
      :primary-keys="primaryKeys"
      :is-new-row="isDuplicateRow"
      :connection-id="connectionId"
      :database="database"
      :schema="schema"
      :table-name="tableName"
      @update:open="editDialogOpen = $event"
      @saved="onEditSaved"
    />

    <RowDetailsDialog
      :open="detailsDialogOpen"
      :row="currentDetailsRow"
      :columns="columns"
      :column-types="columnTypes"
      @update:open="detailsDialogOpen = $event"
    />

    <!-- Delete Confirmation -->
    <AlertDialog :open="deleteDialogOpen" @update:open="deleteDialogOpen = $event">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{{ $t('components.dataGrid.delete.title') }}</AlertDialogTitle>
          <AlertDialogDescription>{{ $t('components.dataGrid.delete.message') }}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{{ $t('components.dataGrid.edit.cancel') }}</AlertDialogCancel>
          <AlertDialogAction
            class="text-destructive-foreground bg-destructive hover:bg-destructive/90"
            :disabled="isDeleting"
            @click="confirmDelete"
          >
            {{ $t('components.dataGrid.delete.confirm') }}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
