<script setup lang="ts">
import type { ColumnInfo } from '@/composables/sqlCompletion/metadata'
import type { QueryResult } from '@/store/tabStore'
import type { ApiError, ApiResponse } from '@/types/api'
import type { ExplainResult } from '@/types/explainPlan'
import type { ColumnFilter } from '@/types/grid'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ExplainPlanPanel } from '@/components/explain-plan'
import { DataGrid } from '@/components/grid'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Spinner } from '@/components/ui/spinner'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { useDataGridCopy } from '@/composables/useDataGridCopy'
import { useConnectionStore } from '@/store'
import { formatApiError, isApiError } from '@/types/api'

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
const selectedResult = ref('0')
const resultSets = computed(() => props.results?.resultSets ?? [])
const isScriptResult = computed(() => resultSets.value.length > 0)
const selectedResults = computed(() => resultSets.value[Number(selectedResult.value)] ?? props.results)

const handleResultSelectorOpen = (open: boolean) => {
  if (open)
    activeOutputView.value = 'results'
}

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
const exportActions = [
  { label: 'copyAllCsv', icon: 'i-lucide-file-spreadsheet', format: 'csv', download: false },
  { label: 'copyAllJson', icon: 'i-lucide-file-json', format: 'json', download: false },
  { label: 'copyAllInsert', icon: 'i-lucide-database', format: 'insert', download: false },
  { label: 'exportCsv', icon: 'i-lucide-file-down', format: 'csv', download: true },
  { label: 'exportJson', icon: 'i-lucide-file-down', format: 'json', download: true },
] as const

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

// ── Filter Re-execution (backend-driven) ──
// Sorting is handled client-side by DataGrid over the returned rows; only
// filters still need a round-trip because they change the result set itself.
const gridLoading = ref(false)
const gridError = ref<string | null>(null)
const gridResults = ref<QueryResult | null>(null)
const gridExecutionTimeMs = ref<number | undefined>(undefined)
const activeFilters = ref<ColumnFilter[]>([])

// ── Result editability (single-table SELECTs only) ──
const connectionStore = useConnectionStore()

type SqlEditability = {
  editable: boolean
  tableName?: string | null
  schema?: string | null
  reason?: string | null
}

const editability = ref<SqlEditability>({ editable: false })
const editableTableName = ref<string | undefined>(undefined)
const editableSchema = ref<string | undefined>(undefined)
const editablePrimaryKeys = ref<string[]>([])

const connectionTypeForAnalysis = computed(() => {
  if (!props.connectionId)
    return undefined
  const conn = connectionStore.getConnectionById(props.connectionId)
  return conn?.type ? String(conn.type).toLowerCase() : undefined
})

async function refreshEditability() {
  editability.value = { editable: false }
  editableTableName.value = undefined
  editableSchema.value = undefined
  editablePrimaryKeys.value = []

  if (isScriptResult.value || !props.sql || !props.connectionId) {
    return
  }

  try {
    const result = await invoke<SqlEditability>('analyze_sql_editability_command', {
      sql: props.sql,
      databaseType: connectionTypeForAnalysis.value,
    })
    editability.value = result
    if (!result.editable || !result.tableName) {
      return
    }

    editableTableName.value = result.tableName
    editableSchema.value = result.schema ?? undefined

    // Fetch primary keys so edit/delete can locate rows.
    const columns = await invoke<ColumnInfo[]>('list_columns', {
      connectionId: props.connectionId,
      database: props.database ?? null,
      schema: result.schema ?? props.schema ?? null,
      tableName: result.tableName,
    })
    editablePrimaryKeys.value = (columns ?? [])
      .filter(col => col.is_primary_key)
      .map(col => col.name)
  }
  catch {
    // Analysis is best-effort; fall back to read-only grid on failure.
    editability.value = { editable: false }
  }
}

watch(
  () => [props.sql, props.connectionId, props.database, isScriptResult.value],
  () => {
    void refreshEditability()
  },
  { immediate: true },
)

// Sync from props when results change
watch(() => props.results, (r) => {
  selectedResult.value = '0'
  gridResults.value = r ?? null
  gridError.value = null
  gridExecutionTimeMs.value = undefined
  activeFilters.value = []
}, { immediate: true })

