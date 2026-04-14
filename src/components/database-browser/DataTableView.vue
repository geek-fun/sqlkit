<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { save as showSaveDialog } from '@tauri-apps/plugin-dialog'
import { computed, onMounted, ref, watch } from 'vue'
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
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Spinner } from '@/components/ui/spinner'
import { useMinLoadingTime } from '@/composables/useMinLoadingTime'
import { toast } from '@/composables/useNotifications'
import { ConnectionStatus, useConnectionStore } from '@/store'
import {
  computeOffset,
  computeTotalPages,
  formatTableValue,
  isTableNullValue,
  rowsToCsv,
} from './dataTableHelpers'

type TableDataResult = {
  columns: string[]
  rows: Record<string, unknown>[]
  rows_affected?: number
  execution_time_ms?: number
}

type ColumnTypeInfo = {
  name: string
  data_type: string
  is_primary_key: boolean
  nullable: boolean
}

const props = defineProps<{
  connectionId: string
  database: string
  schema?: string
  tableName: string
}>()

const { t } = useI18n()

const ROWS_PER_PAGE_OPTIONS = [100, 500, 1000] as const
type RowsPerPage = (typeof ROWS_PER_PAGE_OPTIONS)[number]

const currentPage = ref(1)
const rowsPerPage = ref<RowsPerPage>(100)
const filterInput = ref('')
const appliedFilter = ref('')
const hiddenColumns = ref<Set<string>>(new Set())
const showColumnMenu = ref(false)

const data = ref<TableDataResult | null>(null)
const totalCount = ref(0)
const loading = ref(false)
const error = ref<string | null>(null)
const executionTimeMs = ref<number | null>(null)
const columnInfoList = ref<ColumnTypeInfo[]>([])

// --- Connection state ---
const connectionStore = useConnectionStore()
const isReconnecting = ref(false)
const connectionError = ref<string | null>(null)

// --- Delete state ---
const deleteDialogOpen = ref(false)
const deletingRow = ref<Record<string, unknown> | null>(null)
const isDeleting = ref(false)

// --- Edit state ---
const editDialogOpen = ref(false)
const editingRow = ref<Record<string, unknown> | null>(null)
// editForm: col -> { value: string, setNull: boolean }
const editForm = ref<Record<string, { value: string, setNull: boolean }>>({})
const editErrors = ref<Record<string, string>>({})
const isSaving = ref(false)

// --- Export state ---
const isExporting = ref(false)

const { withMinLoadingTime } = useMinLoadingTime()

const visibleColumns = computed(() =>
  data.value ? data.value.columns.filter((c: string) => !hiddenColumns.value.has(c)) : [],
)

const columnTypeMap = computed(() =>
  Object.fromEntries(columnInfoList.value.map(c => [c.name, c.data_type])),
)

const columnIsPK = computed(() =>
  Object.fromEntries(columnInfoList.value.map(c => [c.name, c.is_primary_key])),
)

const columnNullable = computed(() =>
  Object.fromEntries(columnInfoList.value.map(c => [c.name, c.nullable])),
)

/** PK columns (in order they appear in the result set) */
const pkColumns = computed(() =>
  (data.value?.columns ?? []).filter((c: string) => columnIsPK.value[c]),
)

function extractPkValues(row: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(pkColumns.value.map((col: string) => [col, row[col] ?? null]))
}

function formatPkSummary(row: Record<string, unknown>): string {
  return pkColumns.value.length > 0
    ? pkColumns.value.map((col: string) => `${col}: ${formatTableValue(row[col])}`).join(', ')
    : Object.entries(row).slice(0, 2).map(([k, v]) => `${k}: ${formatTableValue(v)}`).join(', ')
}

const totalPages = computed(() => computeTotalPages(totalCount.value, rowsPerPage.value))

const offset = computed(() => computeOffset(currentPage.value, rowsPerPage.value))

