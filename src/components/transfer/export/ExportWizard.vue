<script setup lang="ts">
import type { ExportFormat, ExportSource, TransferResult, TransferScope } from '@/types/transfer'

import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'

import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { executeExport } from '@/datasources/transferApi'
import { ConnectionStatus, useConnectionStore } from '@/store/connectionStore'
import { useDatabaseStore } from '@/store/databaseStore'
import { useTransferStore } from '@/store/transferStore'
import ConnectionSelector from '../shared/ConnectionSelector.vue'
import MultiTableSelector from '../shared/MultiTableSelector.vue'
import ScopeSelector from '../shared/ScopeSelector.vue'
import TransferStepCard from '../shared/TransferStepCard.vue'

const { t } = useI18n()
const transferStore = useTransferStore()
const connectionStore = useConnectionStore()
const databaseStore = useDatabaseStore()

// Local state for all inputs
const scope = ref<TransferScope>('server')
const connectionId = ref('')
const database = ref('')
const schema = ref('')
const selectedTables = ref<string[]>([])
const selectedFormat = ref<ExportFormat>('sql')

// Available databases from the connected connection
const availableDatabases = computed(() => {
  if (!connectionId.value)
    return []
  const isConnected = connectionStore.getConnectionStatus(connectionId.value) === ConnectionStatus.CONNECTED
  if (!isConnected)
    return []
  return databaseStore.metadata[connectionId.value]?.databases ?? []
})

