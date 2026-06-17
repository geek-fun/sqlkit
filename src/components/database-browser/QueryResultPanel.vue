<script setup lang="ts">
import type { QueryResult } from '@/store/tabStore'
import type { ApiError } from '@/types/api'
import type { ExplainResult } from '@/types/explainPlan'
import type { ColumnFilter, SortColumn } from '@/types/grid'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ExplainPlanPanel } from '@/components/explain-plan'
import DataGrid from '@/components/grid/DataGrid.vue'
import { Button } from '@/components/ui/button'
import { Spinner } from '@/components/ui/spinner'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { useDataGridCopy } from '@/composables/useDataGridCopy'
import { formatApiError } from '@/types/api'

type Props = {
  results?: QueryResult | null
  error?: ApiError | string | null
  isExecuting?: boolean
  executionTime?: number
  height?: string
  resizable?: boolean
  sql?: string
  connectionId?: string
  database?: string
  schema?: string
  explainPlan?: ExplainResult | null
  isExplaining?: boolean
  explainError?: string | null
}

const props = withDefaults(defineProps<Props>(), {
  results: null,
  error: null,
  isExecuting: false,
  executionTime: 0,
  height: '300px',
  resizable: true,
  sql: undefined,
  connectionId: undefined,
  database: undefined,
  schema: undefined,
  explainPlan: null,
  isExplaining: false,
  explainError: null,
})

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'resize', height: number): void
  (e: 'refresh'): void
}>()

const activeOutputView = ref<'results' | 'explain'>('results')

watch(() => props.explainPlan, (plan) => {
  if (plan)
    activeOutputView.value = 'explain'
})

watch(() => props.results, (res) => {
  if (res && !props.explainPlan)
    activeOutputView.value = 'results'
})

const { t } = useI18n()
const copyUtil = useDataGridCopy()

const panelHeight = ref(300)
const isResizing = ref(false)
const startY = ref(0)
const startHeight = ref(0)

const formattedTime = computed(() => {
  if (!props.executionTime)
    return ''
  return props.executionTime < 1000
    ? `${props.executionTime}ms`
    : `${(props.executionTime / 1000).toFixed(2)}s`
})

// ── Sort/Filter Re-execution ──
const gridLoading = ref(false)
const gridError = ref<string | null>(null)
const gridResults = ref<QueryResult | null>(null)
const gridExecutionTimeMs = ref<number | undefined>(undefined)
const activeSort = ref<SortColumn[]>([])
const activeFilters = ref<ColumnFilter[]>([])

// Sync from props when results change
watch(() => props.results, (r) => {
  if (r) {
    gridResults.value = r
    gridError.value = null
  }
}, { immediate: true })

function buildWrappedSql(sort: SortColumn[], filters: ColumnFilter[]): string {
  if (!props.sql)
    return props.sql ?? ''
  let sql = `SELECT * FROM (${props.sql}) AS _sqlkit_grid`

  // Build WHERE clause
  if (filters.length > 0) {
    const clauses = filters.map((f) => {
      const esc = (v: string) => v.replace(/'/g, '\'\'')
      const col = f.column
      switch (f.operator) {
        case 'eq':
          return `${col} = '${esc(f.value)}'`
        case 'neq':
          return `${col} != '${esc(f.value)}'`
        case 'like':
          return `${col} LIKE '%${esc(f.value)}%'`
        case 'gt':
          return `${col} > '${esc(f.value)}'`
        case 'lt':
          return `${col} < '${esc(f.value)}'`
        case 'gte':
          return `${col} >= '${esc(f.value)}'`
        case 'lte':
          return `${col} <= '${esc(f.value)}'`
        case 'between':
          return `${col} >= '${esc(f.value)}' AND ${col} <= '${esc(f.value2 ?? '')}'`
        default:
          return ''
      }
    }).filter(Boolean)
    if (clauses.length > 0) {
      sql += ` WHERE ${clauses.join(' AND ')}`
    }
  }

  // Build ORDER BY clause
  if (sort.length > 0) {
    sql += ` ORDER BY ${sort.map(s => `${s.column} ${s.direction}`).join(', ')}`
  }

  return sql
}

async function handleSortChange(sort: SortColumn[]) {
  activeSort.value = sort
  await reExecuteWithSortFilter()
}

async function handleFilterChange(filters: ColumnFilter[]) {
  activeFilters.value = filters
  await reExecuteWithSortFilter()
}

async function reExecuteWithSortFilter() {
  if (!props.connectionId || !props.sql)
    return

  gridLoading.value = true
  gridError.value = null

  try {
    const wrappedSql = buildWrappedSql(activeSort.value, activeFilters.value)
    const start = performance.now()
    const result = await invoke<{ status: string, data: QueryResult }>('execute_query', {
      connectionId: props.connectionId,
      sql: wrappedSql,
      database: props.database ?? null,
    })
    const elapsed = Math.round(performance.now() - start)

    if (result.status === 'success') {
      gridResults.value = result.data
      gridExecutionTimeMs.value = elapsed
    }
    else {
      gridError.value = t('components.queryResult.error')
    }
  }
  catch (err) {
    gridError.value = String(err)
  }
  finally {
    gridLoading.value = false
  }
}