const pageNumbers = computed<(number | '...')[]>(() => {
  const total = totalPages.value
  const cur = currentPage.value
  if (total <= 7)
    return Array.from({ length: total }, (_, i) => i + 1)
  const middlePages = Array.from(
    { length: Math.min(total - 1, cur + 1) - Math.max(2, cur - 1) + 1 },
    (_, i) => Math.max(2, cur - 1) + i,
  )
  return [
    1,
    ...(cur > 3 ? ['...' as const] : []),
    ...middlePages,
    ...(cur < total - 2 ? ['...' as const] : []),
    total,
  ]
})

const formattedTime = computed(() => {
  if (executionTimeMs.value === null)
    return ''
  return executionTimeMs.value < 1000
    ? `${executionTimeMs.value}ms`
    : `${(executionTimeMs.value / 1000).toFixed(2)}s`
})

async function fetchData() {
  loading.value = true
  error.value = null

  try {
    await withMinLoadingTime(async () => {
      const result = await invoke<TableDataResult>('get_table_data', {
        connectionId: props.connectionId,
        query: {
          database: props.database || null,
          table: props.tableName,
          schema: props.schema ?? null,
          filter: appliedFilter.value.trim() || null,
          limit: rowsPerPage.value,
          offset: offset.value,
        },
      })
      data.value = result
      executionTimeMs.value = result.execution_time_ms ?? null
    })
  }
  catch (err) {
    error.value = String(err)
    data.value = null
  }
  finally {
    loading.value = false
  }
}

async function fetchCount() {
  try {
    const count = await invoke<number>('get_table_count', {
      connectionId: props.connectionId,
      database: props.database || null,
      table: props.tableName,
      schema: props.schema ?? null,
      filter: appliedFilter.value.trim() || null,
    })
    totalCount.value = count
  }
  catch {
    totalCount.value = 0
  }
}

/** Check if connection is active and reconnect if needed */
async function ensureConnection(): Promise<boolean> {
  const status = connectionStore.getConnectionStatus(props.connectionId)

  if (status === ConnectionStatus.CONNECTED) {
    return true
  }

  // Try to reconnect
  isReconnecting.value = true
  connectionError.value = null

  try {
    await withMinLoadingTime(async () => {
      await connectionStore.connect(props.connectionId)
    })
    isReconnecting.value = false
    return true
  }
  catch (err) {
    isReconnecting.value = false
    connectionError.value = err instanceof Error ? err.message : String(err)
    return false
  }
}

async function refresh() {
  await Promise.all([fetchData(), fetchCount()])
}

async function handleRetry() {
  connectionError.value = null
  const connected = await ensureConnection()
  if (connected) {
    fetchColumnInfo()
    refresh()
  }
}

async function fetchColumnInfo() {
  if (!props.database || !props.tableName) {
    columnInfoList.value = []
    return
  }
  try {
    columnInfoList.value = await invoke<ColumnTypeInfo[]>('list_columns', {
      connectionId: props.connectionId,
      database: props.database,
      schema: props.schema ?? null,
      tableName: props.tableName,
    })
  }
  catch {
    columnInfoList.value = []
  }
}

function applyFilter() {
  appliedFilter.value = filterInput.value.trim()
  currentPage.value = 1
  refresh()
}

function clearFilter() {
  filterInput.value = ''
  if (appliedFilter.value !== '') {
    appliedFilter.value = ''
    currentPage.value = 1
    refresh()
  }
}

function goToPage(page: number) {
  if (page < 1 || page > totalPages.value)
    return
  currentPage.value = page
  fetchData()
}

function changeRowsPerPage(val: string) {
  rowsPerPage.value = Number(val) as RowsPerPage
  currentPage.value = 1
  fetchData()
}

function toggleColumn(col: string) {
  hiddenColumns.value = hiddenColumns.value.has(col)
    ? new Set([...hiddenColumns.value].filter(c => c !== col))
    : new Set([...hiddenColumns.value, col])
}

