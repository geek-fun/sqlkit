<script setup lang="ts">
import type { MigrationMapping, MigrationPreview, MigrationRequest, MigrationTablePlan, TransferScope } from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

import { ConnectionStatus, useConnectionStore } from '@/store/connectionStore'
import ConnectionSelector from '../shared/ConnectionSelector.vue'
import TransferStepCard from '../shared/TransferStepCard.vue'

const { t } = useI18n()
const connectionStore = useConnectionStore()

const sourceConnectionId = ref('')
const sourceDatabase = ref('')
const sourceSchema = ref('')
const targetConnectionId = ref('')
const targetDatabase = ref('')
const targetSchema = ref('')

const scope = ref<TransferScope>('tables')
const createTargetDatabaseIfNotExists = ref(false)

const availableTables = ref<{ name: string, rowCount?: number }[]>([])
const selectedTables = ref<string[]>([])
const tablePlans = ref<MigrationTablePlan[]>([])
const preview = ref<MigrationPreview | null>(null)

const loadingTables = ref(false)
const loadingPreview = ref(false)
const executing = ref(false)
const result = ref<{ success: boolean, message: string } | null>(null)

const createTables = ref(true)
const dropTables = ref(false)
const batchSize = ref(5000)
const onError = ref<'skipRow' | 'skipTable' | 'abort'>('skipRow')

// Connection status
const isSourceConnected = computed(() => {
  if (!sourceConnectionId.value)
    return false
  return connectionStore.getConnectionStatus(sourceConnectionId.value) === ConnectionStatus.CONNECTED
})

const isTargetConnected = computed(() => {
  if (!targetConnectionId.value)
    return false
  return connectionStore.getConnectionStatus(targetConnectionId.value) === ConnectionStatus.CONNECTED
})

// Connection type
const sourceEngine = computed(() => {
  const conn = connectionStore.getConnectionById(sourceConnectionId.value)
  return conn?.type || ''
})

const targetEngine = computed(() => {
  const conn = connectionStore.getConnectionById(targetConnectionId.value)
  return conn?.type || ''
})

// Summaries
const sourceSummary = computed(() => {
  if (!sourceConnectionId.value)
    return ''

  switch (scope.value) {
    case 'server':
      return 'All databases will be migrated'
    case 'database':
      return sourceDatabase.value ? `All tables in ${sourceDatabase.value}` : 'Select a database'
    case 'tables':
    default:
      return selectedTables.value.length > 0
        ? `${selectedTables.value.length} tables`
        : ''
  }
})

const targetSummary = computed(() => {
  if (targetConnectionId.value)
    return `${targetDatabase.value || 'default'}`
  return ''
})

// Load tables
async function loadTables() {
  if (scope.value !== 'tables') {
    availableTables.value = []
    return
  }

  if (!sourceConnectionId.value || !sourceDatabase.value || !isSourceConnected.value)
    return

  loadingTables.value = true
  try {
    const tables = await invoke<{ name: string }[]>('list_tables', {
      connectionId: sourceConnectionId.value,
      database: sourceDatabase.value,
      schema: sourceSchema.value || null,
    })
    availableTables.value = tables
  }
  catch (error) {
    console.error('Failed to load tables:', error)
    availableTables.value = []
  }
  finally {
    loadingTables.value = false
  }
}

// Generate mappings
async function generateMappings() {
  if (!sourceConnectionId.value || !targetConnectionId.value || selectedTables.value.length === 0)
    return

  tablePlans.value = []

  for (const table of selectedTables.value) {
    try {
      const mappings = await invoke<MigrationMapping[]>('auto_map_migration_columns', {
        connectionId: sourceConnectionId.value,
        database: sourceDatabase.value || null,
        schema: sourceSchema.value || null,
        table,
        targetEngine: targetEngine.value,
      })

      tablePlans.value.push({
        sourceTable: table,
        targetTable: table,
        columnMappings: mappings,
      })
    }
    catch (error) {
      console.error(`Failed to map columns for ${table}:`, error)
    }
  }
}

// Preview migration
async function previewMigration() {
  if (tablePlans.value.length === 0)
    return

  loadingPreview.value = true
  try {
    preview.value = await invoke<MigrationPreview>('preview_migration_data', {
      request: buildRequest(),
    })
  }
  catch (error) {
    console.error('Failed to preview migration:', error)
  }
  finally {
    loadingPreview.value = false
  }
}

