<script setup lang="ts">
import type { ExportFormat, TableColumns } from '@/types/transfer'

import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'

import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useTransferStore } from '@/store/transferStore'
import ConnectionSelector from '../shared/ConnectionSelector.vue'
import MultiTableSelector from '../shared/MultiTableSelector.vue'
import TabbedColumnSelector from '../shared/TabbedColumnSelector.vue'
import TransferStepCard from '../shared/TransferStepCard.vue'

const { t } = useI18n()
const transferStore = useTransferStore()

// Local state for all inputs
const connectionId = ref('')
const database = ref('')
const schema = ref('')
const selectedTables = ref<string[]>([])
const tableColumns = ref<TableColumns[]>([])
const selectedFormat = ref<ExportFormat>('csv')

// Reset tables and columns when connection or database changes
watch([connectionId, database], ([newConnId, newDb], [oldConnId, oldDb]) => {
  if (newConnId !== oldConnId || newDb !== oldDb) {
    selectedTables.value = []
    tableColumns.value = []
  }
})

// Format-specific options
const csvDelimiter = ref(',')
const csvIncludeHeader = ref(true)
const jsonlDateFormat = ref('ISO8601')
const excelIncludeHeader = ref(true)
const excelFreezeHeader = ref(true)
const excelAutoFit = ref(true)
const sqlIncludeCreateTable = ref(true)
const sqlIncludeDropTable = ref(false)
const sqlBatchSize = ref(1000)

const outputPath = ref('')

// Source summary for display
const sourceSummary = computed(() => {
  if (connectionId.value && selectedTables.value.length > 0) {
    const colCount = tableColumns.value.reduce((sum, tc) => sum + tc.selectedColumns.length, 0)
    return `${selectedTables.value.length} tables (${colCount} cols)`
  }
  if (connectionId.value)
    return connectionId.value
  return ''
})

// Format options
const formatOptions: { value: ExportFormat, label: string, icon: string }[] = [
  { value: 'csv', label: 'CSV', icon: 'i-carbon-document-csv' },
  { value: 'jsonl', label: 'JSONL', icon: 'i-carbon-document-json' },
  { value: 'excel', label: 'Excel', icon: 'i-carbon-document-xls' },
  { value: 'sql', label: 'SQL', icon: 'i-carbon-document-sql' },
]

// Browse output file
async function handleBrowse() {
  const extension = selectedFormat.value === 'excel' ? 'xlsx' : selectedFormat.value
  const selected = await open({
    multiple: false,
    directory: false,
    save: true,
    filters: [
      { name: 'Export File', extensions: [extension] },
    ],
  })
  if (selected) {
    outputPath.value = selected as string
  }
}

// Check if can export
const canExport = computed(() =>
  connectionId.value !== ''
  && selectedTables.value.length > 0
  && tableColumns.value.some(tc => tc.selectedColumns.length > 0)
  && outputPath.value !== '',
)

