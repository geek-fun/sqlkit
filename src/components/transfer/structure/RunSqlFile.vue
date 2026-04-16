<script setup lang="ts">
import type { TransferProgress, TransferResult } from '@/types/transfer'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import ConnectionSelector from '../shared/ConnectionSelector.vue'
import FileDropZone from '../shared/FileDropZone.vue'
import ProgressPanel from '../shared/ProgressPanel.vue'

import ResultPanel from '../shared/ResultPanel.vue'

const { t } = useI18n()

const connectionId = ref('')
const database = ref('')
const filePath = ref('')
const fileContent = ref('')
const onError = ref<'rollback' | 'skipAndContinue' | 'stop'>('stop')

const executing = ref(false)
const progress = ref<TransferProgress | null>(null)
const result = ref<TransferResult | null>(null)

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

async function executeSql() {
  if (!connectionId.value || !filePath.value)
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
    progress.value = {
      operation: 'sqlFile',
      phase: 'finalizing',
      processedRows: result.value?.processedRows ?? 0,
      skippedRows: result.value?.skippedRows ?? 0,
      errorCount: result.value?.errorCount ?? 0,
      percent: 100,
      elapsedMs: result.value?.durationMs ?? 0,
    }
  }
}

function reset() {
  filePath.value = ''
  fileContent.value = ''
  result.value = null
}

const canExecute = computed(() =>
  connectionId.value !== '' && filePath.value !== '',
)
</script>

<template>
  <div class="space-y-6">
    <ConnectionSelector
      v-model:connection-id="connectionId"
      v-model:database="database"
    />

    <div class="space-y-4">
      <Label>{{ t('pages.transfer.structure.sqlFile') }}</Label>
      <FileDropZone
        :accepted-formats="['sql']"
        @file-selected="handleFileSelected"
      />

      <div v-if="fileContent" class="p-4 border rounded bg-muted/30 max-h-200px overflow-auto">
        <pre class="text-sm font-mono whitespace-pre-wrap">{{ fileContent.slice(0, 2000) }}{{ fileContent.length > 2000 ? '...' : '' }}</pre>
      </div>
    </div>

    <div class="space-y-4">
      <Label>{{ t('pages.transfer.structure.errorHandling') }}</Label>
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

    <div class="flex gap-2 justify-end">
      <Button v-if="result" variant="outline" @click="reset">
        {{ t('pages.transfer.structure.reset') }}
      </Button>
      <Button :disabled="!canExecute" :loading="executing" @click="executeSql">
        {{ t('pages.transfer.structure.execute') }}
      </Button>
    </div>

    <ProgressPanel v-if="executing" :progress="progress" :is-running="executing" />

    <ResultPanel v-if="result" :result="result" />
  </div>
</template>
