<script setup lang="ts">
import type { ExportFormat } from '@/types/transfer'

import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'

import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { previewExport } from '@/datasources/transferApi'
import { useTransferStore } from '@/store/transferStore'
import ColumnSelector from '../shared/ColumnSelector.vue'
import ConnectionSelector from '../shared/ConnectionSelector.vue'
import TableSelector from '../shared/TableSelector.vue'
import TransferStepCard from '../shared/TransferStepCard.vue'

const { t } = useI18n()
const transferStore = useTransferStore()

// Local state for all inputs (synced to store)
const connectionId = ref('')
const database = ref('')
const schema = ref('')
const table = ref('')
const columns = ref<string[]>([])
const whereClause = ref('')
const orderBy = ref('')
const limit = ref<number | undefined>()
const selectedFormat = ref<ExportFormat>('csv')
const csvDelimiter = ref(',')
const csvIncludeHeader = ref(true)
const excelSheetName = ref('Sheet1')
const excelIncludeHeader = ref(true)
const excelFreezeHeader = ref(true)
const excelAutoFit = ref(true)
const sqlIncludeCreateTable = ref(true)
const sqlIncludeDropTable = ref(false)
const sqlBatchSize = ref(1000)
const outputPath = ref('')
const previewData = ref('')
const estimatedRows = ref<number | undefined>()
const isLoadingPreview = ref(false)

// Sync local state to store
watch([
  connectionId,
  database,
  schema,
  table,
  columns,
  whereClause,
  orderBy,
  limit,
  selectedFormat,
  outputPath,
], () => {
  transferStore.exportRequest = {
    connectionId: connectionId.value || undefined,
    database: database.value || undefined,
    schema: schema.value || undefined,
    source: {
      table: table.value,
      columns: columns.value,
      whereClause: whereClause.value || undefined,
      orderBy: orderBy.value || undefined,
      limit: limit.value || undefined,
    },
    format: selectedFormat.value,
    outputPath: outputPath.value || undefined,
    csvOptions: selectedFormat.value === 'csv'
      ? {
          delimiter: csvDelimiter.value,
          encoding: 'UTF-8',
          includeHeader: csvIncludeHeader.value,
        }
      : undefined,
    excelOptions: selectedFormat.value === 'excel'
      ? {
          sheetName: excelSheetName.value,
          includeHeader: excelIncludeHeader.value,
          freezeHeader: excelFreezeHeader.value,
          autoFitColumns: excelAutoFit.value,
        }
      : undefined,
    sqlOptions: selectedFormat.value === 'sql'
      ? {
          targetTable: table.value,
          includeCreateTable: sqlIncludeCreateTable.value,
          includeDropTable: sqlIncludeDropTable.value,
          batchSize: sqlBatchSize.value,
        }
      : undefined,
  }
})

// Sync from store to local state on mount
watch(() => transferStore.exportRequest, (req) => {
  if (req.connectionId && !connectionId.value)
    connectionId.value = req.connectionId
  if (req.database && !database.value)
    database.value = req.database
  if (req.schema && !schema.value)
    schema.value = req.schema
  if (req.source?.table && !table.value)
    table.value = req.source.table
  if (req.source?.columns?.length && !columns.value.length)
    columns.value = req.source.columns
  if (req.format && !selectedFormat.value)
    selectedFormat.value = req.format
}, { immediate: true })

// Source summary for display
const sourceSummary = computed(() => {
  if (connectionId.value && table.value)
    return `${table.value} (${columns.value.length} cols)`
  if (connectionId.value)
    return connectionId.value
  return ''
})

// Format options (using subset of ExportFormat)
const formatOptions: { value: ExportFormat, label: string, desc: string }[] = [
  { value: 'csv', label: 'CSV', desc: 'Comma-separated values' },
  { value: 'jsonl', label: 'JSONL', desc: 'JSON Lines format' },
  { value: 'excel', label: 'Excel', desc: 'Microsoft Excel (.xlsx)' },
  { value: 'sql', label: 'SQL', desc: 'INSERT statements (.sql)' },
]

// Load preview when source is configured
async function loadPreview() {
  if (!connectionId.value || !table.value || !columns.value.length)
    return

  isLoadingPreview.value = true
  try {
    const result = await previewExport(transferStore.exportRequest as any, 10)
    previewData.value = result.formattedPreview
    estimatedRows.value = result.totalRowsEstimate
  }
  catch (error) {
    console.error('Preview failed:', error)
  }
  finally {
    isLoadingPreview.value = false
  }
}

