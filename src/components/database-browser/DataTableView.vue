<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  computeOffset,
  computeTotalPages,
  formatTableValue,
  isTableNullValue,
  rowsToCsv,
} from './dataTableHelpers'

interface TableDataResult {
  columns: string[]
  rows: Record<string, unknown>[]
  rows_affected?: number
  execution_time_ms?: number
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

const visibleColumns = computed(() =>
  data.value ? data.value.columns.filter(c => !hiddenColumns.value.has(c)) : [],
)

const totalPages = computed(() => computeTotalPages(totalCount.value, rowsPerPage.value))

const offset = computed(() => computeOffset(currentPage.value, rowsPerPage.value))

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
    const result = await invoke<TableDataResult>('get_table_data', {
      connectionId: props.connectionId,
      table: props.tableName,
      schema: props.schema ?? null,
      filter: appliedFilter.value.trim() || null,
      limit: rowsPerPage.value,
      offset: offset.value,
    })
    data.value = result
    executionTimeMs.value = result.execution_time_ms ?? null
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

async function refresh() {
  await Promise.all([fetchData(), fetchCount()])
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
  // Only fetch the new page of data; the total count does not change on page navigation.
  // The count is refreshed when a filter is applied via applyFilter() or refresh().
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

function exportCSV() {
  if (!data.value)
    return

  const csv = rowsToCsv(data.value.rows, visibleColumns.value)
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${props.tableName}.csv`
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
}

const formatValue = formatTableValue
const isNullValue = isTableNullValue

onMounted(refresh)

watch(
  () => [props.connectionId, props.tableName, props.schema] as const,
  () => {
    currentPage.value = 1
    appliedFilter.value = ''
    filterInput.value = ''
    hiddenColumns.value = new Set()
    refresh()
  },
)
</script>

<template>
  <div class="data-table-view flex flex-col h-full" @click="showColumnMenu = false">
    <!-- Filter / Toolbar bar -->
    <div class="border-b flex items-center gap-1.5 px-3 py-1.5 bg-muted/20">
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
        class="flex-1 h-7 text-xs font-mono"
        @keydown.enter="applyFilter"
      />

      <!-- Clear filter button -->
      <Button
        v-if="filterInput || appliedFilter"
        variant="ghost"
        size="icon"
        class="h-7 w-7 flex-shrink-0"
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
        class="h-7 w-7 flex-shrink-0"
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
      <div class="relative flex-shrink-0">
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
          class="text-popover-foreground border rounded-md bg-popover shadow-md absolute right-0 top-full z-50 mt-1 min-w-36 max-h-64 overflow-auto"
          @click.stop
        >
          <div class="p-1 text-xs font-semibold text-muted-foreground px-2 py-1.5 border-b">
            {{ t('components.dataTableView.columns') }}
          </div>
          <div class="p-1">
            <button
              v-for="col in data.columns"
              :key="col"
              class="w-full text-sm px-2 py-1 rounded flex items-center gap-2 cursor-pointer hover:bg-accent text-left"
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
              <span v-else class="inline-block w-3 flex-shrink-0" />
              <span class="truncate">{{ col }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- Export CSV button (current page) -->
      <Button
        variant="ghost"
        size="icon"
        class="h-7 w-7 flex-shrink-0"
        :disabled="!data || data.rows.length === 0"
        :title="t('components.dataTableView.exportCsv')"
        @click.stop="exportCSV"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 10 12 15 17 10" />
          <line x1="12" x2="12" y1="15" y2="3" />
        </svg>
      </Button>

      <!-- Apply Filters button -->
      <Button
        size="sm"
        class="h-7 text-xs flex-shrink-0"
        @click.stop="applyFilter"
      >
        {{ t('components.dataTableView.applyFilters') }}
      </Button>
    </div>

    <!-- Table area -->
    <div class="flex-1 overflow-auto relative">
      <!-- Loading overlay -->
      <div
        v-if="loading"
        class="absolute inset-0 bg-background/60 flex items-center justify-center z-10"
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
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-red-600 dark:text-red-400 mt-0.5 flex-shrink-0">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" x2="12" y1="8" y2="12" />
              <line x1="12" x2="12.01" y1="16" y2="16" />
            </svg>
            <div>
              <p class="text-sm font-medium text-red-800 dark:text-red-200">
                {{ t('components.dataTableView.error') }}
              </p>
              <p class="text-sm text-red-700 dark:text-red-300 mt-1">
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
              {{ col }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, i) in data.rows"
            :key="i"
            class="border-b hover:bg-muted/50"
          >
            <td class="px-3 py-1.5 text-xs text-muted-foreground text-center w-10">
              {{ offset + i + 1 }}
            </td>
            <td
              v-for="col in visibleColumns"
              :key="`${i}-${col}`"
              class="px-3 py-1.5 text-sm max-w-xs truncate"
              :class="{ 'italic text-muted-foreground': isNullValue(row[col]) }"
              :title="formatValue(row[col])"
            >
              {{ formatValue(row[col]) }}
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
    <div class="border-t flex items-center justify-between px-3 py-1 text-xs text-muted-foreground bg-muted/20">
      <!-- Left: row count + timing -->
      <div class="flex gap-4 items-center">
        <span v-if="totalCount > 0">
          <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="inline mr-1">
            <path d="M12 3v18" />
            <rect width="18" height="18" x="3" y="3" rx="2" />
            <path d="M3 9h18" />
            <path d="M3 15h18" />
          </svg>
          {{ totalCount.toLocaleString() }} {{ t('components.dataTableView.rows') }}
        </span>
        <span v-if="formattedTime">
          <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="inline mr-1">
            <circle cx="12" cy="12" r="10" />
            <polyline points="12 6 12 12 16 14" />
          </svg>
          {{ formattedTime }}
        </span>
      </div>

      <!-- Right: pagination controls -->
      <div class="flex items-center gap-1">
        <!-- First page -->
        <Button
          variant="ghost"
          size="icon"
          class="h-6 w-6"
          :disabled="currentPage === 1 || loading"
          @click="goToPage(1)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m11 17-5-5 5-5" />
            <path d="m18 17-5-5 5-5" />
          </svg>
        </Button>

        <!-- Prev page -->
        <Button
          variant="ghost"
          size="icon"
          class="h-6 w-6"
          :disabled="currentPage === 1 || loading"
          @click="goToPage(currentPage - 1)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m15 18-6-6 6-6" />
          </svg>
        </Button>

        <span class="px-1">{{ t('components.dataTableView.page', { page: currentPage, total: totalPages }) }}</span>

        <!-- Next page -->
        <Button
          variant="ghost"
          size="icon"
          class="h-6 w-6"
          :disabled="currentPage >= totalPages || loading"
          @click="goToPage(currentPage + 1)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m9 18 6-6-6-6" />
          </svg>
        </Button>

        <!-- Last page -->
        <Button
          variant="ghost"
          size="icon"
          class="h-6 w-6"
          :disabled="currentPage >= totalPages || loading"
          @click="goToPage(totalPages)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m13 17 5-5-5-5" />
            <path d="m6 17 5-5-5-5" />
          </svg>
        </Button>

        <!-- Rows per page -->
        <span class="ml-2">{{ t('components.dataTableView.rowsPerPage') }}</span>
        <Select
          :model-value="String(rowsPerPage)"
          @update:model-value="changeRowsPerPage"
        >
          <SelectTrigger class="h-6 text-xs w-20 ml-1">
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
  white-space: nowrap;
}
</style>
