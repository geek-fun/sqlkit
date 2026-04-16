<script setup lang="ts">
import type { TransferResult } from '@/types/transfer'

import { computed } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'

import { Card, CardContent } from '@/components/ui/card'

const props = defineProps<{
  result?: TransferResult | null
  outputPath?: string
}>()

const emit = defineEmits<{
  openFile: []
  openFolder: []
  viewTable: []
  again: []
}>()

const isSuccess = computed(() => props.result?.success ?? false)

const durationSeconds = computed(() =>
  Math.floor((props.result?.durationMs ?? 0) / 1000),
)

function formatFileSize(bytes: number) {
  if (bytes < 1024)
    return `${bytes} B`
  if (bytes < 1024 * 1024)
    return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

function formatDuration(seconds: number) {
  if (seconds < 60)
    return `${seconds}s`
  const mins = Math.floor(seconds / 60)
  const secs = seconds % 60
  return `${mins}m ${secs}s`
}
</script>

<template>
  <div v-if="props.result" class="space-y-4">
    <div class="flex gap-2 items-center">
      <Badge :variant="isSuccess ? 'default' : 'destructive'">
        {{ isSuccess ? 'Completed' : 'Failed' }}
      </Badge>
      <span class="text-sm text-muted-foreground">
        {{ formatDuration(durationSeconds) }}
      </span>
    </div>

    <Card>
      <CardContent class="pt-4 space-y-2">
        <div class="text-sm gap-4 grid grid-cols-2">
          <div>
            <span class="text-muted-foreground">Rows processed:</span>
            <span class="font-medium ml-2">{{ props.result.processedRows.toLocaleString() }}</span>
          </div>
          <div v-if="props.result.skippedRows > 0">
            <span class="text-muted-foreground">Rows skipped:</span>
            <span class="font-medium ml-2">{{ props.result.skippedRows.toLocaleString() }}</span>
          </div>
          <div v-if="props.result.outputSizeBytes">
            <span class="text-muted-foreground">File size:</span>
            <span class="font-medium ml-2">{{ formatFileSize(props.result.outputSizeBytes) }}</span>
          </div>
          <div v-if="props.result.errorCount > 0">
            <span class="text-muted-foreground">Errors:</span>
            <span class="text-destructive font-medium ml-2">{{ props.result.errorCount }}</span>
          </div>
        </div>
      </CardContent>
    </Card>

    <div v-if="props.result.errors.length > 0" class="space-y-2">
      <div class="text-sm text-destructive font-medium">
        Error Log
      </div>
      <Card>
        <CardContent class="pt-4">
          <div class="text-sm max-h-32 overflow-auto space-y-1">
            <div v-for="error in props.result.errors.slice(0, 10)" :key="error.message">
              <span v-if="error.rowNumber" class="text-muted-foreground">Row {{ error.rowNumber }}:</span>
              <span class="ml-2">{{ error.message }}</span>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>

    <div class="flex gap-2 justify-end">
      <Button v-if="props.outputPath" variant="outline" @click="emit('openFile')">
        Open File
      </Button>
      <Button v-if="props.outputPath" variant="outline" @click="emit('openFolder')">
        Open Folder
      </Button>
      <Button variant="outline" @click="emit('viewTable')">
        View Table
      </Button>
      <Button @click="emit('again')">
        {{ props.outputPath ? 'Export Again' : 'Import Again' }}
      </Button>
    </div>
  </div>
</template>