// Sync to store (simplified for MVP - single table export from first selected)
watch([
  connectionId,
  database,
  schema,
  selectedTables,
  tableColumns,
  selectedFormat,
  outputPath,
], () => {
  // For MVP, we'll export the first selected table
  const firstTable = selectedTables.value[0]
  const firstTableColumns = tableColumns.value.find(tc => tc.tableName === firstTable)

  transferStore.exportRequest = {
    connectionId: connectionId.value || undefined,
    database: database.value || undefined,
    schema: schema.value || undefined,
    source: {
      table: firstTable || '',
      columns: firstTableColumns?.selectedColumns || [],
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
    jsonlOptions: selectedFormat.value === 'jsonl'
      ? {
          dateFormat: jsonlDateFormat.value,
        }
      : undefined,
    excelOptions: selectedFormat.value === 'excel'
      ? {
          includeHeader: excelIncludeHeader.value,
          freezeHeader: excelFreezeHeader.value,
          autoFitColumns: excelAutoFit.value,
        }
      : undefined,
    sqlOptions: selectedFormat.value === 'sql'
      ? {
          targetTable: firstTable || '',
          includeCreateTable: sqlIncludeCreateTable.value,
          includeDropTable: sqlIncludeDropTable.value,
          batchSize: sqlBatchSize.value,
        }
      : undefined,
  }
})

// Start export (placeholder - actual implementation via background task)
function startExport() {
  // TODO: Implement actual export execution
  console.log('Starting export:', transferStore.exportRequest)
}
</script>

<template>
  <div class="pb-4 flex flex-col gap-2.5">
    <!-- Step 1: Source -->
    <TransferStepCard
      :title="t('transfer.export.step.source')"
      :step-number="1"
      icon="i-carbon-data-base"
      icon-class="text-emerald-600 dark:text-emerald-500"
      :summary="sourceSummary"
      min-height="340px"
    >
      <div class="gap-3 grid grid-cols-1 h-[280px] items-stretch overflow-hidden lg:grid-cols-3">
        <!-- Left: Connection, Database, Schema (1/3) -->
        <div class="lg:col-span-1">
          <ConnectionSelector
            v-model:connection-id="connectionId"
            v-model:database="database"
            v-model:schema="schema"
            show-schema
          />
        </div>

        <!-- Right: Tables and Columns (2/3) -->
        <div class="gap-3 grid grid-cols-1 h-full overflow-hidden lg:col-span-2 lg:grid-cols-2">
          <MultiTableSelector
            v-model:selected-tables="selectedTables"
            :connection-id="connectionId"
            :database="database"
            :schema="schema"
          />

          <TabbedColumnSelector
            v-model:table-columns="tableColumns"
            :connection-id="connectionId"
            :database="database"
            :schema="schema"
            :selected-tables="selectedTables"
          />
        </div>
      </div>
    </TransferStepCard>

    <!-- Step 2: Format & Output -->
    <TransferStepCard
      :title="t('transfer.export.step.formatOutput')"
      :step-number="2"
      icon="i-carbon-document"
      icon-class="text-blue-600 dark:text-blue-500"
      min-height="200px"
    >
      <div class="flex flex-row gap-4 min-h-[120px]">
        <!-- Left: Format Grid (2 items per row) -->
        <div class="flex-1 min-w-0">
          <Label class="text-[11px] text-muted-foreground tracking-wide font-medium mb-2 block uppercase">
            {{ t('transfer.format.label', 'Format') }}
          </Label>
          <div class="gap-2 grid grid-cols-2">
            <button
              v-for="opt in formatOptions"
              :key="opt.value"
              class="px-3 py-2.5 border rounded-md flex gap-2 cursor-pointer transition-all duration-150 items-center"
              :class="selectedFormat === opt.value
                ? 'border-primary/60 bg-primary/[0.04] ring-1 ring-primary/20'
                : 'border-border/40 bg-card hover:bg-muted/40'"
              @click="selectedFormat = opt.value"
            >
              <span :class="[opt.icon, selectedFormat === opt.value ? 'h-4 w-4 text-primary' : 'h-4 w-4 text-muted-foreground']" />
              <span class="text-xs font-semibold" :class="selectedFormat === opt.value ? 'text-primary' : ''">
                {{ opt.label }}
              </span>
            </button>
          </div>
        </div>

        <!-- Right: Format-specific Config -->
        <div class="pl-4 border-l border-border/40 flex-1 min-w-0 space-y-3">
          <!-- CSV Options -->
          <div v-if="selectedFormat === 'csv'" class="space-y-3">
            <div class="gap-3 grid grid-cols-2 items-center">
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Delimiter</Label>
                <Select v-model="csvDelimiter">
                  <SelectTrigger class="text-xs font-mono h-8">
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
              <div class="flex items-center space-x-2 sm:mt-5">
                <Checkbox id="csv-header" v-model:checked="csvIncludeHeader" class="h-3.5 w-3.5" />
                <Label for="csv-header" class="text-xs cursor-pointer">Include header row</Label>
              </div>
            </div>
          </div>

          <!-- JSONL Options -->
          <div v-if="selectedFormat === 'jsonl'" class="space-y-3">
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Date Format</Label>
              <Select v-model="jsonlDateFormat">
                <SelectTrigger class="text-xs h-8">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="ISO8601">
                    ISO 8601
                  </SelectItem>
                  <SelectItem value="Unix">
                    Unix timestamp
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <!-- Excel Options -->
          <div v-if="selectedFormat === 'excel'" class="space-y-3">
            <div class="flex flex-col gap-2">
              <div class="flex items-center space-x-2">
                <Checkbox id="excel-header" v-model:checked="excelIncludeHeader" class="h-3.5 w-3.5" />
                <Label for="excel-header" class="text-xs cursor-pointer">Include header</Label>
              </div>
              <div class="flex items-center space-x-2">
                <Checkbox id="excel-freeze" v-model:checked="excelFreezeHeader" class="h-3.5 w-3.5" />
                <Label for="excel-freeze" class="text-xs cursor-pointer">Freeze header</Label>
              </div>
              <div class="flex items-center space-x-2">
                <Checkbox id="excel-autofit" v-model:checked="excelAutoFit" class="h-3.5 w-3.5" />
                <Label for="excel-autofit" class="text-xs cursor-pointer">Auto-fit columns</Label>
              </div>
            </div>
            <div class="text-[10px] text-muted-foreground italic">
              Sheet names: db-schema-table (each table as a separate sheet)
            </div>
          </div>

          <!-- SQL Options -->
          <div v-if="selectedFormat === 'sql'" class="space-y-3">
            <div class="gap-3 grid grid-cols-2 items-start">
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Batch Size</Label>
                <Input v-model.number="sqlBatchSize" type="number" min="1" max="10000" class="text-xs font-mono h-8" />
              </div>
              <div class="flex flex-col gap-2 sm:mt-5">
                <div class="flex items-center space-x-2">
                  <Checkbox id="sql-create" v-model:checked="sqlIncludeCreateTable" class="h-3.5 w-3.5" />
                  <Label for="sql-create" class="text-xs cursor-pointer">Include CREATE TABLE</Label>
                </div>
                <div class="flex items-center space-x-2">
                  <Checkbox id="sql-drop" v-model:checked="sqlIncludeDropTable" class="h-3.5 w-3.5" />
                  <Label for="sql-drop" class="text-xs cursor-pointer">Include DROP TABLE</Label>
                </div>
              </div>
            </div>
          </div>

          <div v-if="!selectedFormat" class="text-[11px] text-muted-foreground py-4 text-center">
            Select a format to configure options
          </div>

          <!-- Output Path (inside config container) -->
          <div v-if="selectedFormat" class="pt-3 border-t border-border/40 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Output Path</Label>
            <div class="flex gap-2 items-center">
              <div class="flex-1 relative">
                <span class="i-carbon-document text-muted-foreground left-2.5 top-1/2 absolute -translate-y-1/2" />
                <Input
                  v-model="outputPath"
                  :placeholder="`/path/to/output.${selectedFormat === 'excel' ? 'xlsx' : selectedFormat}`"
                  class="text-[11px] font-mono pl-8 h-8"
                />
              </div>
              <Button variant="outline" size="sm" class="text-xs px-3 h-8" @click="handleBrowse">
                <span class="i-carbon-folder mr-1.5" /> {{ t('common.buttons.browse') }}
              </Button>
            </div>
          </div>
        </div>
      </div>

      <!-- Start Export Button (at bottom) -->
      <div class="mt-3 pt-3 border-t border-border/40 flex justify-end">
        <Button size="sm" class="text-xs font-semibold px-5 h-8" :disabled="!canExport" @click="startExport">
          <span class="i-carbon-play mr-1.5" /> {{ t('transfer.export.step.execute', 'Start Export') }}
        </Button>
      </div>

      <!-- Selection Summary -->
      <div v-if="selectedTables.length > 0" class="mt-2 pt-2 border-t border-border/40 flex gap-2 items-center">
        <Badge variant="secondary" class="text-[10px] font-mono px-1.5 py-0.5 border-border/40 bg-muted/30">
          {{ selectedTables.length }} {{ t('transfer.migration.tablesSelected', 'tables') }}
        </Badge>
        <Badge variant="secondary" class="text-[10px] font-mono px-1.5 py-0.5 border-border/40 bg-muted/30">
          {{ tableColumns.reduce((sum, tc) => sum + tc.selectedColumns.length, 0) }} cols
        </Badge>
        <Badge variant="outline" class="text-[10px] font-mono px-1.5 py-0.5 uppercase">
          {{ selectedFormat }}
        </Badge>
      </div>
    </TransferStepCard>
  </div>
</template>