// Preview params
const previewParams = computed(() => {
  if (!connectionId.value || !table.value || !columns.value.length)
    return null
  return {
    connectionId: connectionId.value,
    table: table.value,
    columns: [...columns.value], // Clone to ensure identity change if elements change
  }
})

// Watch for changes to load preview
watch(previewParams, (params, oldParams) => {
  if (params && JSON.stringify(params) !== JSON.stringify(oldParams)) {
    loadPreview()
  }
}, { deep: true })

// Browse output file
async function handleBrowse() {
  const selected = await open({
    multiple: false,
    directory: false,
    save: true,
    filters: [
      { name: 'Export File', extensions: ['csv', 'jsonl', 'xlsx', 'sql'] },
    ],
  })
  if (selected) {
    outputPath.value = selected as string
  }
}

// Check if can export
const canExport = computed(() =>
  connectionId.value !== ''
  && table.value !== ''
  && columns.value.length > 0
  && outputPath.value !== '',
)
</script>

<template>
  <div class="pb-6 flex flex-col gap-4">
    <!-- Step 1: Source -->
    <TransferStepCard
      :title="t('transfer.export.step.source')"
      :step-number="1"
      icon="i-carbon-data-base"
      icon-class="text-emerald-600 dark:text-emerald-500"
      :summary="sourceSummary"
    >
      <ConnectionSelector
        v-model:connection-id="connectionId"
        v-model:database="database"
        v-model:schema="schema"
        show-schema
      />

      <div class="mt-4 pt-4 border-t border-border/40 gap-6 grid grid-cols-1 md:grid-cols-2">
        <div class="space-y-4">
          <TableSelector
            v-model:table="table"
            :connection-id="connectionId"
            :database="database"
            :schema="schema"
          />
        </div>
        <div class="space-y-4">
          <ColumnSelector
            v-model:columns="columns"
            :connection-id="connectionId"
            :database="database"
            :schema="schema"
            :table="table"
          />
        </div>
      </div>

      <!-- Advanced filters -->
      <div v-if="table" class="mt-4 pt-4 border-t border-border/40 gap-5 grid grid-cols-1 md:grid-cols-3">
        <div class="space-y-2.5">
          <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">{{ t('transfer.export.whereClause') }}</Label>
          <Input
            v-model="whereClause"
            :placeholder="t('transfer.export.whereClausePlaceholder')"
            class="text-sm font-mono"
          />
        </div>
        <div class="space-y-2.5">
          <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">{{ t('transfer.export.orderBy') }}</Label>
          <Input
            v-model="orderBy"
            :placeholder="t('transfer.export.orderByPlaceholder')"
            class="text-sm font-mono"
          />
        </div>
        <div class="space-y-2.5">
          <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">{{ t('transfer.export.limit') }}</Label>
          <Input
            v-model.number="limit"
            type="number"
            min="1"
            :placeholder="t('transfer.export.limitPlaceholder')"
          />
        </div>
      </div>
    </TransferStepCard>

    <!-- Step 2: Format -->
    <TransferStepCard
      :title="t('transfer.transfer.export.step.formatOutput')"
      :step-number="2"
      icon="i-carbon-document"
      icon-class="text-blue-600 dark:text-blue-500"
    >
      <div class="flex flex-row gap-3">
        <label
          v-for="opt in formatOptions"
          :key="opt.value"
          class="flex-1 px-4 py-3 border rounded-lg flex cursor-pointer transition-all duration-150 items-center gap-3 hover:bg-muted/50"
          :class="selectedFormat === opt.value ? 'border-primary bg-primary/5' : 'border-border bg-card'"
        >
          <input type="radio" :value="opt.value" v-model="selectedFormat" class="accent-primary" />
          <div>
            <div class="text-sm font-semibold leading-none">{{ opt.label }}</div>
            <div class="text-xs text-muted-foreground mt-1">{{ opt.desc }}</div>
          </div>
        </label>
      </div>

      <!-- CSV Options -->
      <div v-if="selectedFormat === 'csv'" class="mt-3 pt-4 border-t border-border/40">
        <div class="gap-5 grid grid-cols-1 md:grid-cols-3">
          <div class="space-y-2.5">
            <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Delimiter</Label>
            <Select v-model="csvDelimiter">
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value=",">
                  Comma (,)
                </SelectItem>
                <SelectItem value=";">
                  Semicolon (;)
                </SelectItem>
                <SelectItem value="\t">
                  Tab
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="flex items-center space-x-2 sm:mt-4">
            <Checkbox id="export-wiz-csv-header" v-model:checked="csvIncludeHeader" />
            <Label for="export-wiz-csv-header" class="text-sm leading-none font-medium cursor-pointer">Include header row</Label>
          </div>
        </div>
      </div>

      <!-- Excel Options -->
      <div v-if="selectedFormat === 'excel'" class="mt-3 pt-4 border-t border-border/40">
        <div class="gap-5 grid grid-cols-1 md:grid-cols-3">
          <div class="space-y-2.5">
            <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Sheet Name</Label>
            <Input v-model="excelSheetName" placeholder="Sheet1" />
          </div>
          <div class="flex items-center space-x-2 sm:mt-8">
            <Checkbox id="export-excel-header" v-model:checked="excelIncludeHeader" />
            <Label for="export-excel-header" class="text-sm leading-none font-medium cursor-pointer">Include header row</Label>
          </div>
          <div class="flex items-center space-x-2 sm:mt-8">
            <Checkbox id="export-excel-freeze" v-model:checked="excelFreezeHeader" />
            <Label for="export-excel-freeze" class="text-sm leading-none font-medium cursor-pointer">Freeze header row</Label>
          </div>
          <div class="flex items-center space-x-2">
            <Checkbox id="export-excel-autofit" v-model:checked="excelAutoFit" />
            <Label for="export-excel-autofit" class="text-sm leading-none font-medium cursor-pointer">Auto-fit column widths</Label>
          </div>
        </div>
      </div>

      <!-- SQL Options -->
      <div v-if="selectedFormat === 'sql'" class="mt-3 pt-4 border-t border-border/40">
        <div class="gap-5 grid grid-cols-1 md:grid-cols-3">
          <div class="space-y-2.5">
            <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Batch Size</Label>
            <Input v-model.number="sqlBatchSize" type="number" min="1" max="10000" />
          </div>
          <div class="flex items-center space-x-2 sm:mt-8">
            <Checkbox id="export-sql-create" v-model:checked="sqlIncludeCreateTable" />
            <Label for="export-sql-create" class="text-sm leading-none font-medium cursor-pointer">Include CREATE TABLE</Label>
          </div>
          <div class="flex items-center space-x-2 sm:mt-8">
            <Checkbox id="export-sql-drop" v-model:checked="sqlIncludeDropTable" />
            <Label for="export-sql-drop" class="text-sm leading-none font-medium cursor-pointer">Include DROP TABLE</Label>
          </div>
        </div>
      </div>

      <!-- Preview -->
      <div v-if="previewData" class="mt-3 pt-4 border-t border-border/40 flex flex-col space-y-2">
        <div class="flex items-center justify-between">
          <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Preview</Label>
          <div v-if="estimatedRows" class="flex gap-2 items-center">
            <Badge variant="secondary" class="text-[10px] tracking-wider px-1.5 py-0 uppercase">
              ~{{ estimatedRows.toLocaleString() }} rows
            </Badge>
            <Badge variant="secondary" class="text-[10px] tracking-wider px-1.5 py-0 uppercase">
              {{ columns.length }} cols
            </Badge>
          </div>
        </div>
        <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border text-xs text-muted-foreground font-mono p-3 border border-border/50 rounded-md bg-muted/30 max-h-[140px] whitespace-pre-wrap shadow-inner overflow-auto">
          {{ previewData.slice(0, 1000) }}{{ previewData.length > 1000 ? '...' : '' }}
        </div>
      </div>
      <div v-else-if="isLoadingPreview" class="mt-3 pt-4 border-t border-border/40 text-sm text-muted-foreground flex items-center gap-2">
        <span class="i-carbon-circle-dash opacity-50 h-4 w-4 animate-spin" />
        <span>Loading preview...</span>
      </div>

      <!-- Output Path + Export -->
      <div class="mt-3 pt-4 border-t border-border/40 flex gap-3 items-center">
        <Input
          v-model="outputPath"
          :placeholder="`/path/to/output.${selectedFormat === 'excel' ? 'xlsx' : selectedFormat}`"          class="text-sm font-mono flex-1"
        />
        <Button variant="outline" @click="handleBrowse">
          <span class="i-carbon-folder mr-2" /> Browse
        </Button>
        <Button :disabled="!canExport">
          <span class="i-carbon-document-export mr-2" /> Export
        </Button>
      </div>
    </TransferStepCard>
  </div>
</template>
