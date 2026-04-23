<script setup lang="ts">
import type { ColumnMapping, ConflictStrategy, FileDetectionResult, TransferProgress, TransferResult } from '@/types/transfer'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
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
import FileDropZone from '../shared/FileDropZone.vue'
import ProgressPanel from '../shared/ProgressPanel.vue'
import ResultPanel from '../shared/ResultPanel.vue'
import TableSelector from '../shared/TableSelector.vue'
import TransferStepCard from '../shared/TransferStepCard.vue'

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
  if (!detectionResult.value)
    return ''
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
  if (!filePath.value)
    return

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
  }
  catch (error) {
    console.error('Detection failed:', error)
  }
  finally {
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
  if (connectionId.value)
    return connectionId.value
  return ''
})

async function fetchTargetColumns() {
  if (!connectionId.value || !database.value || !table.value)
    return

  loadingColumns.value = true
  try {
    const result = await invoke<ColumnInfo[]>('list_columns', {
      connectionId: connectionId.value,
      database: database.value,
      schema: schema.value || null,
      tableName: table.value,
    })
    targetColumns.value = result || []
  }
  catch (error) {
    console.error('Failed to fetch columns:', error)
  }
  finally {
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
  }
  catch (error) {
    console.error('Preview failed:', error)
  }
  finally {
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
  }
  catch (error) {
    result.value = {
      success: false,
      totalRows: 0,
      processedRows: 0,
      skippedRows: 0,
      errorCount: 1,
      durationMs: 0,
      errors: [{ message: String(error) }],
    }
  }
  finally {
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
  <div class="pb-6 flex flex-col gap-2.5">
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

        <div class="flex justify-end">
          <Button variant="outline" size="sm" @click="handleFileBrowse">
            <div class="i-carbon-folder-open mr-2" />
            Browse Files
          </Button>
        </div>

        <div v-if="detectionResult" class="mt-4 pt-3 border-t border-border/40 space-y-3">
          <div class="gap-2.5 grid grid-cols-2">
            <div>
              <Label class="text-[11px] text-muted-foreground tracking-wide mb-1 block uppercase">Detected Format</Label>
              <Badge class="text-[10px] font-mono px-1 py-0.5 w-fit block uppercase" variant="secondary">
                {{ formatLabel }}
              </Badge>
            </div>
            <div>
              <Label class="text-[11px] text-muted-foreground tracking-wide mb-1 block uppercase">Encoding</Label>
              <Badge class="text-[10px] font-mono px-1 py-0.5 w-fit block uppercase" variant="outline">
                {{ detectionResult.encoding }}
              </Badge>
            </div>
            <div>
              <Label class="text-[11px] text-muted-foreground tracking-wide mb-1 block uppercase">Estimated Rows</Label>
              <div class="text-xs font-mono tabular-nums">
                {{ detectionResult.estimatedRows?.toLocaleString() || 'Unknown' }}
              </div>
            </div>
            <div>
              <Label class="text-[11px] text-muted-foreground tracking-wide mb-1 block uppercase">File Size</Label>
              <div class="text-xs font-mono tabular-nums">
                {{ (detectionResult.fileSizeBytes / 1024).toFixed(1) }} KB
              </div>
            </div>
          </div>

          <div class="pt-2 border-t border-border/40">
            <Label class="text-[11px] text-muted-foreground tracking-wide mb-2 block uppercase">Detected Columns</Label>
            <div class="flex flex-wrap gap-1.5">
              <Badge
                v-for="col in detectionResult.columns.slice(0, 10)"
                :key="col"
                variant="outline"
                class="text-[10px] font-mono px-1.5 py-0.5 border-border/40"
              >
                {{ col }}
              </Badge>
              <Badge v-if="detectionResult.columns.length > 10" variant="outline" class="text-[10px] font-mono px-1.5 py-0.5 border-dashed">
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
        <div class="gap-2.5 grid grid-cols-2">
          <div class="col-span-2">
            <ConnectionSelector
              v-model:connection-id="connectionId"
              v-model:database="database"
              v-model:schema="schema"
              show-schema
            />
          </div>
          <div class="col-span-2">
            <TableSelector
              v-model:table="table"
              :connection-id="connectionId"
              :database="database"
              :schema="schema"
            />
          </div>
        </div>

        <div v-if="table" class="mt-4 pt-4 border-t border-border/40 space-y-2">
          <div class="flex items-center justify-between">
            <div class="text-xs tracking-wide font-semibold flex gap-2 items-center">
              <div class="i-carbon-flow-data" />
              COLUMN MAPPING
            </div>
            <div class="flex gap-2">
              <Button variant="outline" size="sm" class="text-[11px] px-2 h-6" @click="autoMap">
                Auto-Map
              </Button>
              <Button variant="ghost" size="sm" class="text-[11px] px-2 h-6" @click="clearAll">
                Clear All
              </Button>
            </div>
          </div>

          <div v-if="loadingColumns" class="text-[11px] text-muted-foreground py-2 text-center border border-border/40 rounded-sm border-dashed">
            Loading target columns...
          </div>

          <div v-else class="border border-border/40 rounded-sm overflow-hidden">
            <table class="text-xs w-full">
              <caption class="sr-only">
                Column mapping between source and target database
              </caption>
              <thead class="text-[10px] text-muted-foreground tracking-wide border-b border-border/40 bg-muted/40 uppercase">
                <tr>
                  <th scope="col" class="font-medium px-2 py-1.5 text-left">
                    Source Column
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-left w-[40%]">
                    Target Column
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-left">
                    Type
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-left w-20">
                    Status
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr v-if="mappings.length === 0" class="border-b border-border/40">
                  <td colspan="4" class="text-[11px] text-muted-foreground px-2 py-4 text-center italic">
                    No columns detected
                  </td>
                </tr>
                <tr v-for="mapping in mappings" :key="mapping.sourceColumn" class="border-b border-border/40 transition-colors last:border-0 hover:bg-muted/40">
                  <th scope="row" class="font-mono font-normal px-2 py-1 text-left align-middle max-w-[120px] truncate" :title="mapping.sourceColumn">
                    {{ mapping.sourceColumn }}
                  </th>
                  <td class="px-2 py-1 align-middle">
                    <Select v-model="mapping.targetColumn">
                      <SelectTrigger
                        class="text-xs font-mono h-7 w-full"
                        :aria-label="`Target column for ${mapping.sourceColumn}`"
                      >
                        <SelectValue placeholder="Skip" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="" class="text-xs text-muted-foreground italic">
                          (Skip)
                        </SelectItem>
                        <SelectItem
                          v-for="col in targetColumns"
                          :key="col.name"
                          :value="col.name"
                          class="text-xs font-mono"
                        >
                          {{ col.name }}
                        </SelectItem>
                      </SelectContent>
                    </Select>
                  </td>
                  <td class="text-[10px] text-muted-foreground font-mono px-2 py-1 align-middle max-w-[80px] truncate uppercase" :title="mapping.targetType">
                    {{ mapping.targetType || '-' }}
                  </td>
                  <td class="px-2 py-1 align-middle">
                    <span v-if="mapping.targetColumn" class="text-[10px] text-green-600 tracking-wide font-medium px-1.5 py-0.5 rounded-sm bg-green-500/10 flex w-fit uppercase items-center">
                      <span class="sr-only">Status: </span>
                      <div class="i-carbon-checkmark mr-1" aria-hidden="true" /> Mapped
                    </span>
                    <span v-else class="text-[10px] text-muted-foreground tracking-wide font-medium px-1.5 py-0.5 rounded-sm bg-muted flex w-fit uppercase items-center">
                      <span class="sr-only">Status: </span>
                      <div class="i-carbon-subtract mr-1" aria-hidden="true" /> Skipped
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="text-[11px] text-muted-foreground tracking-wide mt-2 flex uppercase justify-end">
            <span class="font-mono mr-1 tabular-nums">{{ mappings.filter(m => m.targetColumn).length }}</span> of
            <span class="font-mono mx-1 tabular-nums">{{ mappings.length }}</span> columns mapped
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
          <div class="gap-4 grid grid-cols-2">
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground tracking-wide block uppercase">On Conflict</Label>
              <Select v-model="conflictStrategy">
                <SelectTrigger class="text-xs h-8">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem
                    v-for="opt in conflictOptions"
                    :key="opt.value"
                    :value="opt.value"
                    class="text-xs"
                  >
                    {{ opt.label }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground tracking-wide block uppercase">Batch Size</Label>
              <Input
                v-model.number="batchSize"
                type="number"
                min="1"
                max="100000"
                class="text-xs font-mono h-8 tabular-nums"
              />
            </div>
          </div>

          <div class="mt-2 space-y-0.5">
            <div class="px-2 py-1.5 rounded-sm flex transition-colors items-center space-x-2 hover:bg-muted/40">
              <Checkbox id="import-opt-truncate" v-model:checked="truncateBefore" class="h-3.5 w-3.5" />
              <Label for="import-opt-truncate" class="text-xs cursor-pointer select-none">Truncate table before import</Label>
            </div>

            <div class="px-2 py-1.5 rounded-sm flex transition-colors items-center space-x-2 hover:bg-muted/40">
              <Checkbox id="import-opt-dry-run" v-model:checked="dryRun" class="h-3.5 w-3.5" />
              <Label for="import-opt-dry-run" class="text-xs cursor-pointer select-none">Dry run (validate without inserting)</Label>
            </div>
          </div>

          <!-- Excel Options -->
          <div v-if="detectionResult?.format === 'excel'" class="mt-4 pt-3 border-t border-border/40 gap-4 grid grid-cols-2">
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground tracking-wide block uppercase">Sheet Name</Label>
              <Input v-model="excelSheetName" placeholder="Sheet1" class="text-xs h-8" />
            </div>
            <div class="px-2 py-1.5 rounded-sm flex h-fit transition-colors items-center space-x-2 sm:mt-5 hover:bg-muted/40">
              <Checkbox id="import-excel-header" v-model:checked="excelHasHeader" class="h-3.5 w-3.5" />
              <Label for="import-excel-header" class="text-xs cursor-pointer select-none">First row is header</Label>
            </div>
          </div>

          <!-- Preview -->
          <div class="mt-4 pt-3 border-t border-border/40">
            <div class="mb-2 flex items-center justify-between">
              <div class="text-xs tracking-wide font-semibold flex gap-2 items-center">
                <div class="i-carbon-table" />
                DATA PREVIEW
              </div>
              <span class="text-[10px] text-muted-foreground tracking-wide font-mono uppercase">10 Rows</span>
            </div>
            <div class="border border-border/40 rounded-sm overflow-hidden">
              <div v-if="previewData.length > 0" class="max-h-[240px] overflow-auto">
                <table class="text-[10px] text-left w-full whitespace-nowrap border-collapse">
                  <thead class="bg-muted/40 shadow-border/40 shadow-sm top-0 sticky z-10">
                    <tr>
                      <th v-for="col in previewColumns" :key="col" class="text-muted-foreground tracking-wide font-medium px-2 py-1.5 border-b border-border/40 uppercase">
                        {{ col }}
                      </th>
                    </tr>
                  </thead>
                  <tbody class="font-mono">
                    <tr v-for="(row, i) in previewData" :key="i" class="border-b border-border/40 transition-colors last:border-0 hover:bg-muted/40">
                      <td v-for="(val, j) in row" :key="j" class="px-2 py-1.5 max-w-[150px] truncate" :title="val">
                        {{ val }}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="text-[11px] text-muted-foreground py-6 text-center bg-muted/20">
                <div v-if="isLoadingPreview" class="flex flex-col gap-2 items-center justify-center">
                  <div class="i-carbon-circle-dash opacity-50 animate-spin" />
                  Loading preview...
                </div>
                <span v-else>Select a file to preview data</span>
              </div>
            </div>
          </div>

          <!-- Execute Button -->
          <div class="mt-4 pt-3 border-t border-border/40 flex justify-end">
            <Button :disabled="!canImport" size="sm" class="min-w-[120px]" @click="startImport">
              <div class="i-carbon-document-import mr-2" /> Import Data
            </Button>
          </div>
        </template>
      </div>
    </TransferStepCard>
  </div>
</template>
