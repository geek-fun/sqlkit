<script setup lang="ts">
import type { ColumnMapping, ConflictStrategy, FileDetectionResult, TransferProgress, TransferResult } from '@/types/transfer'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

import { detectFile, executeImport, previewImport } from '@/datasources/transferApi'
import { useTransferStore } from '@/store/transferStore'

import ConnectionSelector from '../shared/ConnectionSelector.vue'
import TableSelector from '../shared/TableSelector.vue'
import TransferStepCard from '../shared/TransferStepCard.vue'
import FileDropZone from '../shared/FileDropZone.vue'
import ProgressPanel from '../shared/ProgressPanel.vue'
import ResultPanel from '../shared/ResultPanel.vue'

export type ColumnInfo = {
  name: string
  data_type?: string
  nullable?: boolean
  default_value?: string
  is_primary_key?: boolean
}

const { t } = useI18n()
const transferStore = useTransferStore()

// --- Step 1: File ---
const filePath = ref('')
const detectionResult = ref<FileDetectionResult | null>(null)
const isDetecting = ref(false)

const formatLabel = computed(() => {
  if (!detectionResult.value) return ''
  return detectionResult.value.format.toUpperCase()
})

const fileSummary = computed(() => {
  if (filePath.value) {
    const parts = filePath.value.split(/[\\/]/)
    return parts[parts.length - 1]
  }
  return ''
})

async function handleFileDrop(_file: File) {
  filePath.value = 'uploaded-file'
  await detectFileInfo()
}

async function handleFileBrowse() {
  const selected = await open({
    multiple: false,
    filters: [
      { name: 'Import File', extensions: ['csv', 'jsonl', 'sql', 'xlsx'] },
    ],
  })
  if (selected) {
    filePath.value = selected as string
    await detectFileInfo()
  }
}

async function detectFileInfo() {
  if (!filePath.value) return

  isDetecting.value = true
  try {
    const result = await detectFile(filePath.value)
    detectionResult.value = result

    transferStore.importRequest = {
      ...transferStore.importRequest,
      filePath: filePath.value,
      format: result.format,
      columnMappings: result.columns.map(col => ({
        sourceColumn: col,
        targetColumn: col,
      })),
    }
  } catch (error) {
    console.error('Detection failed:', error)
  } finally {
    isDetecting.value = false
  }
}

watch(filePath, () => {
  if (filePath.value) {
    detectFileInfo()
  }
})

// --- Step 2: Target & Mapping ---
const connectionId = ref('')
const database = ref('')
const schema = ref('')
const table = ref('')
const mappings = ref<ColumnMapping[]>([])
const targetColumns = ref<ColumnInfo[]>([])
const loadingColumns = ref(false)

const targetSummary = computed(() => {
  if (connectionId.value && table.value) {
    return `${table.value} (${mappings.value.filter(m => m.targetColumn).length} mapped)`
  }
  if (connectionId.value) return connectionId.value
  return ''
})

async function fetchTargetColumns() {
  if (!connectionId.value || !database.value || !table.value) return

  loadingColumns.value = true
  try {
    const result = await invoke<ColumnInfo[]>('list_columns', {
      connectionId: connectionId.value,
      database: database.value,
      schema: schema.value,
      table: table.value,
    })
    targetColumns.value = result || []
  } catch (error) {
    console.error('Failed to fetch columns:', error)
  } finally {
    loadingColumns.value = false
  }
}

function autoMap() {
  mappings.value = mappings.value.map((m) => {
    const match = targetColumns.value.find((tc: ColumnInfo) => tc.name.toLowerCase() === m.sourceColumn.toLowerCase())
    return {
      ...m,
      targetColumn: match?.name,
      targetType: match?.data_type,
    }
  })
}

function clearAll() {
  mappings.value = mappings.value.map(m => ({
    ...m,
    targetColumn: undefined,
  }))
}

const targetParams = computed(() => {
  return {
    connectionId: connectionId.value,
    database: database.value,
    schema: schema.value,
    table: table.value,
  }
})

watch(targetParams, (params, oldParams) => {
  transferStore.importRequest = {
    ...transferStore.importRequest,
    connectionId: params.connectionId || undefined,
    database: params.database || undefined,
    schema: params.schema || undefined,
    table: params.table,
    columnMappings: mappings.value,
  }

  if (params && JSON.stringify(params) !== JSON.stringify(oldParams)) {
    if (params.connectionId && params.database && params.table) {
      fetchTargetColumns()
    }
  }
}, { deep: true })

watch(table, () => {
  autoMap()
})