// Reset tables when connection or database changes
watch([connectionId, database], ([newConnId, newDb], [oldConnId, oldDb]) => {
  if (newConnId !== oldConnId || newDb !== oldDb) {
    selectedTables.value = []
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
  if (!connectionId.value)
    return ''

  switch (scope.value) {
    case 'server':
      return 'All databases on this server'
    case 'database':
      return database.value ? `All tables in ${database.value}` : 'Select a database'
    case 'tables':
    default:
      return selectedTables.value.length > 0
        ? `${selectedTables.value.length} tables`
        : 'Select tables'
  }
})

// Format options
const formatOptions: { value: ExportFormat, label: string, icon: string }[] = [
  { value: 'sql', label: 'SQL', icon: 'i-carbon-document-sql' },
  { value: 'jsonl', label: 'JSONL', icon: 'i-carbon-document-json' },
  { value: 'csv', label: 'CSV', icon: 'i-carbon-document-csv' },
  { value: 'excel', label: 'XLS', icon: 'i-carbon-document-xls' },
]

// Browse output file or directory
async function handleBrowse() {
  if (scope.value === 'tables') {
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
  else {
    const selected = await open({
      directory: true,
      multiple: false,
    })
    if (selected) {
      outputPath.value = selected as string
    }
  }
}

// Check if can export
const canExport = computed(() => {
  if (!connectionId.value || !outputPath.value)
    return false

  switch (scope.value) {
    case 'server':
      return true
    case 'database':
      return !!database.value
    case 'tables':
    default:
      return selectedTables.value.length > 0
  }
})

// Sync to store
watch([
  scope,
  connectionId,
  database,
  schema,
  selectedTables,
  selectedFormat,
  outputPath,
], () => {
  const sources: ExportSource[] = scope.value === 'tables'
    ? selectedTables.value.map(t => ({ table: t, columns: [] }))
    : []

  const firstTable = selectedTables.value[0]

  transferStore.exportRequest = {
    connectionId: connectionId.value || undefined,
    database: database.value || undefined,
    schema: schema.value || undefined,
    scope: scope.value,
    sources,
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

// Start export
async function startExport() {
  const request = transferStore.exportRequest
  if (!request.connectionId || !request.outputPath)
    return

  transferStore.startOperation()
  try {
    const result = await executeExport(request as any)
    transferStore.completeOperation(result)
  }
  catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    const errorResult: TransferResult = {
      success: false,
      totalRows: 0,
      processedRows: 0,
      skippedRows: 0,
      errorCount: 1,
      durationMs: 0,
      errors: [{ message }],
    }
    transferStore.completeOperation(errorResult)
  }
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
        <!-- Left: Connection + Scope + conditional selectors (1/3) -->
        <div class="space-y-4 lg:col-span-1">
          <!-- Step 1: Connection (Server) selector only -->
          <ConnectionSelector
            v-model:connection-id="connectionId"
            v-model:database="database"
            v-model:schema="schema"
            :show-database="false"
            :show-schema="false"
          />

          <!-- Step 2: Scope selector -->
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground tracking-wide font-medium uppercase">Scope</Label>
            <ScopeSelector :scope="scope" @update:scope="scope = $event" />
          </div>

          <!-- Step 3: Database selector (only when scope is 'database' or 'tables') -->
          <div v-if="scope === 'database' || scope === 'tables'" class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground tracking-wide font-medium uppercase">Database</Label>
            <Select v-model="database" :disabled="!connectionId">
              <SelectTrigger class="text-xs border-border/40 bg-muted/20 h-8">
                <SelectValue placeholder="Select database" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="db in availableDatabases"
                  :key="db.name"
                  :value="db.name"
                  class="text-xs"
                >
                  <div class="flex gap-2 w-full items-center">
                    <span>{{ db.name }}</span>
                    <span
                      v-if="db.is_system"
                      class="text-[10px] text-muted-foreground tracking-wide font-mono ml-auto px-1 rounded-sm bg-muted/60 uppercase"
                    >
                      System
                    </span>
                  </div>
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <!-- Scope info badge -->
          <Badge variant="secondary" class="text-[10px] font-mono px-1.5 py-0.5 border-border/40 bg-muted/30">
            {{ scope === 'server' ? 'All databases on this server' : scope === 'database' ? (database ? `All tables in ${database}` : 'Select a database') : (selectedTables.length > 0 ? `${selectedTables.length} tables` : 'Select tables') }}
          </Badge>
        </div>

        <!-- Right: Table Selection (2/3) - only for tables scope -->
        <div v-if="scope === 'tables'" class="h-full overflow-hidden lg:col-span-2">
          <MultiTableSelector
            v-model:selected-tables="selectedTables"
            :connection-id="connectionId"
            :database="database"
            :schema="schema"
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
      min-height="180px"
    >
      <div class="flex flex-col gap-3">
        <!-- Row 1: FORMAT + OUTPUT PATH side by side -->
        <div class="flex flex-row gap-4">
          <!-- Left: Format + Format-specific Options -->
          <div class="flex-1 min-w-0 space-y-2">
            <Label class="text-[11px] text-muted-foreground tracking-wide font-medium uppercase">
              {{ t('transfer.format.label', 'Format') }}
            </Label>

            <!-- Row 1: Format buttons -->
            <div class="gap-2 grid grid-cols-4">
              <button
                v-for="opt in formatOptions"
                :key="opt.value"
                class="px-3 py-2.5 border rounded-md flex gap-2 cursor-pointer transition-all duration-150 items-center justify-center"
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

            <!-- Row 2: Format-specific config (fixed min-height prevents layout shift) -->
            <div class="flex flex-col min-h-[60px] justify-start">
              <!-- CSV Options -->
              <div v-if="selectedFormat === 'csv'" class="flex gap-3 items-center">
                <div class="space-y-1.5">
                  <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Delimiter</Label>
                  <Select v-model="csvDelimiter">
                    <SelectTrigger class="text-xs font-mono h-8 w-24">
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
                  <Label for="csv-header" class="text-xs cursor-pointer">Include header</Label>
                </div>
              </div>

              <!-- JSONL Options -->
              <div v-if="selectedFormat === 'jsonl'" class="flex gap-3 items-center">
                <div class="space-y-1.5">
                  <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Date Format</Label>
                  <Select v-model="jsonlDateFormat">
                    <SelectTrigger class="text-xs h-8 w-28">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="ISO8601">
                        ISO 8601
                      </SelectItem>
                      <SelectItem value="Unix">
                        Unix
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <!-- SQL Options -->
              <div v-if="selectedFormat === 'sql'" class="flex flex-wrap gap-3 items-center">
                <div class="space-y-1.5">
                  <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Rows/Batch</Label>
                  <Input v-model.number="sqlBatchSize" type="number" min="1" max="10000" class="text-xs font-mono h-8 w-24" />
                </div>
                <div class="flex items-center space-x-2 sm:mt-5">
                  <Checkbox id="sql-create" v-model:checked="sqlIncludeCreateTable" class="h-3.5 w-3.5" />
                  <Label for="sql-create" class="text-xs cursor-pointer">CREATE TABLE</Label>
                </div>
                <div class="flex items-center space-x-2 sm:mt-5">
                  <Checkbox id="sql-drop" v-model:checked="sqlIncludeDropTable" class="h-3.5 w-3.5" />
                  <Label for="sql-drop" class="text-xs cursor-pointer">DROP TABLE</Label>
                </div>
              </div>

              <!-- Excel Options -->
              <div v-if="selectedFormat === 'excel'" class="flex gap-3 items-center">
                <div class="space-y-1.5">
                  <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Worksheet</Label>
                  <div class="flex gap-3 items-center">
                    <div class="flex items-center space-x-2">
                      <Checkbox id="excel-header" v-model:checked="excelIncludeHeader" class="h-3.5 w-3.5" />
                      <Label for="excel-header" class="text-xs cursor-pointer">Header</Label>
                    </div>
                    <div class="flex items-center space-x-2">
                      <Checkbox id="excel-freeze" v-model:checked="excelFreezeHeader" class="h-3.5 w-3.5" />
                      <Label for="excel-freeze" class="text-xs cursor-pointer">Freeze</Label>
                    </div>
                    <div class="flex items-center space-x-2">
                      <Checkbox id="excel-autofit" v-model:checked="excelAutoFit" class="h-3.5 w-3.5" />
                      <Label for="excel-autofit" class="text-xs cursor-pointer">Auto-fit</Label>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Right: Output Path -->
          <div class="pl-4 border-l border-border/40 flex-1 min-w-0 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Output Path</Label>
            <div class="flex gap-2 items-center">
              <Button variant="outline" size="sm" class="text-xs px-3 h-8" @click="handleBrowse">
                <span class="i-carbon-folder mr-1.5" /> {{ t('common.buttons.browse') }}
              </Button>
              <div class="flex-1 relative">
                <span class="i-carbon-document text-muted-foreground left-2.5 top-1/2 absolute -translate-y-1/2" />
                <Input
                  v-model="outputPath"
                  :placeholder="scope === 'tables' ? `/path/to/output.${selectedFormat === 'excel' ? 'xlsx' : selectedFormat}` : '/path/to/export/directory'"
                  class="text-[11px] font-mono pl-8 h-8"
                />
              </div>
            </div>
          </div>
        </div>

        <!-- Row 2: Execute action bar (full width) -->
        <div class="pt-3 border-t border-border/40 flex gap-3 items-center justify-end">
          <div v-if="connectionId" class="mr-auto flex gap-2 items-center">
            <Badge variant="secondary" class="text-[10px] font-mono px-1.5 py-0.5 border-border/40 bg-muted/30">
              {{ scope === 'server' ? 'All databases' : scope === 'database' ? (database || 'Select db') : `${selectedTables.length} tables` }}
            </Badge>
            <Badge variant="outline" class="text-[10px] font-mono px-1.5 py-0.5 uppercase">
              {{ selectedFormat }}
            </Badge>
          </div>
          <Button size="sm" class="text-xs font-semibold px-5 h-8" :disabled="!canExport" @click="startExport">
            <span class="i-carbon-play mr-1.5" /> {{ t('transfer.export.step.execute', 'Start Export') }}
          </Button>
        </div>
      </div>
    </TransferStepCard>
  </div>
</template>