function handleRefresh() {
  activeSort.value = []
  activeFilters.value = []
  gridResults.value = props.results ?? null
  gridError.value = null
  emit('refresh')
}

// ── Resize ──
function startResize(e: MouseEvent) {
  if (!props.resizable)
    return
  isResizing.value = true
  startY.value = e.clientY
  startHeight.value = panelHeight.value

  document.addEventListener('mousemove', handleResize)
  document.addEventListener('mouseup', stopResize)
}

function handleResize(e: MouseEvent) {
  if (!isResizing.value)
    return
  const diff = startY.value - e.clientY
  const newHeight = Math.max(100, Math.min(800, startHeight.value + diff))
  panelHeight.value = newHeight
  emit('resize', newHeight)
}

function stopResize() {
  isResizing.value = false
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
}

function close() {
  emit('close')
}

// Determine what to show
const displayResults = computed(() => gridResults.value ?? props.results)
const displayColumnTypes = computed(() => {
  const result = displayResults.value
  if (!result || !result.columnTypes || result.columnTypes.length === 0)
    return {}
  return Object.fromEntries(result.columns.map((col, i) => [col, result.columnTypes[i] ?? '']))
})
const displayError = computed(() => gridError.value ?? props.error)
const displayErrorMessage = computed(() => {
  const err = displayError.value
  if (!err)
    return null
  if (typeof err === 'string')
    return err
  return formatApiError(err, t)
})
const displayExecuting = computed(() => props.isExecuting || gridLoading.value)
const displayExecutionTime = computed(() => gridExecutionTimeMs.value ?? props.executionTime)
</script>