watch(() => transferStore.importRequest.columnMappings, (newMappings) => {
  if (newMappings && newMappings.length > 0) {
    mappings.value = newMappings
  }
}, { immediate: true })

// --- Step 3: Options & Execute ---
const conflictStrategy = ref<ConflictStrategy>('skip')
const batchSize = ref(5000)
const truncateBefore = ref(false)
const dryRun = ref(false)
const excelSheetName = ref('Sheet1')
const excelHasHeader = ref(true)

const previewData = ref<string[][]>([])
const previewColumns = ref<string[]>([])
const isLoadingPreview = ref(false)

const conflictOptions: { value: ConflictStrategy, label: string }[] = [
  { value: 'skip', label: 'Skip duplicates' },
  { value: 'replace', label: 'Replace existing' },
  { value: 'upsert', label: 'Update existing (upsert)' },
  { value: 'abort', label: 'Abort on error' },
]

async function loadPreview() {
  if (!transferStore.importRequest.filePath || !transferStore.importRequest.format) {
    return
  }

  isLoadingPreview.value = true
  try {
    const result = await previewImport(
      transferStore.importRequest.filePath,
      transferStore.importRequest.format,
      10,
    )
    previewColumns.value = result.columns
    previewData.value = result.sampleRows
  } catch (error) {
    console.error('Preview failed:', error)
  } finally {
    isLoadingPreview.value = false
  }
}

watch([conflictStrategy, batchSize, truncateBefore, dryRun, excelSheetName, excelHasHeader], () => {
  transferStore.importRequest = {
    ...transferStore.importRequest,
    conflictStrategy: conflictStrategy.value,
    batchSize: batchSize.value,
    truncateBefore: truncateBefore.value,
    dryRun: dryRun.value,
    excelOptions: detectionResult.value?.format === 'excel'
      ? { sheetName: excelSheetName.value, hasHeader: excelHasHeader.value }
      : undefined,
  }
})

watch(() => transferStore.importRequest.filePath, () => {
  loadPreview()
}, { immediate: true })

// --- Execute Logic ---
const isRunning = ref(false)
const progress = ref<TransferProgress | null>(null)
const result = ref<TransferResult | null>(null)

let unlistenProgress: (() => void) | null = null

const canImport = computed(() => {
  return filePath.value && connectionId.value && table.value && mappings.value.some(m => m.targetColumn)
})

async function startImport() {
  if (!transferStore.importRequest.filePath || !transferStore.importRequest.connectionId) {
    return
  }

  isRunning.value = true
  progress.value = null
  result.value = null
  transferStore.startOperation()

  try {
    const res = await executeImport(transferStore.importRequest as any)
    result.value = res
    transferStore.completeOperation(res)
  } catch (error) {
    result.value = {
      success: false,
      totalRows: 0,
      processedRows: 0,
      skippedRows: 0,
      errorCount: 1,
      durationMs: 0,
      errors: [{ message: String(error) }],
    }
  } finally {
    isRunning.value = false
  }
}

function handleRunInBackground() {
  const task = transferStore.createTask(
    'import',
    {
      connectionId: transferStore.importRequest.connectionId || '',
      table: transferStore.importRequest.table || '',
      filePath: transferStore.importRequest.filePath || '',
      format: transferStore.importRequest.format || 'csv',
      conflictStrategy: transferStore.importRequest.conflictStrategy,
    },
    progress.value?.totalRows || 0,
  )
  transferStore.addRunningTask(task)
  transferStore.detachActiveTask('import')
}

function handleCancel() {
  isRunning.value = false
}

function handleAgain() {
  result.value = null
  progress.value = null
  transferStore.resetImport()
}

onMounted(async () => {
  unlistenProgress = await listen<TransferProgress>('transfer-progress', (event) => {
    progress.value = event.payload
    if (transferStore.activeImportTaskId) {
      transferStore.syncProgressToTask(transferStore.activeImportTaskId, event.payload)
    }
  })
})

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress()
  }
})
</script>

