<script setup lang="ts">
import type { TransferProgress, TransferResult } from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { ConnectionStatus, useConnectionStore } from '@/store/connectionStore'
import ConnectionSelector from '../shared/ConnectionSelector.vue'
import FileDropZone from '../shared/FileDropZone.vue'
import TransferStepCard from '../shared/TransferStepCard.vue'

const { t } = useI18n()
const connectionStore = useConnectionStore()

const connectionId = ref('')
const database = ref('')
const filePath = ref('')
const fileContent = ref('')
const onError = ref<'rollback' | 'skipAndContinue' | 'stop'>('stop')

const executing = ref(false)
const progress = ref<TransferProgress | null>(null)
const result = ref<TransferResult | null>(null)

// Check connection status
const isConnected = computed(() => {
  if (!connectionId.value)
    return false
  return connectionStore.getConnectionStatus(connectionId.value) === ConnectionStatus.CONNECTED
})

// Summary for display
const fileSummary = computed(() => {
  if (filePath.value)
    return filePath.value.split('/').pop()
  return ''
})

// Handle file selection - read content via FileReader (browser API)
async function handleFileSelected(file: File) {
  try {
    filePath.value = file.name
    fileContent.value = await file.text()
  }
  catch (error) {
    console.error('Failed to read file:', error)
  }
}

// Execute SQL
async function executeSql() {
  if (!connectionId.value || !fileContent.value || !isConnected.value)
    return

  executing.value = true
  progress.value = {
    operation: 'sqlFile',
    phase: 'preparing',
    processedRows: 0,
    skippedRows: 0,
    errorCount: 0,
    percent: 0,
    elapsedMs: 0,
  }
  result.value = null

  try {
    const res = await invoke<TransferResult>('execute_sql_content', {
      connectionId: connectionId.value,
      database: database.value || null,
      content: fileContent.value,
      onError: onError.value,
    })
    result.value = res
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
    executing.value = false
  }
}

// Reset
function reset() {
  filePath.value = ''
  fileContent.value = ''
  result.value = null
}

const canExecute = computed(() =>
  connectionId.value !== '' && isConnected.value && fileContent.value !== '',
)
</script>

<template>
  <div class="pb-8 flex flex-col gap-4">
    <!-- Connection -->
    <TransferStepCard
      title="Target Database"
      :step-number="1"
      icon="i-carbon-data-base"
      icon-class="text-emerald-600 dark:text-emerald-500"
    >
      <ConnectionSelector
        v-model:connection-id="connectionId"
        v-model:database="database"
      />
    </TransferStepCard>

    <!-- File -->
    <TransferStepCard
      title="SQL File"
      :step-number="2"
      icon="i-carbon-document"
      icon-class="text-blue-600 dark:text-blue-500"
      :summary="fileSummary"
    >
      <div v-if="!fileContent">
        <FileDropZone
          :accepted-formats="['sql']"
          @file-selected="handleFileSelected"
        />
      </div>

      <!-- File Preview -->
      <div v-if="fileContent" class="flex flex-col gap-3">
        <div class="mt-2 flex items-center justify-between">
          <Label class="text-[11px] text-muted-foreground tracking-wide flex gap-1.5 uppercase items-center">
            <span class="i-carbon-view" />
            Content Preview
          </Label>
          <Button variant="ghost" size="sm" class="text-xs px-2 h-8" @click="reset">
            <span class="i-carbon-close mr-1.5" /> Clear File
          </Button>
        </div>
        <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border text-xs leading-snug font-mono p-3 border border-border/40 rounded-md bg-muted/40 max-h-[200px] whitespace-pre-wrap shadow-sm overflow-auto">
          {{ fileContent.slice(0, 1500) }}{{ fileContent.length > 1500 ? '...' : '' }}
        </div>
      </div>
    </TransferStepCard>

    <!-- Options -->
    <TransferStepCard
      title="Execution Options"
      :step-number="3"
      icon="i-carbon-settings"
      icon-class="text-amber-600 dark:text-amber-500"
      variant="highlight"
    >
      <div class="gap-4 grid grid-cols-1 md:grid-cols-2">
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">{{ t('transfer.structure.errorHandling') }}</Label>
          <Select v-model="onError">
            <SelectTrigger class="text-xs h-8">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="stop" class="text-xs">
                {{ t('transfer.structure.onErrorStop') }}
              </SelectItem>
              <SelectItem value="skipAndContinue" class="text-xs">
                {{ t('transfer.structure.onErrorSkip') }}
              </SelectItem>
              <SelectItem value="rollback" class="text-xs">
                {{ t('transfer.structure.onErrorRollback') }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      <!-- Result -->
      <div v-if="result" class="mt-6 pt-4 border-t border-border/40 space-y-3">
        <div v-if="result.success" class="text-xs text-green-600 font-medium px-3 py-2 border border-green-500/20 rounded-md bg-green-500/10 flex gap-2 shadow-sm items-center">
          <span class="i-carbon-checkmark-filled shrink-0 h-4 w-4" />
          <span>
            <span class="font-mono tabular-nums">{{ result.processedRows }}</span> rows processed in <span class="font-mono tabular-nums">{{ result.durationMs }}</span>ms
          </span>
        </div>
        <div v-else class="flex flex-col gap-2">
          <div class="text-xs text-destructive font-medium px-3 py-2 border border-destructive/20 rounded-md bg-destructive/10 flex gap-2 shadow-sm items-center">
            <span class="i-carbon-warning-filled shrink-0 h-4 w-4" />
            <span><span class="font-mono tabular-nums">{{ result.errorCount }}</span> errors occurred</span>
          </div>
          <div v-if="result.errors && result.errors.length" class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border text-xs text-destructive p-3 border border-destructive/20 rounded-md bg-destructive/5 max-h-32 shadow-sm overflow-auto">
            <div v-for="(err, i) in result.errors" :key="i" class="leading-snug font-mono mb-1 last:mb-0">
              {{ err.message }}
            </div>
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="mt-6 pt-4 border-t border-border/40 flex gap-2 justify-end">
        <Button :disabled="!canExecute || executing" size="sm" class="h-8 min-w-[120px]" @click="executeSql">
          <span v-if="executing" class="i-carbon-circle-dash mr-1.5 animate-spin" />
          <span v-else class="i-carbon-play mr-1.5" />
          Execute SQL
        </Button>
      </div>
    </TransferStepCard>
  </div>
</template>