<template>
  <div
    class="query-result-panel border-t bg-background flex flex-col"
    :style="{ height: `${panelHeight}px` }"
  >
    <!-- Resize handle -->
    <div
      v-if="resizable"
      class="resize-handle bg-transparent h-1 cursor-ns-resize transition-colors hover:bg-primary/20"
      @mousedown="startResize"
    />

    <!-- Tab bar (Results | Explain) -->
    <div class="text-xs px-3 border-b bg-muted/20 flex shrink-0 gap-1 h-9 items-center">
      <Button
        size="sm"
        variant="ghost"
        class="text-xs px-2 h-6"
        :class="activeOutputView === 'results' ? 'bg-accent text-accent-foreground' : ''"
        :disabled="!displayResults && !displayExecuting"
        @click="activeOutputView = 'results'"
      >
        {{ t('pages.queries.explain.tabLabels.results') }}
      </Button>
      <Button
        size="sm"
        variant="ghost"
        class="text-xs px-2 h-6"
        :class="activeOutputView === 'explain' ? 'bg-accent text-accent-foreground' : ''"
        :disabled="!explainPlan && !isExplaining && !explainError"
        @click="activeOutputView = 'explain'"
      >
        {{ t('pages.queries.explain.tabLabels.explain') }}
      </Button>

      <!-- Export actions -->
      <span class="text-muted-foreground/30 mx-1">|</span>
      <template v-if="displayResults && displayResults.rows.length > 0 && activeOutputView === 'results'">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger as-child>
              <Button variant="ghost" size="sm" class="text-foreground p-0 h-8 w-8 hover:bg-muted" @click="copyUtil.copyRowsAs(displayResults.rows, displayResults.columns, 'csv')">
                <span class="i-carbon-table-split h-4.5 w-4.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{{ t('components.dataGrid.export.copyAllCsv') }}</TooltipContent>
          </Tooltip>
        </TooltipProvider>
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger as-child>
              <Button variant="ghost" size="sm" class="text-foreground p-0 h-8 w-8 hover:bg-muted" @click="copyUtil.copyRowsAs(displayResults.rows, displayResults.columns, 'json')">
                <span class="i-carbon-code h-4.5 w-4.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{{ t('components.dataGrid.export.copyAllJson') }}</TooltipContent>
          </Tooltip>
        </TooltipProvider>
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger as-child>
              <Button variant="ghost" size="sm" class="text-foreground p-0 h-8 w-8 hover:bg-muted" @click="copyUtil.copyRowsAs(displayResults.rows, displayResults.columns, 'insert')">
                <span class="i-carbon-sql h-4.5 w-4.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{{ t('components.dataGrid.export.copyAllInsert') }}</TooltipContent>
          </Tooltip>
        </TooltipProvider>
        <span class="text-xs text-muted-foreground mx-1">|</span>
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger as-child>
              <Button variant="ghost" size="sm" class="text-foreground p-0 h-8 w-8 hover:bg-muted" @click="copyUtil.exportToFile(displayResults.rows, displayResults.columns, 'csv')">
                <span class="i-carbon-document-export h-4.5 w-4.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{{ t('components.dataGrid.export.exportCsv') }}</TooltipContent>
          </Tooltip>
        </TooltipProvider>
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger as-child>
              <Button variant="ghost" size="sm" class="text-foreground p-0 h-8 w-8 hover:bg-muted" @click="copyUtil.exportToFile(displayResults.rows, displayResults.columns, 'json')">
                <span class="i-carbon-document-export h-4.5 w-4.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{{ t('components.dataGrid.export.exportJson') }}</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </template>
      <div class="flex-1" />
      <div v-if="displayResults && activeOutputView === 'results'" class="text-xs text-muted-foreground flex gap-2 items-center">
        <span v-if="displayResults.columns.length > 0">{{ t('components.queryResult.rows', { count: displayResults.rows.length }) }}</span>
        <span v-else-if="displayResults.rowsAffected !== undefined">{{ t('components.queryResult.rowsAffected', { count: displayResults.rowsAffected }) }}</span>
        <span v-if="formattedTime">• {{ t('components.queryResult.time', { time: formattedTime }) }}</span>
      </div>
      <Button
        variant="ghost"
        size="icon"
        class="h-6 w-6"
        :title="t('components.queryResult.close')"
        @click="close"
      >
        <span class="i-carbon-close h-3.5 w-3.5" />
      </Button>
    </div>

    <!-- Content -->
    <div class="flex-1 min-h-0">
      <!-- Results view -->
      <template v-if="activeOutputView === 'results'">
        <!-- Loading state -->
        <div v-if="displayExecuting && !displayResults" class="flex h-full items-center justify-center">
          <div class="text-center">
            <Spinner class="mx-auto mb-2 h-8 w-8" />
            <p class="text-sm text-muted-foreground">
              {{ t('components.queryResult.executing') }}
            </p>
          </div>
        </div>

        <!-- Error state -->
        <div v-else-if="displayError && !displayResults" class="p-4">
          <div class="p-4 border border-destructive/30 rounded-md bg-destructive/5">
            <div class="flex gap-3 items-start">
              <span class="i-carbon-warning-alt text-destructive mt-0.5 flex-shrink-0 h-5 w-5" />
              <div>
                <p
                  v-for="(line, index) in (typeof displayError === 'string' ? displayError : formatApiError(displayError, t)).split('\n\n')"
                  :key="index"
                  class="text-sm text-destructive"
                >
                  {{ line }}
                </p>
              </div>
            </div>
          </div>
        </div>

        <!-- DataGrid (SELECT results with columns) -->
        <DataGrid
          v-else-if="displayResults && displayResults.columns.length > 0"
          :columns="displayResults.columns"
          :column-types="displayColumnTypes"
          :rows="displayResults.rows"
          :row-count="displayResults.rows.length"
          :execution-time-ms="displayExecutionTime"
          :connection-id="connectionId"
          :database="database"
          :schema="schema"
          :loading="displayExecuting"
          :hide-toolbar="true"
          :error="displayErrorMessage"
          @sort-change="handleSortChange"
          @filter-change="handleFilterChange"
          @refresh="handleRefresh"
        />

        <!-- DML/DLL success (no columns returned) -->
        <div v-else-if="displayResults && displayResults.columns.length === 0" class="flex h-full items-center justify-center">
          <div class="text-muted-foreground text-center">
            <span class="i-carbon-checkmark mx-auto mb-2 opacity-50 h-8 w-8 block" />
            <p class="text-sm font-medium">
              {{ t('components.queryResult.success') }}
            </p>
            <p v-if="displayResults.rowsAffected !== undefined" class="text-xs mt-1">
              {{ t('components.queryResult.rowsAffected', { count: displayResults.rowsAffected }) }}
            </p>
            <p v-else class="text-xs mt-1">
              {{ t('components.queryResult.commandCompleted') }}
            </p>
          </div>
        </div>

        <!-- No results (no query executed yet) -->
        <div v-else class="flex h-full items-center justify-center">
          <div class="text-muted-foreground text-center">
            <span class="i-carbon-document-blank mx-auto mb-2 opacity-40 h-8 w-8 block" />
            <p class="text-sm">
              {{ t('components.queryResult.noResults') }}
            </p>
          </div>
        </div>
      </template>

      <!-- Explain view -->
      <ExplainPlanPanel
        v-else
        :explain-result="explainPlan"
        :loading="isExplaining || false"
        :error="explainError"
      />
    </div>
  </div>
</template>

<style scoped>
.resize-handle {
  position: relative;
}

.resize-handle::before {
  content: '';
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 30px;
  height: 3px;
  background-color: currentColor;
  opacity: 0.3;
  border-radius: 2px;
}

.resize-handle:hover::before {
  opacity: 0.6;
}
</style>