<template>
  <div class="pb-6 flex flex-col gap-4">
    <!-- Step 1: File -->
    <TransferStepCard
      :title="t('transfer.import.step.file', 'Source File')"
      :step-number="1"
      icon="i-carbon-document-import"
      icon-class="text-emerald-600 dark:text-emerald-500"
      :summary="fileSummary"
    >
      <div class="space-y-4">
        <FileDropZone @file-selected="handleFileDrop" />

        <Button variant="outline" class="w-full" @click="handleFileBrowse">
          Browse Files
        </Button>

        <div v-if="detectionResult" class="mt-4 pt-4 border-t border-border/40 space-y-4">
          <div class="text-sm gap-4 grid grid-cols-1 sm:grid-cols-2">
            <div>
              <Label>Detected Format</Label>
              <Badge class="mt-1 block w-fit">
                {{ formatLabel }}
              </Badge>
            </div>
            <div>
              <Label>Encoding</Label>
              <div class="mt-1 text-muted-foreground">
                {{ detectionResult.encoding }}
              </div>
            </div>
            <div>
              <Label>Estimated Rows</Label>
              <div class="mt-1 text-muted-foreground">
                {{ detectionResult.estimatedRows?.toLocaleString() || 'Unknown' }}
              </div>
            </div>
            <div>
              <Label>File Size</Label>
              <div class="mt-1 text-muted-foreground">
                {{ (detectionResult.fileSizeBytes / 1024).toFixed(1) }} KB
              </div>
            </div>
          </div>

          <div>
            <Label>Detected Columns</Label>
            <div class="mt-2 flex flex-wrap gap-2">
              <Badge
                v-for="col in detectionResult.columns.slice(0, 10)"
                :key="col"
                variant="outline"
              >
                {{ col }}
              </Badge>
              <Badge v-if="detectionResult.columns.length > 10" variant="outline">
                +{{ detectionResult.columns.length - 10 }} more
              </Badge>
            </div>
          </div>
        </div>
      </div>
    </TransferStepCard>

    <!-- Step 2: Target & Mapping -->
    <TransferStepCard
      :title="t('transfer.import.step.mapping', 'Target & Mapping')"
      :step-number="2"
      icon="i-carbon-data-base"
      icon-class="text-blue-600 dark:text-blue-500"
      :summary="targetSummary"
    >
      <div class="space-y-4">
        <ConnectionSelector
          v-model:connection-id="connectionId"
          v-model:database="database"
          v-model:schema="schema"
          show-schema
        />

        <TableSelector
          v-model:table="table"
          :connection-id="connectionId"
          :database="database"
          :schema="schema"
        />

        <div v-if="table" class="mt-4 pt-4 border-t border-border/40 space-y-4">
          <div class="flex items-center justify-between">
            <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Column Mapping</Label>
            <div class="flex gap-2">
              <Button variant="outline" size="sm" @click="autoMap">
                Auto-Map
              </Button>
              <Button variant="outline" size="sm" @click="clearAll">
                Clear All
              </Button>
            </div>
          </div>

          <div v-if="loadingColumns" class="text-sm text-muted-foreground">
            Loading target columns...
          </div>

          <div v-else class="border border-border/50 rounded overflow-hidden">
            <table class="text-sm w-full">
              <caption class="sr-only">
                Column mapping between source and target database
              </caption>
              <thead class="border-b bg-muted/30">
                <tr>
                  <th scope="col" class="font-medium p-2 text-left">
                    Source Column
                  </th>
                  <th scope="col" class="font-medium p-2 text-left">
                    Target Column
                  </th>
                  <th scope="col" class="font-medium p-2 text-left">
                    Type
                  </th>
                  <th scope="col" class="font-medium p-2 text-left">
                    Status
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="mapping in mappings" :key="mapping.sourceColumn" class="border-b border-border/40 transition-colors hover:bg-muted/50">
                  <th scope="row" class="font-normal p-2 text-left align-middle">
                    {{ mapping.sourceColumn }}
                  </th>
                  <td class="p-2 align-middle">
                    <Select v-model="mapping.targetColumn">
                      <SelectTrigger
                        class="w-full h-8 text-xs"
                        :aria-label="`Target column for ${mapping.sourceColumn}`"
                      >
                        <SelectValue placeholder="Select column" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="">
                          (Skip)
                        </SelectItem>
                        <SelectItem
                          v-for="col in targetColumns"
                          :key="col.name"
                          :value="col.name"
                        >
                          {{ col.name }}
                        </SelectItem>
                      </SelectContent>
                    </Select>
                  </td>
                  <td class="text-muted-foreground p-2 text-xs align-middle">
                    {{ mapping.targetType || '-' }}
                  </td>
                  <td class="p-2 align-middle">
                    <span v-if="mapping.targetColumn" class="text-xs text-emerald-600 font-medium flex items-center dark:text-emerald-500">
                      <span class="sr-only">Status: </span>
                      <span aria-hidden="true" class="mr-1">✓</span> Mapped
                    </span>
                    <span v-else class="text-xs text-muted-foreground font-medium flex items-center">
                      <span class="sr-only">Status: </span>
                      <span aria-hidden="true" class="mr-1">⊘</span> Skipped
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="text-xs text-muted-foreground">
            {{ mappings.filter(m => m.targetColumn).length }} of {{ mappings.length }} columns mapped
          </div>
        </div>
      </div>
    </TransferStepCard>

    <!-- Step 3: Options & Execute -->
    <TransferStepCard
      :title="t('transfer.import.step.options', 'Options & Import')"
      :step-number="3"
      icon="i-carbon-settings"
      icon-class="text-amber-600 dark:text-amber-500"
      variant="highlight"
    >
      <div class="space-y-4">
        <!-- Result/Progress Panel overrides the options if we are executing -->
        <template v-if="isRunning || result">
          <ProgressPanel
            v-if="isRunning"
            :progress="progress"
            :is-running="isRunning"
            @cancel="handleCancel"
            @run-in-background="handleRunInBackground"
          />

          <ResultPanel
            v-if="result && !isRunning"
            :result="result"
            @again="handleAgain"
          />
        </template>
        
        <template v-else>
          <div class="gap-5 grid grid-cols-1 md:grid-cols-2">
            <div class="space-y-2.5">
              <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">On Conflict</Label>
              <Select v-model="conflictStrategy">
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem
                    v-for="opt in conflictOptions"
                    :key="opt.value"
                    :value="opt.value"
                  >
                    {{ opt.label }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div class="space-y-2.5">
              <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Batch Size</Label>
              <Input
                v-model.number="batchSize"
                type="number"
                min="1"
                max="100000"
              />
            </div>
          </div>

          <div class="mt-4 gap-4 grid grid-cols-1 sm:grid-cols-2">
            <div class="flex items-center space-x-2">
              <Checkbox id="import-opt-truncate" v-model:checked="truncateBefore" />
              <Label for="import-opt-truncate" class="text-sm leading-none font-medium cursor-pointer">Truncate table before import</Label>
            </div>

            <div class="flex items-center space-x-2">
              <Checkbox id="import-opt-dry-run" v-model:checked="dryRun" />
              <Label for="import-opt-dry-run" class="text-sm leading-none font-medium cursor-pointer">Dry run (validate without inserting)</Label>
            </div>
          </div>

          <!-- Excel Options -->
          <div v-if="detectionResult?.format === 'excel'" class="mt-4 pt-4 border-t border-border/40 gap-5 grid grid-cols-1 md:grid-cols-2">
            <div class="space-y-2.5">
              <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Sheet Name</Label>
              <Input v-model="excelSheetName" placeholder="Sheet1" />
            </div>
            <div class="flex items-center space-x-2 sm:mt-8">
              <Checkbox id="import-excel-header" v-model:checked="excelHasHeader" />
              <Label for="import-excel-header" class="text-sm leading-none font-medium cursor-pointer">First row is header</Label>
            </div>
          </div>

          <!-- Preview -->
          <div class="mt-4 pt-4 border-t border-border/40">
            <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase mb-2 block">Data Preview (first 10 rows)</Label>
            <div v-if="previewData.length > 0" class="mt-2 max-h-[200px] overflow-auto border border-border/50 rounded-md">
              <table class="text-xs w-full whitespace-nowrap">
                <thead class="bg-muted/30 sticky top-0 backdrop-blur-sm">
                  <tr>
                    <th v-for="col in previewColumns" :key="col" class="p-2 text-left border-b font-medium">
                      {{ col }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, i) in previewData" :key="i" class="border-b border-border/40 hover:bg-muted/10">
                    <td v-for="(val, j) in row" :key="j" class="p-2">
                      {{ val }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div v-else class="text-sm mt-2 p-6 text-center border rounded-md border-dashed bg-muted/10 flex flex-col items-center justify-center text-muted-foreground">
              <span v-if="isLoadingPreview" class="i-carbon-circle-dash animate-spin h-5 w-5 mb-2 opacity-50" />
              <span v-else class="i-carbon-data-view h-5 w-5 mb-2 opacity-50" />
              {{ isLoadingPreview ? 'Loading preview...' : 'Select a file to preview data' }}
            </div>
          </div>

          <!-- Execute Button -->
          <div class="mt-4 pt-4 flex justify-end">
            <Button :disabled="!canImport" class="min-w-[120px]" @click="startImport">
              <span class="i-carbon-document-import mr-2" /> Import Data
            </Button>
          </div>
        </template>
      </div>
    </TransferStepCard>
  </div>
</template>