async function handleFilterChange(filters: ColumnFilter[]) {
  activeFilters.value = filters
  await reExecuteFiltered()
}

async function reExecuteFiltered() {
  if (isScriptResult.value || !props.connectionId || !props.sql)
    return

  gridLoading.value = true
  gridError.value = null

  try {
    const start = performance.now()
    const result = await invoke<ApiResponse<QueryResult>>('execute_sorted_query', {
      connectionId: props.connectionId,
      database: props.database ?? null,
      options: {
        originalSql: props.sql,
        schemaContext: props.schema ?? null,
        sort: [],
        filters: activeFilters.value.map(f => ({
          column: f.column,
          operator: f.operator.toUpperCase(),
          value: f.value,
          value2: f.value2 ?? null,
        })),
        limit: null,
        offset: null,
      },
    })
    const elapsed = Math.round(performance.now() - start)

    if (result.status === 'success') {
      gridResults.value = result.data
      gridExecutionTimeMs.value = elapsed
    }
    else if (isApiError(result)) {
      gridError.value = formatApiError(result.error, t)
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
const displayResults = computed(() => isScriptResult.value ? selectedResults.value : gridResults.value ?? props.results)
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
const handleExport = (action: typeof exportActions[number]) => {
  const result = displayResults.value
  if (!result?.rows.length)
    return
  if (action.download)
    void copyUtil.exportToFile(result.rows, result.columns, action.format)
  else
    void copyUtil.copyRowsAs(result.rows, result.columns, action.format)
}
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

    <div class="result-toolbar text-xs px-3 border-b bg-muted/20 flex shrink-0 gap-1 h-9 min-w-0 items-center">
      <Select
        v-if="resultSets.length > 1"
        v-model="selectedResult"
        :disabled="displayExecuting"
        @update:open="handleResultSelectorOpen"
        @update:model-value="activeOutputView = 'results'"
      >
        <SelectTrigger
          class="result-set-selector text-xs px-2 shrink-0 h-7 w-40"
          :class="activeOutputView === 'results' ? 'bg-accent text-accent-foreground' : ''"
          :aria-label="t('components.queryResult.selectResult')"
        >
          <SelectValue>
            {{ t('components.queryResult.resultSet', { index: Number(selectedResult) + 1, count: resultSets.length }) }}
          </SelectValue>
        </SelectTrigger>
        <SelectContent>
          <SelectItem v-for="(_, index) in resultSets" :key="index" :value="String(index)">
            {{ t('components.queryResult.resultSet', { index: index + 1, count: resultSets.length }) }}
          </SelectItem>
        </SelectContent>
      </Select>
      <Button
        v-else
        size="sm"
        variant="ghost"
        class="text-xs px-2 shrink-0 h-6"
        :class="activeOutputView === 'results' ? 'bg-accent text-accent-foreground' : ''"
        :aria-pressed="activeOutputView === 'results'"
        :disabled="!displayResults && !displayExecuting"
        @click="activeOutputView = 'results'"
      >
        {{ t('pages.queries.explain.tabLabels.results') }}
      </Button>
      <Button
        size="sm"
        variant="ghost"
        class="text-xs px-2 shrink-0 h-6"
        :class="activeOutputView === 'explain' ? 'bg-accent text-accent-foreground' : ''"
        :aria-pressed="activeOutputView === 'explain'"
        :disabled="!explainPlan && !isExplaining && !explainError"
        @click="activeOutputView = 'explain'"
      >
        {{ t('pages.queries.explain.tabLabels.explain') }}
      </Button>

      <template v-if="activeOutputView === 'results'">
        <div class="mx-1 bg-border shrink-0 h-4 w-px" aria-hidden="true" />
        <div class="result-export-actions flex shrink-0 gap-0.5 items-center">
          <TooltipProvider>
            <Tooltip v-for="action in exportActions" :key="action.label">
              <TooltipTrigger as-child>
                <Button
                  variant="ghost"
                  size="icon"
                  class="text-muted-foreground h-7 w-7 hover:text-foreground"
                  :aria-label="t(`components.dataGrid.export.${action.label}`)"
                  :disabled="!displayResults?.rows.length"
                  @click="handleExport(action)"
                >
                  <span :class="action.icon" class="shrink-0 h-3.5 w-3.5" aria-hidden="true" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{{ t(`components.dataGrid.export.${action.label}`) }}</TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button
              variant="ghost"
              size="icon"
              class="result-export-menu text-muted-foreground shrink-0 h-7 w-7 hover:text-foreground"
              :aria-label="t('components.queryResult.exportActions')"
              :title="t('components.queryResult.exportActions')"
              :disabled="!displayResults?.rows.length"
            >
              <span class="i-lucide-ellipsis h-3.5 w-3.5" aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <template v-for="(action, index) in exportActions" :key="action.label">
              <DropdownMenuSeparator v-if="index === 3" />
              <DropdownMenuItem @select="handleExport(action)">
                <span :class="action.icon" class="h-3.5 w-3.5" aria-hidden="true" />
                {{ t(`components.dataGrid.export.${action.label}`) }}
              </DropdownMenuItem>
            </template>
          </DropdownMenuContent>
        </DropdownMenu>
      </template>
      <div class="flex-1" />
      <TooltipProvider v-if="isScriptResult && activeOutputView === 'results'">
        <Tooltip>
          <TooltipTrigger as-child>
            <Button variant="ghost" size="icon" class="text-muted-foreground shrink-0 h-6 w-6" :aria-label="t('components.queryResult.scriptReadOnly')">
              <span class="i-lucide-lock-keyhole h-3 w-3" aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{{ t('components.queryResult.scriptReadOnly') }}</TooltipContent>
        </Tooltip>
      </TooltipProvider>
      <Button
        variant="ghost"
        size="icon"
        class="text-muted-foreground shrink-0 h-7 w-7 hover:text-foreground"
        :aria-label="t('components.queryResult.close')"
        :title="t('components.queryResult.close')"
        @click="close"
      >
        <span class="i-lucide-x shrink-0 h-3.5 w-3.5" aria-hidden="true" />
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
          :key="isScriptResult ? selectedResult : 'query'"
          :columns="displayResults.columns"
          :column-types="displayColumnTypes"
          :rows="displayResults.rows"
          :row-count="displayResults.rows.length"
          :truncated="displayResults.truncated"
          :execution-time-ms="displayExecutionTime"
          :connection-id="connectionId"
          :database="database"
          :schema="editableSchema ?? schema"
          :table-name="isScriptResult ? undefined : editableTableName"
          :primary-keys="editablePrimaryKeys"
          :editability-reason="isScriptResult ? t('components.queryResult.scriptReadOnly') : editability.reason ?? undefined"
          :filterable="!isScriptResult"
          :loading="displayExecuting"
          :hide-toolbar="true"
          :hide-batch-actions="true"
          :error="displayErrorMessage"
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

    <!-- Status footer for statements without a result grid -->
    <div
      v-if="activeOutputView === 'results' && displayResults && displayResults.columns.length === 0"
      class="text-xs text-muted-foreground px-3 py-1.5 border-t bg-muted/20 flex shrink-0 gap-3 items-center overflow-hidden"
    >
      <span v-if="database" class="tabular-nums">{{ t('pages.queries.status.database') }}: {{ database }}</span>
      <span v-if="displayResults.rowsAffected !== undefined" class="tabular-nums">
        {{ t('components.queryResult.rowsAffected', { count: displayResults.rowsAffected }) }}
      </span>
      <span v-else class="tabular-nums">{{ t('components.queryResult.commandCompleted') }}</span>
      <span v-if="formattedTime" class="text-muted-foreground/70 tabular-nums">
        {{ t('components.queryResult.time', { time: formattedTime }) }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.query-result-panel {
  container-type: inline-size;
  container-name: query-result;
}

.result-export-menu {
  display: none;
}

@container query-result (max-width: 720px) {
  .result-export-actions {
    display: none;
  }

  .result-export-menu {
    display: inline-flex;
  }
}

@container query-result (max-width: 400px) {
  .result-set-selector {
    width: 8rem;
  }
}

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
