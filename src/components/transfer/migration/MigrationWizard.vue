<script setup lang="ts">
import type { MigrationMapping, MigrationPreview, MigrationRequest, MigrationTablePlan } from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

import { useConnectionStore } from '@/store/connectionStore'
import ConnectionSelector from '../shared/ConnectionSelector.vue'

import WizardStepper from '../shared/WizardStepper.vue'

const { t } = useI18n()
const connectionStore = useConnectionStore()

const steps = computed(() => [
  t('pages.transfer.migration.step.source'),
  t('pages.transfer.migration.step.target'),
  t('pages.transfer.migration.step.mapping'),
  t('pages.transfer.migration.step.configure'),
  t('pages.transfer.migration.step.execute'),
])

const currentStep = ref(0)

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
const migrateIndexes = ref(false)
const migrateForeignKeys = ref(false)
const migrateConstraints = ref(true)
const batchSize = ref(5000)
const onError = ref<'skipRow' | 'skipTable' | 'abort'>('skipRow')

const canGoBack = computed(() => currentStep.value > 0 && !executing.value)
const canGoNext = computed(() => {
  if (currentStep.value === 0)
    return sourceConnectionId.value !== '' && selectedTables.value.length > 0
  if (currentStep.value === 1)
    return targetConnectionId.value !== ''
  if (currentStep.value === 2)
    return tablePlans.value.length > 0
  if (currentStep.value === 3)
    return true
  return false
})

const sourceConnections = computed(() => connectionStore.connections)
const targetConnections = computed(() => connectionStore.connections)

const sourceEngine = computed(() => {
  const conn = sourceConnections.value.find(c => c.id === sourceConnectionId.value)
  return conn?.type || ''
})

const targetEngine = computed(() => {
  const conn = targetConnections.value.find(c => c.id === targetConnectionId.value)
  return conn?.type || ''
})

function handleBack() {
  if (canGoBack.value)
    currentStep.value--
}

async function handleNext() {
  if (!canGoNext.value)
    return

  if (currentStep.value === 0 && sourceConnectionId.value && sourceDatabase.value) {
    await loadTables()
  }

  if (currentStep.value === 1 && targetConnectionId.value) {
    await generateMappings()
  }

  if (currentStep.value === 2) {
    await previewMigration()
  }

  if (currentStep.value < 4)
    currentStep.value++
}

async function loadTables() {
  if (!sourceConnectionId.value || !sourceDatabase.value)
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
        ? `Migration completed: ${res.processedRows} rows migrated in ${res.durationMs}ms`
        : `Migration completed with ${res.errorCount} errors`,
    }
  }
  catch (error) {
    result.value = { success: false, message: String(error) }
  }
  finally {
    executing.value = false
  }
}

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
    migrateIndexes: migrateIndexes.value,
    migrateForeignKeys: migrateForeignKeys.value,
    migrateConstraints: migrateConstraints.value,
  }
}

const isTableSelected = (name: string) => selectedTables.value.includes(name)

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