async function exportCSV() {
  if (!data.value)
    return

  const csv = rowsToCsv(data.value.rows, visibleColumns.value)
  const defaultName = `${props.tableName}.csv`

  try {
    const selectedPath = await showSaveDialog({
      filters: [{ name: 'CSV Files', extensions: ['csv'] }],
      defaultPath: defaultName,
    })
    if (!selectedPath)
      return

    isExporting.value = true
    const filePath = selectedPath.endsWith('.csv') ? selectedPath : `${selectedPath}.csv`

    try {
      await withMinLoadingTime(async () => {
        await invoke('write_text_file', { path: filePath, content: csv })
      })
      toast.success(t('components.dataTableView.notifications.csvExported'), { description: filePath })
    }
    finally {
      isExporting.value = false
    }
  }
  catch (err) {
    const msg = err instanceof Error ? err.message : String(err)
    toast.error(t('components.dataTableView.notifications.exportFailed'), { description: msg })
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = defaultName
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }
}

function openDeleteDialog(row: Record<string, unknown>) {
  deletingRow.value = row
  deleteDialogOpen.value = true
}

async function confirmDelete() {
  const row = deletingRow.value
  if (!row)
    return
  isDeleting.value = true
  try {
    await withMinLoadingTime(async () => {
      await invoke('delete_table_row', {
        connectionId: props.connectionId,
        database: props.database ?? null,
        table: props.tableName,
        schema: props.schema ?? null,
        pkValues: extractPkValues(row),
      })
    })
    toast.success(t('components.dataTableView.notifications.rowDeleted'))
    deleteDialogOpen.value = false
    deletingRow.value = null
    await refresh()
  }
  catch (err) {
    toast.error(t('components.dataTableView.notifications.deleteFailed'), {
      description: err instanceof Error ? err.message : String(err),
    })
  }
  finally {
    isDeleting.value = false
  }
}

function rawValueToString(v: unknown): string {
  if (v === null || v === undefined)
    return ''
  if (typeof v === 'object')
    return JSON.stringify(v)
  return String(v)
}

function openEditDialog(row: Record<string, unknown>) {
  editingRow.value = row
  editErrors.value = {}
  const cols = data.value?.columns ?? []
  editForm.value = Object.fromEntries(
    cols.map((col: string) => [
      col,
      {
        value: rawValueToString(row[col]),
        setNull: row[col] === null || row[col] === undefined,
      },
    ]),
  )
  editDialogOpen.value = true
}

function validateEditForm(): boolean {
  const columns = data.value?.columns ?? []
  const errors: Record<string, string> = {}

  for (const col of columns) {
    const field = editForm.value[col]
    if (!field)
      continue

    // Skip validation if set to NULL
    if (field.setNull) {
      if (!columnNullable.value[col]) {
        errors[col] = t('components.dataTableView.validation.required')
      }
      continue
    }

    const value = field.value.trim()
    const type = (columnTypeMap.value[col] ?? '').toLowerCase()

    // Required field check
    if (value === '' && !columnNullable.value[col]) {
      errors[col] = t('components.dataTableView.validation.required')
      continue
    }

    // Skip empty values for nullable fields
    if (value === '')
      continue

    // Type validation
    if (isNumericType(type)) {
      if (!isValidNumber(value)) {
        errors[col] = t('components.dataTableView.validation.invalidNumber')
        continue
      }
    }

    if (type === 'bool' || type === 'boolean') {
      if (!isValidBoolean(value)) {
        errors[col] = t('components.dataTableView.validation.invalidBoolean')
        continue
      }
    }
  }

  editErrors.value = errors
  return Object.keys(errors).length === 0
}

function isNumericType(type: string): boolean {
  return type.includes('int')
    || type.includes('serial')
    || type.includes('numeric')
    || type.includes('decimal')
    || type.includes('float')
    || type.includes('double')
    || type.includes('real')
    || type.includes('money')
    || type.includes('number')
}

function isValidNumber(value: string): boolean {
  // Allow negative numbers, decimals, scientific notation
  return /^-?(?:\d+(?:\.\d+)?|\.\d+)(?:e[+-]?\d+)?$/i.test(value) && !Number.isNaN(Number(value))
}

