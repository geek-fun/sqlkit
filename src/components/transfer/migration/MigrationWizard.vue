<script setup lang="ts">
import type { MigrationMapping, MigrationPreview, MigrationRequest, MigrationTablePlan } from '@/types/transfer'

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
  if (sourceConnectionId.value && selectedTables.value.length)
    return `${selectedTables.value.length} tables`
  return ''
})

const targetSummary = computed(() => {
  if (targetConnectionId.value)
    return `${targetDatabase.value || 'default'}`
  return ''
})

// Load tables
async function loadTables() {
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

const canExecute = computed(() =>
  isSourceConnected.value
  && isTargetConnected.value
  && selectedTables.value.length > 0
  && tablePlans.value.length > 0,
)
</script>

<template>
  <div class="pb-6 flex flex-col gap-4">
    <!-- Source -->
    <TransferStepCard
      :title="t('pages.transfer.migration.sourceConnection')"
      :step-number="1"
      icon="i-carbon-data-base"
      icon-class="text-emerald-600 dark:text-emerald-500"
      :summary="sourceSummary"
    >
      <ConnectionSelector
        v-model:connection-id="sourceConnectionId"
        v-model:database="sourceDatabase"
        v-model:schema="sourceSchema"
        show-schema
      />

      <!-- Tables -->
      <div class="mt-4 pt-4 border-t border-border/40">
        <div class="mb-4 flex items-center justify-between">
          <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Tables</Label>
          <div class="flex gap-2 items-center">
            <Button variant="ghost" size="sm" class="text-xs h-8" @click="selectAllTables">
              Select All
            </Button>
            <Button variant="ghost" size="sm" class="text-xs h-8" @click="deselectAllTables">
              Deselect All
            </Button>
          </div>
        </div>

        <div v-if="loadingTables" class="text-sm text-muted-foreground p-8 border rounded-md border-dashed flex items-center justify-center">
          <span class="i-carbon-circle-dash mr-2 animate-spin" /> Loading tables...
        </div>

        <div v-else-if="availableTables.length === 0 && sourceDatabase" class="text-sm text-muted-foreground p-8 text-center border rounded-md border-dashed bg-muted/10 flex flex-col items-center justify-center">
          <span class="i-carbon-data-base mb-2 opacity-50 h-6 w-6" />
          No tables found
        </div>

        <div v-else class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border pr-2 gap-3 grid grid-cols-1 max-h-[300px] overflow-y-auto md:grid-cols-3 sm:grid-cols-2">
          <label
            v-for="table in availableTables"
            :key="table.name"
            class="p-3 border rounded-md flex cursor-pointer transition-colors items-center space-x-3 hover:bg-muted/50"
            :class="selectedTables.includes(table.name) ? 'border-primary/50 bg-primary/5' : 'border-border bg-transparent'"
          >
            <Checkbox
              :id="`mig-table-${table.name}`"
              :checked="selectedTables.includes(table.name)"
              @update:checked="toggleTable(table.name)"
            />
            <div class="flex flex-col">
              <span class="text-sm leading-none font-medium">{{ table.name }}</span>
              <span v-if="table.rowCount" class="text-[10px] text-muted-foreground tracking-wider mt-1 uppercase">{{ table.rowCount.toLocaleString() }} rows</span>
            </div>
          </label>
        </div>

        <div v-if="availableTables.length > 0" class="text-xs text-muted-foreground mt-3">
          {{ selectedTables.length }} of {{ availableTables.length }} tables selected
        </div>
      </div>
    </TransferStepCard>

    <!-- Target -->
    <TransferStepCard
      :title="t('pages.transfer.migration.targetConnection')"
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
      <div v-if="sourceConnectionId && targetConnectionId" class="mt-4 p-6 border rounded-lg border-dashed bg-muted/10 flex gap-6 items-center justify-center">
        <div class="p-4 border rounded-md bg-card flex flex-col gap-2 shadow-sm items-center justify-center">
          <Badge variant="outline" class="text-[10px] tracking-wider px-1.5 py-0 uppercase">
            {{ sourceEngine }}
          </Badge>
          <span class="text-sm font-medium">{{ sourceDatabase }}</span>
          <span class="text-xs text-muted-foreground">{{ selectedTables.length }} tables selected</span>
        </div>
        <span class="i-carbon-arrow-right text-2xl text-muted-foreground opacity-50" />
        <div class="p-4 border rounded-md bg-card flex flex-col gap-2 shadow-sm items-center justify-center">
          <Badge variant="outline" class="text-[10px] tracking-wider px-1.5 py-0 uppercase">
            {{ targetEngine }}
          </Badge>
          <span class="text-sm font-medium">{{ targetDatabase }}</span>
          <span class="text-xs text-muted-foreground">{{ targetSchema || 'default schema' }}</span>
        </div>
      </div>

      <!-- Options -->
      <div class="mt-4 pt-4 border-t border-border/40">
        <div class="gap-5 grid grid-cols-1 md:grid-cols-2">
          <div class="space-y-2.5">
            <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Batch Size</Label>
            <Input
              v-model.number="batchSize"
              type="number"
              min="100"
              max="10000"
            />
          </div>
          <div class="space-y-2.5">
            <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">On Error</Label>
            <Select v-model="onError">
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="skipRow">
                  Skip Row
                </SelectItem>
                <SelectItem value="skipTable">
                  Skip Table
                </SelectItem>
                <SelectItem value="abort">
                  Abort
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div class="mt-3 gap-4 grid grid-cols-1 sm:grid-cols-2">
          <label class="flex cursor-pointer items-center space-x-2">
            <Checkbox id="mig-opt-create" v-model:checked="createTables" />
            <span class="text-sm leading-none font-medium">Create tables if not exist</span>
          </label>
          <label class="flex cursor-pointer items-center space-x-2">
            <Checkbox id="mig-opt-drop" v-model:checked="dropTables" />
            <span class="text-sm leading-none font-medium">Drop existing tables</span>
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
        <div class="mb-4 flex flex-wrap gap-2">
          <Badge variant="outline" class="text-xs">
            {{ preview.tables.length }} tables
          </Badge>
          <Badge variant="outline" class="text-xs">
            {{ preview.totalRows.toLocaleString() }} total rows
          </Badge>
          <Badge variant="secondary" class="text-xs">
            {{ preview.typeConversions }} type conversions needed
          </Badge>
        </div>

        <!-- Mappings Preview -->
        <div class="space-y-2">
          <div v-for="plan in tablePlans.slice(0, 3)" :key="plan.sourceTable" class="p-3 border rounded-md bg-card flex shadow-sm items-center justify-between">
            <span class="text-sm font-medium">{{ plan.sourceTable }}</span>
            <span class="text-xs text-muted-foreground">{{ plan.columnMappings.length }} columns mapped</span>
          </div>
          <div v-if="tablePlans.length > 3" class="text-xs text-muted-foreground p-2 text-center italic">
            And {{ tablePlans.length - 3 }} more tables...
          </div>
        </div>
      </div>

      <!-- Result -->
      <div v-if="result" class="mt-3 pt-4 border-t border-border/40">
        <div v-if="result.success" class="text-sm text-emerald-600 font-medium flex gap-2 items-center dark:text-emerald-500">
          <span class="i-carbon-checkmark-filled h-5 w-5" />
          {{ result.message }}
        </div>
        <div v-else class="text-sm text-destructive font-medium flex gap-2 items-center">
          <span class="i-carbon-warning-filled h-5 w-5" />
          {{ result.message }}
        </div>
      </div>

      <!-- Execute -->
      <div class="mt-4 pt-4 border-t border-border/40 flex justify-end">
        <Button
          :disabled="!canExecute || executing"
          class="min-w-[120px]"
          @click="executeMigration"
        >
          <span v-if="executing" class="i-carbon-circle-dash mr-2 animate-spin" />
          <span v-else class="i-carbon-play mr-2" />
          Run Migration
        </Button>
      </div>
    </TransferStepCard>
  </div>
</template>
