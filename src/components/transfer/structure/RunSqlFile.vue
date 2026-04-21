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

// Handle file selection
async function handleFileSelected(file: File) {
  try {
    const path = await invoke<string>('save_temp_file', {
      name: file.name,
      content: await file.arrayBuffer(),
    })
    filePath.value = path
    const content = await invoke<string>('read_text_file', { path })
    fileContent.value = content
  }
  catch (error) {
    console.error('Failed to read file:', error)
  }
}

// Execute SQL
async function executeSql() {
  if (!connectionId.value || !filePath.value || !isConnected.value)
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
    const res = await invoke<TransferResult>('execute_sql_file', {
      connectionId: connectionId.value,
      database: database.value || null,
      filePath: filePath.value,
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
  connectionId.value !== '' && isConnected.value && filePath.value !== '',
)
</script>

<template>
  <div class="pb-8 flex flex-col gap-8">
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
      <div v-if="fileContent" class="flex flex-col space-y-3">
        <div class="flex items-center justify-between">
          <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Content Preview</Label>
          <Button variant="ghost" size="sm" class="h-8" @click="reset">
            <span class="i-carbon-close mr-2" /> Clear File
          </Button>
        </div>
        <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border text-xs text-muted-foreground font-mono p-3 border border-border/50 rounded-md bg-muted/30 max-h-[200px] whitespace-pre-wrap shadow-inner overflow-auto">
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
      <div class="gap-5 grid grid-cols-1 md:grid-cols-2">
        <div class="space-y-2.5">
          <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">{{ t('pages.transfer.structure.errorHandling') }}</Label>
          <Select v-model="onError">
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="stop">
                {{ t('pages.transfer.structure.onErrorStop') }}
              </SelectItem>
              <SelectItem value="skipAndContinue">
                {{ t('pages.transfer.structure.onErrorSkip') }}
              </SelectItem>
              <SelectItem value="rollback">
                {{ t('pages.transfer.structure.onErrorRollback') }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      <!-- Result -->
      <div v-if="result" class="mt-6 pt-6 border-t border-border/40">
        <div v-if="result.success" class="text-sm text-emerald-600 font-medium flex gap-2 items-center dark:text-emerald-500">
          <span class="i-carbon-checkmark-filled h-5 w-5" />
          {{ result.processedRows }} rows processed in {{ result.durationMs }}ms
        </div>
        <div v-else class="flex flex-col gap-2">
          <div class="text-sm text-destructive font-medium flex gap-2 items-center">
            <span class="i-carbon-warning-filled h-5 w-5" />
            {{ result.errorCount }} errors occurred
          </div>
          <div v-if="result.errors && result.errors.length" class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border text-xs text-destructive mt-2 p-3 border border-destructive/20 rounded bg-destructive/5 max-h-32 overflow-auto">
            <div v-for="(err, i) in result.errors" :key="i" class="mb-1 last:mb-0">
              {{ err.message }}
            </div>
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="mt-8 pt-4 flex gap-3 justify-end">
        <Button :disabled="!canExecute || executing" class="min-w-[120px]" @click="executeSql">
          <span v-if="executing" class="i-carbon-circle-dash mr-2 animate-spin" />
          <span v-else class="i-carbon-play mr-2" />
          Execute SQL
        </Button>
      </div>
    </TransferStepCard>
  </div>
</template>
