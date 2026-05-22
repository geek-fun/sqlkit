<script setup lang="ts">
import type { ExportFormat, ExportRequest } from '@/types/transfer'
import { save } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import ColumnSelector from '@/components/transfer/shared/ColumnSelector.vue'
import { Button } from '@/components/ui/button'

import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { executeExport } from '@/datasources/transferApi'
import { useTransferStore } from '@/store/transferStore'

const props = defineProps<{
  open: boolean
  connectionId: string
  database?: string
  schema?: string
  table: string
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  'submitted': [jobId: string]
}>()

const { t } = useI18n()
const transferStore = useTransferStore()

const isOpen = computed({
  get: () => props.open,
  set: val => emit('update:open', val),
})

const selectedFormat = ref<ExportFormat>('csv')
const destination = ref('')
const selectedColumns = ref<string[]>([])
const showAdvanced = ref(false)

// SQL Options
const whereClause = ref('')
const limit = ref<number | undefined>(undefined)

// CSV Options
const csvDelimiter = ref(',')
const csvEncoding = ref('UTF-8')
const csvIncludeHeader = ref(true)

watch(() => props.open, (newVal) => {
  if (newVal) {
    selectedFormat.value = 'csv'
    showAdvanced.value = false
    whereClause.value = ''
    limit.value = undefined
    selectedColumns.value = [] // ColumnSelector will auto-select all
    destination.value = ''
  }
})

async function pickDestination() {
  const ext = selectedFormat.value === 'excel' ? 'xlsx' : selectedFormat.value
  const path = await save({ defaultPath: `${props.table}.${ext}` })
  if (path) {
    destination.value = path
  }
}

async function handleExport() {
  if (!destination.value)
    return

  const task = transferStore.createTask(
    'export',
    {
      connectionId: props.connectionId,
      database: props.database,
      schema: props.schema,
      table: props.table,
      columns: selectedColumns.value,
      whereClause: whereClause.value,
      limit: limit.value,
      format: selectedFormat.value,
      outputPath: destination.value,
    },
    0,
  )

  const request: ExportRequest = {
    connectionId: props.connectionId,
    database: props.database,
    schema: props.schema,
    source: {
      table: props.table,
      columns: selectedColumns.value,
      whereClause: whereClause.value,
      limit: limit.value,
    },
    format: selectedFormat.value,
    csvOptions: selectedFormat.value === 'csv'
      ? {
          delimiter: csvDelimiter.value,
          encoding: csvEncoding.value,
          includeHeader: csvIncludeHeader.value,
        }
      : undefined,
    outputPath: destination.value,
  }

  transferStore.addRunningTask(task)
  emit('submitted', task.id)
  emit('update:open', false)

  try {
    const result = await executeExport(request)
    transferStore.updateTaskStatus(task.id, 'completed')
    transferStore.syncProgressToTask(task.id, {
      operation: 'export',
      phase: 'completed',
      processedRows: result.processedRows,
      totalRows: result.totalRows,
      skippedRows: result.skippedRows,
      errorCount: result.errorCount,
      percent: 100,
      elapsedMs: result.durationMs,
    })
  }
  catch (error) {
    transferStore.updateTaskStatus(task.id, 'failed', String(error))
  }
}
</script>

<template>
  <Dialog v-model:open="isOpen">
    <DialogContent class="sm:max-w-[500px]">
      <div class="mb-4 flex flex-col space-y-1">
        <DialogTitle class="text-xl">
          {{ t('transfer.surface.quickExport.title', 'Quick export') }}
        </DialogTitle>
        <DialogDescription class="text-sm">
          {{ t('transfer.surface.quickExport.subtitle', 'Export this table in one step') }}
        </DialogDescription>
      </div>

      <div class="space-y-6">
        <!-- Destination & Format -->
        <div class="space-y-4">
          <div class="gap-3 grid grid-cols-[1fr_auto] items-end">
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Destination file</Label>
              <Input
                v-model="destination"
                class="text-xs font-mono"
                placeholder="/path/to/output.csv"
              />
            </div>
            <Button variant="secondary" @click="pickDestination">
              Browse...
            </Button>
          </div>
        </div>

        <!-- Advanced Toggle -->
        <div>
          <Button
            variant="ghost"
            size="sm"
            class="text-xs text-muted-foreground h-8 w-full"
            @click="showAdvanced = !showAdvanced"
          >
            <span :class="showAdvanced ? 'i-carbon-chevron-up' : 'i-carbon-chevron-down'" class="mr-1.5" />
            {{ t('transfer.surface.quickExport.advanced', 'Advanced options') }}
          </Button>

          <!-- Advanced Content -->
          <div v-if="showAdvanced" class="mt-4 p-4 border border-border/40 rounded-md bg-muted/20 space-y-6">
            <!-- Format -->
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">
                {{ t('transfer.surface.quickExport.format', 'Format') }}
              </Label>
              <Select v-model="selectedFormat">
                <SelectTrigger class="text-xs h-8">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="csv">
                    CSV (.csv)
                  </SelectItem>
                  <SelectItem value="jsonl">
                    JSONL (.jsonl)
                  </SelectItem>
                  <SelectItem value="sql">
                    SQL (.sql)
                  </SelectItem>
                  <SelectItem value="excel">
                    Excel (.xlsx)
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            <!-- CSV Options -->
            <div v-if="selectedFormat === 'csv'" class="gap-4 grid grid-cols-2">
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Delimiter</Label>
                <Select v-model="csvDelimiter">
                  <SelectTrigger class="text-xs h-8">
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
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Encoding</Label>
                <Select v-model="csvEncoding">
                  <SelectTrigger class="text-xs h-8">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="UTF-8">
                      UTF-8
                    </SelectItem>
                    <SelectItem value="ISO-8859-1">
                      ISO-8859-1
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div class="flex col-span-2 items-center space-x-2">
                <Checkbox id="export-csv-header" v-model:checked="csvIncludeHeader" />
                <Label for="export-csv-header" class="text-xs font-medium cursor-pointer">Include header row</Label>
              </div>
            </div>

            <!-- Columns -->
            <div class="pt-2 border-t border-border/40">
              <ColumnSelector
                v-model:columns="selectedColumns"
                :connection-id="props.connectionId"
                :database="props.database"
                :schema="props.schema"
                :table="props.table"
              />
            </div>

            <!-- WHERE & Limit -->
            <div class="pt-2 border-t border-border/40 gap-4 grid grid-cols-2">
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">
                  {{ t('transfer.surface.quickExport.whereClause', 'Where clause') }}
                </Label>
                <Input
                  v-model="whereClause"
                  class="text-xs font-mono h-8"
                  placeholder="id > 1000"
                />
              </div>
              <div class="space-y-1.5">
                <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">
                  {{ t('transfer.surface.quickExport.limit', 'Row limit') }}
                </Label>
                <Input
                  v-model="limit"
                  type="number"
                  class="text-xs font-mono h-8"
                  placeholder="No limit"
                  min="1"
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="mt-6 pt-4 border-t border-border/40 flex gap-3 justify-end">
        <Button variant="ghost" @click="isOpen = false">
          {{ t('transfer.surface.quickExport.cancel', 'Cancel') }}
        </Button>
        <Button :disabled="!destination" @click="handleExport">
          {{ t('transfer.surface.quickExport.submit', 'Start export') }}
        </Button>
      </div>
    </DialogContent>
  </Dialog>
</template>