// Execute migration
async function executeMigration() {
  if (tablePlans.value.length === 0)
    return

  executing.value = true
  result.value = null

  try {
    const res = await invoke<{ success: boolean, processedRows: number, errorCount: number, durationMs: number }>(
      'execute_migration_data',
      { request: buildRequest() },
    )
    result.value = {
      success: res.success,
      message: res.success
        ? `${res.processedRows} rows migrated in ${res.durationMs}ms`
        : `${res.errorCount} errors`,
    }
  }
  catch (error) {
    result.value = { success: false, message: String(error) }
  }
  finally {
    executing.value = false
  }
}

// Build request
function buildRequest(): MigrationRequest {
  return {
    sourceConnectionId: sourceConnectionId.value,
    sourceDatabase: sourceDatabase.value || undefined,
    sourceSchema: sourceSchema.value || undefined,
    targetConnectionId: targetConnectionId.value,
    targetDatabase: targetDatabase.value || undefined,
    targetSchema: targetSchema.value || undefined,
    scope: scope.value,
    createTargetDatabaseIfNotExists: createTargetDatabaseIfNotExists.value,
    tablePlans: tablePlans.value,
    batchSize: batchSize.value,
    onError: onError.value,
    createTables: createTables.value,
    dropTables: dropTables.value,
  }
}

// Toggle table
function toggleTable(name: string) {
  const current = [...selectedTables.value]
  const index = current.indexOf(name)
  if (index > -1) {
    current.splice(index, 1)
  }
  else {
    current.push(name)
  }
  selectedTables.value = current
}

function selectAllTables() {
  selectedTables.value = availableTables.value.map(t => t.name)
}

function deselectAllTables() {
  selectedTables.value = []
}

// Watch for source changes
const sourceParams = computed(() => {
  if (scope.value !== 'tables')
    return null
  if (!isSourceConnected.value || !sourceDatabase.value)
    return null
  return {
    connectionId: sourceConnectionId.value,
    database: sourceDatabase.value,
    schema: sourceSchema.value,
  }
})

watch(sourceParams, (params, oldParams) => {
  if (params && JSON.stringify(params) !== JSON.stringify(oldParams)) {
    loadTables()
  }
}, { deep: true })

// Watch for target changes
const targetMigrationParams = computed(() => {
  if (scope.value !== 'tables')
    return null
  if (!isTargetConnected.value || !targetDatabase.value || !selectedTables.value.length)
    return null
  return {
    connectionId: targetConnectionId.value,
    database: targetDatabase.value,
    schema: targetSchema.value,
    tables: [...selectedTables.value],
  }
})

watch(targetMigrationParams, async (params, oldParams) => {
  if (params && JSON.stringify(params) !== JSON.stringify(oldParams)) {
    await generateMappings()
    await previewMigration()
  }
}, { deep: true })

const canExecute = computed(() => {
  // For server and database scopes, we don't need table plans - just connections
  if (scope.value === 'server' || scope.value === 'database') {
    return isSourceConnected.value && isTargetConnected.value && tablePlans.value.length >= 0
  }
  // For tables scope, require selected tables and table plans
  return isSourceConnected.value
    && isTargetConnected.value
    && selectedTables.value.length > 0
    && tablePlans.value.length > 0
})
</script>