function isValidBoolean(value: string): boolean {
  const lower = value.toLowerCase()
  return ['true', 'false', '1', '0', 't', 'f', 'y', 'n', 'yes', 'no'].includes(lower)
}

function getEditInputType(col: string): string {
  const type = (columnTypeMap.value[col] ?? '').toLowerCase()
  if (isNumericType(type))
    return 'number'
  return 'text'
}

function coerceEditValue(col: string, value: string): unknown {
  const type = (columnTypeMap.value[col] ?? '').toLowerCase()
  if (type.includes('int') || type.includes('serial') || type.includes('numeric') || type.includes('decimal') || type.includes('float') || type.includes('double') || type.includes('real') || type.includes('money')) {
    const n = Number(value)
    if (!Number.isNaN(n))
      return n
  }
  if (type === 'bool' || type === 'boolean') {
    if (value.toLowerCase() === 'true')
      return true
    if (value.toLowerCase() === 'false')
      return false
  }
  return value
}

async function confirmEdit() {
  const row = editingRow.value
  if (!row)
    return
  if (!validateEditForm())
    return

  isSaving.value = true
  try {
    const columns = data.value?.columns ?? []
    const updates = Object.fromEntries(
      columns
        .map((col: string) => {
          const field = editForm.value[col]
          return field ? [col, field.setNull ? null : coerceEditValue(col, field.value)] : null
        })
        .filter((entry): entry is [string, unknown] => entry !== null),
    )

    await withMinLoadingTime(async () => {
      await invoke('update_table_row', {
        connectionId: props.connectionId,
        database: props.database ?? null,
        table: props.tableName,
        schema: props.schema ?? null,
        pkValues: extractPkValues(row),
        updates,
      })
    })
    toast.success(t('components.dataTableView.notifications.rowUpdated'))
    editDialogOpen.value = false
    editingRow.value = null
    await refresh()
  }
  catch (err) {
    toast.error(t('components.dataTableView.notifications.updateFailed'), {
      description: err instanceof Error ? err.message : String(err),
    })
  }
  finally {
    isSaving.value = false
  }
}

const formatValue = formatTableValue
const isNullValue = isTableNullValue

onMounted(async () => {
  const connected = await ensureConnection()
  if (connected) {
    fetchColumnInfo()
    refresh()
  }
})

watch(
  () => [props.connectionId, props.tableName, props.schema] as const,
  async () => {
    currentPage.value = 1
    appliedFilter.value = ''
    filterInput.value = ''
    hiddenColumns.value = new Set()
    connectionError.value = null

    const connected = await ensureConnection()
    if (connected) {
      fetchColumnInfo()
      refresh()
    }
  },
)
</script>