function reset() {
  currentStep.value = 0
  sourceConnectionId.value = ''
  sourceDatabase.value = ''
  sourceSchema.value = ''
  targetConnectionId.value = ''
  targetDatabase.value = ''
  targetSchema.value = ''
  selectedTables.value = []
  tablePlans.value = []
  preview.value = null
  result.value = null
}
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>{{ t('pages.transfer.migration.title') }}</CardTitle>
    </CardHeader>
    <CardContent class="pt-6">
      <WizardStepper :steps="steps" :current-step="currentStep" />

      <div class="mt-6 min-h-400px">
        <!-- Step 1: Source -->
        <div v-if="currentStep === 0" class="space-y-6">
          <div>
            <Label>{{ t('pages.transfer.migration.sourceConnection') }}</Label>
            <ConnectionSelector
              v-model:connection-id="sourceConnectionId"
              v-model:database="sourceDatabase"
              v-model:schema="sourceSchema"
              show-schema
            />
          </div>

          <div class="space-y-4">
            <div class="flex items-center justify-between">
              <Label>{{ t('pages.transfer.migration.availableTables') }}</Label>
              <div class="flex gap-2">
                <Button variant="outline" size="sm" @click="selectAllTables">
                  {{ t('pages.transfer.migration.selectAll') }}
                </Button>
                <Button variant="outline" size="sm" @click="deselectAllTables">
                  {{ t('pages.transfer.migration.deselectAll') }}
                </Button>
              </div>
            </div>

            <div v-if="loadingTables" class="text-sm text-muted-foreground">
              {{ t('pages.transfer.migration.loadingTables') }}
            </div>

            <div v-else-if="availableTables.length === 0 && sourceDatabase" class="text-sm text-muted-foreground">
              {{ t('pages.transfer.migration.noTables') }}
            </div>

            <div v-else class="border rounded max-h-300px overflow-auto">
              <div
                v-for="table in availableTables"
                :key="table.name"
                class="p-2 border-b flex cursor-pointer items-center space-x-2 hover:bg-secondary/50"
                :class="isTableSelected(table.name) ? 'bg-secondary' : ''"
                @click="toggleTable(table.name)"
              >
                <Checkbox :checked="isTableSelected(table.name)" />
                <span class="text-sm">{{ table.name }}</span>
                <span v-if="table.rowCount" class="text-xs text-muted-foreground">
                  {{ table.rowCount }} rows
                </span>
              </div>
            </div>

            <div class="text-sm text-muted-foreground">
              {{ selectedTables.length }} {{ t('pages.transfer.migration.tablesSelected') }}
            </div>
          </div>
        </div>

        <!-- Step 2: Target -->
        <div v-if="currentStep === 1" class="space-y-6">
          <div>
            <Label>{{ t('pages.transfer.migration.targetConnection') }}</Label>
            <ConnectionSelector
              v-model:connection-id="targetConnectionId"
              v-model:database="targetDatabase"
              v-model:schema="targetSchema"
              show-schema
            />
          </div>

          <div class="p-4 border rounded bg-muted/30">
            <div class="text-sm text-muted-foreground mb-2">
              {{ t('pages.transfer.migration.direction') }}
            </div>
            <div class="flex gap-4 items-center justify-center">
              <div class="p-3 text-center border rounded">
                <div class="font-medium">
                  {{ sourceEngine }}
                </div>
                <div class="text-xs text-muted-foreground">
                  {{ sourceDatabase }}
                </div>
                <div class="text-xs">
                  {{ selectedTables.length }} tables
                </div>
              </div>
              <div class="text-2xl">
                →
              </div>
              <div class="p-3 text-center border rounded">
                <div class="font-medium">
                  {{ targetEngine }}
                </div>
                <div class="text-xs text-muted-foreground">
                  {{ targetDatabase }}
                </div>
                <div class="text-xs">
                  {{ targetSchema || 'default' }}
                </div>
              </div>
            </div>
          </div>

          <div class="space-y-4">
            <div class="flex items-center space-x-2">
              <Checkbox v-model:checked="createTables" />
              <Label class="text-sm">{{ t('pages.transfer.migration.createTables') }}</Label>
            </div>
            <div class="flex items-center space-x-2">
              <Checkbox v-model:checked="dropTables" />
              <Label class="text-sm">{{ t('pages.transfer.migration.dropTables') }}</Label>
            </div>
          </div>
        </div>

        <!-- Step 3: Mapping -->
        <div v-if="currentStep === 2" class="space-y-6">
          <div v-if="loadingPreview" class="text-sm text-muted-foreground">
            {{ t('pages.transfer.migration.loadingMapping') }}
          </div>

          <div v-else class="space-y-4">
            <div v-for="plan in tablePlans" :key="plan.sourceTable" class="p-4 border rounded">
              <div class="font-medium mb-2">
                {{ plan.sourceTable }}
              </div>
              <div class="text-xs text-muted-foreground mb-3">
                {{ plan.columnMappings.length }} columns
              </div>
              <div class="max-h-200px overflow-auto">
                <table class="text-sm w-full">
                  <thead class="border-b">
                    <tr>
                      <th class="p-2 text-left">
                        Source
                      </th>
                      <th class="p-2 text-left">
                        Type
                      </th>
                      <th class="p-2 text-left">
                        →
                      </th>
                      <th class="p-2 text-left">
                        Target
                      </th>
                      <th class="p-2 text-left">
                        Type
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="m in plan.columnMappings" :key="m.sourceColumn" class="border-b">
                      <td class="p-2">
                        {{ m.sourceColumn }}
                      </td>
                      <td class="text-muted-foreground p-2">
                        {{ m.sourceType }}
                      </td>
                      <td class="p-2">
                        →
                      </td>
                      <td class="p-2">
                        {{ m.targetColumn }}
                      </td>
                      <td class="text-muted-foreground p-2">
                        {{ m.targetType }}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <div v-if="preview" class="text-sm text-muted-foreground">
              {{ preview.typeConversions }} {{ t('pages.transfer.migration.typeConversions') }}
            </div>
          </div>
        </div>

        <!-- Step 4: Configure -->
        <div v-if="currentStep === 3" class="space-y-6">
          <div class="space-y-4">
            <div>
              <Label>{{ t('pages.transfer.migration.batchSize') }}</Label>
              <input
                v-model.number="batchSize"
                type="number"
                class="px-3 py-2 border rounded w-full"
                min="100"
                max="10000"
              >
            </div>

            <div>
              <Label>{{ t('pages.transfer.migration.onError') }}</Label>
              <Select v-model="onError">
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="skipRow">
                    {{ t('pages.transfer.migration.onErrorSkipRow') }}
                  </SelectItem>
                  <SelectItem value="skipTable">
                    {{ t('pages.transfer.migration.onErrorSkipTable') }}
                  </SelectItem>
                  <SelectItem value="abort">
                    {{ t('pages.transfer.migration.onErrorAbort') }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div class="gap-4 grid grid-cols-2">
              <div class="flex items-center space-x-2">
                <Checkbox v-model:checked="migrateIndexes" />
                <Label class="text-sm">{{ t('pages.transfer.migration.migrateIndexes') }}</Label>
              </div>
              <div class="flex items-center space-x-2">
                <Checkbox v-model:checked="migrateForeignKeys" />
                <Label class="text-sm">{{ t('pages.transfer.migration.migrateForeignKeys') }}</Label>
              </div>
              <div class="flex items-center space-x-2">
                <Checkbox v-model:checked="migrateConstraints" />
                <Label class="text-sm">{{ t('pages.transfer.migration.migrateConstraints') }}</Label>
              </div>
            </div>
          </div>

          <div v-if="preview" class="p-4 border rounded bg-muted/30">
            <div class="font-medium mb-2">
              {{ t('pages.transfer.migration.summary') }}
            </div>
            <div class="text-sm space-y-1">
              <div>{{ t('pages.transfer.migration.tables') }}: {{ preview.tables.length }}</div>
              <div>{{ t('pages.transfer.migration.totalRows') }}: {{ preview.totalRows.toLocaleString() }}</div>
              <div>{{ t('pages.transfer.migration.conversions') }}: {{ preview.typeConversions }}</div>
            </div>
          </div>
        </div>

        <!-- Step 5: Execute -->
        <div v-if="currentStep === 4" class="space-y-6">
          <div v-if="executing" class="text-center">
            <div class="text-lg">
              {{ t('pages.transfer.migration.executing') }}
            </div>
            <div class="text-sm text-muted-foreground mt-2">
              {{ t('pages.transfer.migration.pleaseWait') }}
            </div>
          </div>

          <div v-else-if="result" class="text-center">
            <div v-if="result.success" class="text-lg text-green-600">
              ✓ {{ result.message }}
            </div>
            <div v-else class="text-lg text-red-600">
              ✗ {{ result.message }}
            </div>
            <Button variant="outline" class="mt-4" @click="reset">
              {{ t('pages.transfer.migration.migrateAgain') }}
            </Button>
          </div>

          <div v-else class="text-center">
            <Button :disabled="tablePlans.length === 0" @click="executeMigration">
              {{ t('pages.transfer.migration.startMigration') }}
            </Button>
          </div>
        </div>
      </div>

      <div v-if="currentStep < 4" class="mt-6 flex gap-2 justify-end">
        <Button variant="outline" :disabled="!canGoBack" @click="handleBack">
          {{ t('pages.transfer.migration.back') }}
        </Button>
        <Button :disabled="!canGoNext" @click="handleNext">
          {{ t('pages.transfer.migration.next') }}
        </Button>
      </div>
    </CardContent>
  </Card>
</template>