<template>
  <div class="pb-6 flex flex-col gap-2.5">
    <!-- Source -->
    <TransferStepCard
      :title="t('transfer.migration.sourceConnection')"
      :step-number="1"
      icon="i-carbon-data-base"
      icon-class="text-emerald-600 dark:text-emerald-500"
      :summary="sourceSummary"
      :scope="scope"
      @update:scope="scope = $event"
    >
      <ConnectionSelector
        v-model:connection-id="sourceConnectionId"
        v-model:database="sourceDatabase"
        v-model:schema="sourceSchema"
        :show-schema="scope === 'tables'"
      />

      <!-- Scope-specific badges -->
      <div v-if="scope === 'server' && sourceConnectionId" class="mt-3 px-3 py-2 border border-emerald-500/20 rounded-md bg-emerald-500/5 flex gap-2 items-center">
        <span class="i-carbon-information text-emerald-600 h-4 w-4 dark:text-emerald-500" />
        <span class="text-xs text-emerald-700 font-medium dark:text-emerald-400">All databases will be migrated</span>
      </div>

      <div v-if="scope === 'database' && sourceDatabase" class="mt-3 px-3 py-2 border border-emerald-500/20 rounded-md bg-emerald-500/5 flex gap-2 items-center">
        <span class="i-carbon-information text-emerald-600 h-4 w-4 dark:text-emerald-500" />
        <span class="text-xs text-emerald-700 font-mono dark:text-emerald-400">All tables in <span class="font-semibold">{{ sourceDatabase }}</span> will be migrated</span>
      </div>

      <!-- Tables grid (only for 'tables' scope) -->
      <div v-if="scope === 'tables'" class="mt-4 pt-4 border-t border-border/40">
        <div class="mb-3 flex items-center justify-between">
          <Label class="text-[11px] text-muted-foreground tracking-wide font-semibold flex gap-1.5 uppercase items-center">
            <span class="i-carbon-table" />
            Tables
          </Label>
          <div class="flex gap-2 items-center">
            <Button variant="ghost" size="sm" class="text-xs px-2 h-8" @click="selectAllTables">
              Select All
            </Button>
            <Button variant="ghost" size="sm" class="text-xs px-2 h-8" @click="deselectAllTables">
              Deselect All
            </Button>
          </div>
        </div>

        <div v-if="loadingTables" class="text-xs text-muted-foreground p-6 border border-border/40 rounded-md border-dashed flex items-center justify-center">
          <span class="i-carbon-circle-dash mr-2 animate-spin" /> Loading tables...
        </div>

        <div v-else-if="availableTables.length === 0 && sourceDatabase" class="text-xs text-muted-foreground p-6 text-center border border-border/40 rounded-md border-dashed bg-muted/10 flex flex-col items-center justify-center">
          <span class="i-carbon-data-base mb-2 opacity-50 h-5 w-5" />
          No tables found
        </div>

        <div v-else class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border pr-2 gap-2 grid grid-cols-1 max-h-[300px] overflow-y-auto md:grid-cols-3 sm:grid-cols-2">
          <label
            v-for="table in availableTables"
            :key="table.name"
            class="px-3 py-1.5 border border-border/40 rounded-md flex cursor-pointer transition-colors items-center space-x-2 hover:bg-muted/50"
            :class="selectedTables.includes(table.name) ? 'border-primary/30 bg-primary/5' : 'bg-transparent'"
          >
            <Checkbox
              :id="`mig-table-${table.name}`"
              :checked="selectedTables.includes(table.name)"
              @update:checked="toggleTable(table.name)"
            />
            <div class="flex flex-1 min-w-0 items-center justify-between">
              <span class="text-xs leading-none font-mono truncate">{{ table.name }}</span>
              <Badge v-if="table.rowCount" variant="secondary" class="text-[10px] font-mono ml-2 px-1 py-0 bg-muted/50 uppercase tabular-nums">{{ table.rowCount }} rows</Badge>
            </div>
          </label>
        </div>

        <div v-if="availableTables.length > 0" class="text-[11px] text-muted-foreground font-mono mt-3 tabular-nums">
          {{ selectedTables.length }} / {{ availableTables.length }} tables selected
        </div>
      </div>
    </TransferStepCard>

    <!-- Target -->
    <TransferStepCard
      :title="t('transfer.migration.targetConnection')"
      :step-number="2"
      icon="i-carbon-data-refinery"
      icon-class="text-blue-600 dark:text-blue-500"
      :summary="targetSummary"
    >
      <ConnectionSelector
        v-model:connection-id="targetConnectionId"
        v-model:database="targetDatabase"
        v-model:schema="targetSchema"
        show-schema
      />

      <!-- Migration Direction -->
      <div v-if="sourceConnectionId && targetConnectionId" class="mb-1 mt-5 flex gap-4 items-center justify-center">
        <div class="px-4 py-3 border border-border/40 rounded-md bg-muted/20 flex flex-col gap-1 min-w-[140px] shadow-sm items-center justify-center">
          <Badge variant="outline" class="text-[10px] tracking-wide font-mono px-1.5 py-0 border-border/60 uppercase">
            {{ sourceEngine }}
          </Badge>
          <span class="text-xs font-medium font-mono">{{ sourceDatabase }}</span>
          <span class="text-[11px] text-muted-foreground font-mono tabular-nums">{{ selectedTables.length }} tables</span>
        </div>

        <span class="i-carbon-arrow-right text-xl text-muted-foreground/40" />

        <div class="px-4 py-3 border border-border/40 rounded-md bg-muted/20 flex flex-col gap-1 min-w-[140px] shadow-sm items-center justify-center">
          <Badge variant="outline" class="text-[10px] tracking-wide font-mono px-1.5 py-0 border-border/60 uppercase">
            {{ targetEngine }}
          </Badge>
          <span class="text-xs font-medium font-mono">{{ targetDatabase }}</span>
          <span class="text-[11px] text-muted-foreground font-mono">{{ targetSchema || 'default' }}</span>
        </div>
      </div>

      <!-- Options -->
      <div class="mt-5 pt-4 border-t border-border/40">
        <!-- Create database if not exists (database scope) -->
        <div v-if="scope === 'database'" class="mb-4">
          <label class="flex cursor-pointer items-center space-x-1.5">
            <Checkbox id="mig-opt-create-db" v-model:checked="createTargetDatabaseIfNotExists" class="h-3.5 w-3.5" />
            <span class="text-[11px] leading-none font-medium">Create target database if not exists</span>
          </label>
        </div>

        <div class="gap-4 grid grid-cols-1 md:grid-cols-2">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Batch Size</Label>
            <Input
              v-model.number="batchSize"
              type="number"
              min="100"
              max="10000"
              class="text-xs font-mono h-8 tabular-nums"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">On Error</Label>
            <Select v-model="onError">
              <SelectTrigger class="text-xs h-8">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="skipRow" class="text-xs">
                  Skip Row
                </SelectItem>
                <SelectItem value="skipTable" class="text-xs">
                  Skip Table
                </SelectItem>
                <SelectItem value="abort" class="text-xs">
                  Abort
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div class="mt-4 flex flex-wrap gap-4">
          <label class="flex cursor-pointer items-center space-x-1.5">
            <Checkbox id="mig-opt-create" v-model:checked="createTables" class="h-3.5 w-3.5" />
            <span class="text-[11px] leading-none font-medium">Create tables if not exist</span>
          </label>
          <label class="flex cursor-pointer items-center space-x-1.5">
            <Checkbox id="mig-opt-drop" v-model:checked="dropTables" class="h-3.5 w-3.5" />
            <span class="text-[11px] leading-none font-medium">Drop existing tables</span>
          </label>
        </div>
      </div>
    </TransferStepCard>

    <!-- Preview & Execute -->
    <TransferStepCard
      v-if="preview || result"
      title="Preview & Execute"
      :step-number="3"
      icon="i-carbon-play-filled-alt"
      icon-class="text-primary"
      variant="highlight"
    >
      <div v-if="preview">
        <div class="mb-3 flex flex-wrap gap-2">
          <Badge variant="outline" class="text-[10px] font-mono px-1.5 py-0 border-border/60 tabular-nums">
            {{ preview.tables.length }} tables
          </Badge>
          <Badge variant="outline" class="text-[10px] font-mono px-1.5 py-0 border-border/60 tabular-nums">
            {{ preview.totalRows.toLocaleString() }} total rows
          </Badge>
          <Badge variant="secondary" class="text-[10px] font-mono px-1.5 py-0 bg-muted tabular-nums">
            {{ preview.typeConversions }} type conversions
          </Badge>
        </div>

        <!-- Mappings Preview -->
        <div class="space-y-1.5">
          <div v-for="plan in tablePlans.slice(0, 3)" :key="plan.sourceTable" class="px-3 py-2 border border-border/40 rounded-md bg-muted/20 flex shadow-sm items-center justify-between">
            <span class="text-xs font-mono">{{ plan.sourceTable }}</span>
            <span class="text-[11px] text-muted-foreground font-mono tabular-nums">{{ plan.columnMappings.length }} columns mapped</span>
          </div>
          <div v-if="tablePlans.length > 3" class="text-[11px] text-muted-foreground font-mono p-1 text-center opacity-70 tabular-nums">
            + {{ tablePlans.length - 3 }} more tables
          </div>
        </div>
      </div>

      <!-- Result -->
      <div v-if="result" class="mt-4 pt-3 border-t border-border/40">
        <div v-if="result.success" class="text-xs text-green-600 font-medium px-3 py-2 border border-green-500/20 rounded-md bg-green-500/10 flex gap-2 shadow-sm items-center">
          <span class="i-carbon-checkmark-filled shrink-0 h-4 w-4" />
          <span class="font-mono">{{ result.message }}</span>
        </div>
        <div v-else class="text-xs text-destructive font-medium px-3 py-2 border border-destructive/20 rounded-md bg-destructive/10 flex gap-2 shadow-sm items-center">
          <span class="i-carbon-warning-filled shrink-0 h-4 w-4" />
          <span class="font-mono">{{ result.message }}</span>
        </div>
      </div>

      <!-- Execute -->
      <div class="mt-5 pt-4 border-t border-border/40 flex justify-end">
        <Button
          :disabled="!canExecute || executing"
          size="sm"
          class="h-8 min-w-[120px]"
          @click="executeMigration"
        >
          <span v-if="executing" class="i-carbon-circle-dash mr-1.5 animate-spin" />
          <span v-else class="i-carbon-play mr-1.5" />
          Start Migration
        </Button>
      </div>
    </TransferStepCard>
  </div>
</template>
