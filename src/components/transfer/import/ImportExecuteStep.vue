<script setup lang="ts">
import type { TransferProgress, TransferResult } from '@/types/transfer'

import { listen } from '@tauri-apps/api/event'

import { onMounted, onUnmounted, ref } from 'vue'
import { executeImport } from '@/datasources/transferApi'

import { useTransferStore } from '@/store/transferStore'
import ProgressPanel from '../shared/ProgressPanel.vue'

import ResultPanel from '../shared/ResultPanel.vue'

const transferStore = useTransferStore()

const isRunning = ref(false)
const progress = ref<TransferProgress | null>(null)
const result = ref<TransferResult | null>(null)

let unlistenProgress: (() => void) | null = null

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

  if (!result.value) {
    startImport()
  }
})

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress()
  }
})
</script>

<template>
  <div class="space-y-3">
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
  </div>
</template>