<template>
  <div class="data-table-view flex flex-col h-full" @click="showColumnMenu = false">
    <!-- Filter / Toolbar bar -->
    <div class="px-3 py-1.5 border-b bg-muted/20 flex gap-1.5 items-center">
      <!-- Filter icon -->
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="text-muted-foreground flex-shrink-0"
      >
        <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" />
      </svg>

      <!-- Filter input -->
      <Input
        v-model="filterInput"
        :placeholder="t('components.dataTableView.filterPlaceholder')"
        class="text-xs font-mono flex-1 h-7"
        @keydown.enter="applyFilter"
      />

      <!-- Clear filter button -->
      <Button
        v-if="filterInput || appliedFilter"
        variant="ghost"
        size="icon"
        class="flex-shrink-0 h-7 w-7"
        :title="t('components.dataTableView.clearFilter')"
        @click.stop="clearFilter"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 6 6 18" />
          <path d="m6 6 12 12" />
        </svg>
      </Button>

      <!-- Refresh button -->
      <Button
        variant="ghost"
        size="icon"
        class="flex-shrink-0 h-7 w-7"
        :disabled="loading"
        :title="t('components.dataTableView.refresh')"
        @click.stop="refresh"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" :class="{ 'animate-spin': loading }">
          <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
          <path d="M21 3v5h-5" />
          <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" />
          <path d="M8 16H3v5" />
        </svg>
      </Button>

      <!-- Column visibility dropdown -->
      <div class="flex-shrink-0 relative">
        <Button
          variant="ghost"
          size="icon"
          class="h-7 w-7"
          :title="t('components.dataTableView.columns')"
          @click.stop="showColumnMenu = !showColumnMenu"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect width="18" height="18" x="3" y="3" rx="2" />
            <path d="M9 3v18" />
            <path d="M15 3v18" />
          </svg>
        </Button>

        <div
          v-if="showColumnMenu && data && data.columns.length > 0"
          class="text-popover-foreground mt-1 border rounded-md bg-popover max-h-64 min-w-36 shadow-md right-0 top-full absolute z-50 overflow-auto"
          @click.stop
        >
          <div class="text-xs text-muted-foreground font-semibold p-1 px-2 py-1.5 border-b">
            {{ t('components.dataTableView.columns') }}
          </div>
          <div class="p-1">
            <button
              v-for="col in data.columns"
              :key="col"
              class="text-sm px-2 py-1 text-left rounded flex gap-2 w-full cursor-pointer items-center hover:bg-accent"
              @click="toggleColumn(col)"
            >
              <svg
                v-if="!hiddenColumns.has(col)"
                xmlns="http://www.w3.org/2000/svg"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="text-primary flex-shrink-0"
              >
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <span v-else class="flex-shrink-0 w-3 inline-block" />
              <span class="truncate">{{ col }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- Export CSV button (current page) -->
      <Button
        variant="ghost"
        size="icon"
        class="flex-shrink-0 h-7 w-7"
        :disabled="!data || data.rows.length === 0 || isExporting"
        :title="t('components.dataTableView.exportCsv')"
        @click.stop="exportCSV"
      >
        <Spinner v-if="isExporting" size="sm" />
        <svg v-else xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 10 12 15 17 10" />
          <line x1="12" x2="12" y1="15" y2="3" />
        </svg>
      </Button>

      <!-- Apply Filters button -->
      <Button
        size="sm"
        class="text-xs flex-shrink-0 h-7"
        @click.stop="applyFilter"
      >
        {{ t('components.dataTableView.applyFilters') }}
      </Button>
    </div>

    <!-- Table area -->
    <div class="flex-1 relative overflow-auto">
      <!-- Connection error state -->
      <div v-if="connectionError" class="p-4 flex h-full items-center justify-center">
        <div class="text-center max-w-md">
          <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="text-muted-foreground mx-auto mb-4">
            <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" />
            <polyline points="10 17 15 12 10 7" />
            <line x1="15" x2="3" y1="12" y2="12" />
          </svg>
          <p class="text-sm text-muted-foreground mb-2">
            {{ t('components.dataTableView.connectionLost') }}
          </p>
          <p class="text-xs text-muted-foreground/70 mb-4">
            {{ connectionError }}
          </p>
          <Button size="sm" @click="handleRetry">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-1.5">
              <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
              <path d="M21 3v5h-5" />
              <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" />
              <path d="M8 16H3v5" />
            </svg>
            {{ t('components.dataTableView.reconnect') }}
          </Button>
        </div>
      </div>

      <!-- Reconnecting state -->
      <div v-else-if="isReconnecting" class="p-4 flex h-full items-center justify-center">
        <div class="text-center">
          <svg class="text-primary mx-auto mb-2 h-8 w-8 animate-spin" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
          </svg>
          <p class="text-sm text-muted-foreground">
            {{ t('components.dataTableView.reconnecting') }}
          </p>
        </div>
      </div>

      <!-- Loading overlay -->
      <div
        v-if="loading"
        class="bg-background/60 flex items-center inset-0 justify-center absolute z-10"
      >
        <div class="text-center">
          <svg
            class="text-primary mx-auto mb-2 h-8 w-8 animate-spin"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
          </svg>
          <p class="text-sm text-muted-foreground">
            {{ t('components.dataTableView.loading') }}
          </p>
        </div>
      </div>

      <!-- Error state -->
      <div v-else-if="error" class="p-4">
        <div class="p-4 border border-red-200 rounded-md bg-red-50 dark:border-red-800 dark:bg-red-900/20">
          <div class="flex gap-3 items-start">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-red-600 mt-0.5 flex-shrink-0 dark:text-red-400">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" x2="12" y1="8" y2="12" />
              <line x1="12" x2="12.01" y1="16" y2="16" />
            </svg>
            <div>
              <p class="text-sm text-red-800 font-medium dark:text-red-200">
                {{ t('components.dataTableView.error') }}
              </p>
              <p class="text-sm text-red-700 mt-1 dark:text-red-300">
                {{ error }}
              </p>
            </div>
          </div>
        </div>
      </div>

      <!-- Data table -->
      <table
        v-else-if="data && data.columns.length > 0"
        class="data-table w-full"
      >
        <thead>
          <tr class="border-b">
            <th class="data-table-header text-center w-10">
              #
            </th>
            <th
              v-for="col in visibleColumns"
              :key="col"
              class="data-table-header text-left"
            >
              <div class="col-header-cell" :title="columnTypeMap[col] ? `${col} · ${columnTypeMap[col]}` : col">
                <svg
                  v-if="columnIsPK[col]"
                  xmlns="http://www.w3.org/2000/svg"
                  width="10"
                  height="10"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="text-amber-500 flex-shrink-0"
                >
                  <circle cx="7.5" cy="15.5" r="5.5" />
                  <path d="m21 2-9.6 9.6" />
                  <path d="m15.5 7.5 3 3L22 7l-3-3" />
                </svg>
                <span class="col-name truncate">{{ col }}</span>
                <span v-if="columnTypeMap[col]" class="col-type flex-shrink-0">{{ columnTypeMap[col] }}</span>
              </div>
            </th>
            <!-- Actions header — sticky right, excluded from CSV / column visibility -->
            <th class="data-table-header data-table-actions-col text-center">
              {{ t('components.dataTableView.actions') }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, i) in data.rows"
            :key="i"
            class="border-b hover:bg-muted/50"
          >
            <td class="text-xs text-muted-foreground px-3 py-1.5 text-center w-10">
              {{ offset + i + 1 }}
            </td>
            <td
              v-for="col in visibleColumns"
              :key="`${i}-${col}`"
              class="text-sm px-3 py-1.5 max-w-xs truncate"
              :class="{ 'italic text-muted-foreground': isNullValue(row[col]) }"
              :title="formatValue(row[col])"
            >
              {{ formatValue(row[col]) }}
            </td>
            <!-- Actions cell — sticky right -->
            <td class="data-table-actions-col px-2 py-1">
              <div class="flex gap-0.5 items-center justify-center">
                <!-- Edit button -->
                <Button
                  variant="ghost"
                  size="icon"
                  class="text-foreground h-6 w-6"
                  :title="t('components.dataTableView.editRow')"
                  @click.stop="openEditDialog(row)"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
                    <path d="m15 5 4 4" />
                  </svg>
                </Button>
                <!-- Delete button -->
                <Button
                  variant="ghost"
                  size="icon"
                  class="text-foreground h-6 w-6 hover:text-destructive"
                  :title="t('components.dataTableView.deleteRow')"
                  @click.stop="openDeleteDialog(row)"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M3 6h18" />
                    <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
                    <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
                  </svg>
                </Button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>

      <!-- Empty state -->
      <div
        v-else-if="data && data.rows.length === 0"
        class="flex h-full items-center justify-center"
      >
        <div class="text-muted-foreground text-center">
          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mx-auto mb-2 opacity-50">
            <path d="M12 3v18" />
            <rect width="18" height="18" x="3" y="3" rx="2" />
            <path d="M3 9h18" />
            <path d="M3 15h18" />
          </svg>
          <p class="text-sm">
            {{ t('components.dataTableView.noRows') }}
          </p>
        </div>
      </div>

      <!-- No data yet -->
      <div v-else class="flex h-full items-center justify-center">
        <div class="text-muted-foreground text-center">
          <svg
            class="text-primary mx-auto mb-2 h-8 w-8 animate-spin"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
          </svg>
        </div>
      </div>
    </div>

    <!-- Status bar + pagination -->
    <div class="text-xs text-muted-foreground px-3 py-1.5 border-t bg-muted/20 flex flex-wrap gap-3 items-center">
      <!-- Left: stats -->
      <div class="flex flex-shrink-0 gap-3 items-center">
        <span v-if="totalCount > 0" class="tabular-nums">
          {{ totalCount.toLocaleString() }} rows
        </span>
        <span v-if="formattedTime" class="text-muted-foreground/70">{{ formattedTime }}</span>
      </div>

      <div class="flex-1" />

      <!-- Right: pagination + rows per page grouped -->
      <div class="flex flex-shrink-0 gap-2 items-center">
        <!-- Page navigation -->
        <div v-if="data" class="flex gap-0.5 items-center">
          <!-- Prev -->
          <button
            class="text-xs rounded inline-flex h-6 w-6 transition-colors items-center justify-center hover:bg-accent disabled:opacity-40 disabled:cursor-not-allowed"
            :disabled="currentPage === 1 || loading"
            @click="goToPage(currentPage - 1)"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="m15 18-6-6 6-6" />
            </svg>
          </button>

          <template v-for="(p, idx) in pageNumbers" :key="idx">
            <span
              v-if="p === '...'"
              class="text-xs text-muted-foreground inline-flex h-6 w-6 items-center justify-center"
            >…</span>
            <button
              v-else
              class="text-xs font-medium px-1.5 rounded inline-flex h-6 min-w-6 transition-colors items-center justify-center"
              :class="p === currentPage
                ? 'bg-primary text-primary-foreground'
                : 'hover:bg-accent'"
              :disabled="loading"
              @click="goToPage(p as number)"
            >
              {{ p }}
            </button>
          </template>

          <!-- Next -->
          <button
            class="text-xs rounded inline-flex h-6 w-6 transition-colors items-center justify-center hover:bg-accent disabled:opacity-40 disabled:cursor-not-allowed"
            :disabled="currentPage >= totalPages || loading"
            @click="goToPage(currentPage + 1)"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="m9 18 6-6-6-6" />
            </svg>
          </button>
        </div>

        <!-- Divider -->
        <div v-if="data" class="bg-border h-3.5 w-px" />

        <!-- Rows per page -->
        <div class="flex gap-1.5 items-center">
          <span class="text-muted-foreground/70">Rows</span>
          <Select
            :model-value="String(rowsPerPage)"
            @update:model-value="changeRowsPerPage"
          >
            <SelectTrigger class="text-xs h-6 w-16">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="n in ROWS_PER_PAGE_OPTIONS"
                :key="n"
                :value="String(n)"
              >
                {{ n }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>
    </div>

    <!-- ── Delete confirmation dialog ── -->
    <AlertDialog v-model:open="deleteDialogOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{{ t('components.dataTableView.deleteDialog.title') }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{ t('components.dataTableView.deleteDialog.message') }}
          </AlertDialogDescription>
        </AlertDialogHeader>

        <!-- PK summary so user knows exactly which row will be deleted -->
        <div v-if="deletingRow" class="text-xs text-muted-foreground font-mono px-3 py-2 rounded-md bg-muted break-all">
          {{ formatPkSummary(deletingRow) }}
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel :disabled="isDeleting">
            {{ t('common.buttons.cancel') }}
          </AlertDialogCancel>
          <AlertDialogAction
            :disabled="isDeleting"
            @click.prevent="confirmDelete"
          >
            <Spinner v-if="isDeleting" size="sm" class="mr-1.5" />
            {{ t('components.dataTableView.deleteDialog.confirm') }}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <!-- ── Edit row dialog ── -->
    <Dialog v-model:open="editDialogOpen">
      <DialogContent class="p-0 flex flex-col gap-0 max-h-[80vh] max-w-lg">
        <div class="px-6 pb-3 pt-5 border-b">
          <DialogTitle>{{ t('components.dataTableView.editDialog.title') }}</DialogTitle>
        </div>

        <!-- Scrollable form area -->
        <div class="px-6 py-4 flex-1 overflow-y-auto space-y-3">
          <div
            v-for="col in (data?.columns ?? [])"
            :key="col"
            class="space-y-1"
          >
            <div class="flex gap-2 items-center">
              <!-- Column name, type, and PK icon - inline -->
              <Label :for="`edit-field-${col}`" class="text-xs font-medium whitespace-nowrap">
                {{ col }}<span v-if="columnTypeMap[col]" class="text-[10px] text-muted-foreground font-mono font-normal ml-1">{{ columnTypeMap[col] }}</span><svg
                  v-if="columnIsPK[col]"
                  xmlns="http://www.w3.org/2000/svg"
                  width="10"
                  height="10"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="text-amber-500 ml-1.5 inline-block"
                ><circle cx="7.5" cy="15.5" r="5.5" /><path d="m21 2-9.6 9.6" /><path d="m15.5 7.5 3 3L22 7l-3-3" /></svg>
              </Label>
              <div class="flex-1" />
              <!-- Set to NULL toggle -->
              <label class="text-xs text-muted-foreground flex gap-1 cursor-pointer select-none items-center">
                <input
                  type="checkbox"
                  class="h-3 w-3 cursor-pointer"
                  :checked="editForm[col]?.setNull"
                  @change="editForm[col] = { ...editForm[col], setNull: !editForm[col]?.setNull, value: editForm[col]?.value ?? '' }"
                >
                NULL
              </label>
            </div>
            <Input
              :id="`edit-field-${col}`"
              :type="getEditInputType(col)"
              :model-value="editForm[col]?.setNull ? '' : (editForm[col]?.value ?? '')"
              :disabled="editForm[col]?.setNull"
              :placeholder="editForm[col]?.setNull ? 'NULL' : ''"
              :step="getEditInputType(col) === 'number' ? 'any' : undefined"
              class="text-xs font-mono h-7"
              :class="{ 'border-destructive': editErrors[col] }"
              @update:model-value="(v) => editForm[col] = { ...editForm[col], value: String(v), setNull: editForm[col]?.setNull ?? false }"
            />
            <p v-if="editErrors[col]" class="text-xs text-destructive">
              {{ editErrors[col] }}
            </p>
          </div>
        </div>

        <!-- Footer actions -->
        <div class="px-6 py-3 border-t flex gap-2 justify-end">
          <Button variant="outline" size="sm" :disabled="isSaving" @click="editDialogOpen = false">
            {{ t('common.buttons.cancel') }}
          </Button>
          <Button size="sm" :disabled="isSaving" @click="confirmEdit">
            <Spinner v-if="isSaving" size="sm" class="mr-1.5" />
            {{ t('common.buttons.save') }}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  </div>
</template>

<style scoped>
.data-table {
  border-collapse: collapse;
}

.data-table-header {
  position: sticky;
  top: 0;
  z-index: 10;
  background-color: hsl(var(--muted));
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 500;
  max-width: 200px;
  min-width: 80px;
}

/* Sticky actions column — right-pinned, never scrolls away */
.data-table-actions-col {
  position: sticky;
  right: 0;
  background-color: hsl(var(--muted));
  z-index: 11;
  min-width: 72px;
  width: 72px;
  white-space: nowrap;
}

/* In body rows the actions cell background should match the row hover state */
tr:hover .data-table-actions-col {
  background-color: hsl(var(--muted) / 0.5);
}

.col-header-cell {
  display: flex;
  align-items: center;
  gap: 4px;
  max-width: 180px;
  overflow: hidden;
}

.col-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.col-type {
  font-size: 10px;
  font-weight: 400;
  font-family: monospace;
  color: hsl(var(--muted-foreground));
  white-space: nowrap;
  opacity: 0.8;
  flex-shrink: 0;
  max-width: 72px;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
