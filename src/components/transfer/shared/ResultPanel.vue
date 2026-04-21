<script setup lang="ts">
import type { TransferResult } from '@/types/transfer'

import { computed } from 'vue'
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
  <div v-if="props.result" class="flex flex-col space-y-6">
    <!-- Status Header -->
    <div class="py-8 text-center border rounded-lg bg-card flex flex-col shadow-sm items-center justify-center space-y-3">
      <div
        class="rounded-full flex h-12 w-12 items-center justify-center"
        :class="isSuccess ? 'bg-green-500/10 text-green-600' : 'bg-destructive/10 text-destructive'"
      >
        <span v-if="isSuccess" class="i-carbon-checkmark-filled h-6 w-6" />
        <span v-else class="i-carbon-warning-filled h-6 w-6" />
      </div>
      <div class="space-y-1">
        <h3 class="text-lg tracking-tight font-semibold">
          {{ isSuccess ? 'Transfer Completed' : 'Transfer Failed' }}
        </h3>
        <p class="text-sm text-muted-foreground">
          Operation finished in {{ formatDuration(durationSeconds) }}
        </p>
      </div>
    </div>

    <!-- Statistics Grid -->
    <div class="gap-4 grid grid-cols-2 sm:grid-cols-4">
      <Card class="bg-muted/30 shadow-none">
        <CardContent class="p-4 flex flex-col justify-center">
          <span class="text-xs text-muted-foreground tracking-wider font-medium uppercase">Processed</span>
          <span class="text-2xl font-semibold mt-1">{{ props.result.processedRows.toLocaleString() }}</span>
        </CardContent>
      </Card>

      <Card v-if="props.result.skippedRows > 0" class="bg-muted/30 shadow-none">
        <CardContent class="p-4 flex flex-col justify-center">
          <span class="text-xs text-muted-foreground tracking-wider font-medium uppercase">Skipped</span>
          <span class="text-2xl font-semibold mt-1">{{ props.result.skippedRows.toLocaleString() }}</span>
        </CardContent>
      </Card>

      <Card v-if="props.result.outputSizeBytes" class="bg-muted/30 shadow-none">
        <CardContent class="p-4 flex flex-col justify-center">
          <span class="text-xs text-muted-foreground tracking-wider font-medium uppercase">File Size</span>
          <span class="text-2xl font-semibold mt-1">{{ formatFileSize(props.result.outputSizeBytes) }}</span>
        </CardContent>
      </Card>

      <Card v-if="props.result.errorCount > 0" class="border-destructive/20 bg-destructive/5 shadow-none">
        <CardContent class="p-4 flex flex-col justify-center">
          <span class="text-xs text-destructive/80 tracking-wider font-medium uppercase">Errors</span>
          <span class="text-2xl text-destructive font-semibold mt-1">{{ props.result.errorCount.toLocaleString() }}</span>
        </CardContent>
      </Card>
    </div>

    <!-- Error Log -->
    <div v-if="props.result.errors.length > 0" class="space-y-2">
      <h4 class="text-sm text-destructive font-medium">
        Error Log
      </h4>
      <Card class="border-destructive/20 shadow-none">
        <CardContent class="p-0">
          <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border text-sm p-4 max-h-48 overflow-auto">
            <div
              v-for="(error, index) in props.result.errors.slice(0, 10)"
              :key="index"
              class="mb-2 flex items-start last:mb-0"
            >
              <span v-if="error.rowNumber" class="text-xs text-destructive font-medium mr-2 mt-0.5 px-1.5 py-0.5 rounded bg-destructive/10 inline-flex items-center">
                Row {{ error.rowNumber }}
              </span>
              <span class="text-muted-foreground">{{ error.message }}</span>
            </div>
            <div v-if="props.result.errors.length > 10" class="text-xs text-muted-foreground mt-2 italic">
              And {{ props.result.errors.length - 10 }} more errors...
            </div>
          </div>
        </CardContent>
      </Card>
    </div>

    <!-- Actions -->
    <div class="pt-4 border-t flex flex-wrap gap-3 items-center justify-end">
      <Button v-if="props.outputPath" variant="outline" @click="emit('openFile')">
        <span class="i-carbon-document mr-2" /> Open File
      </Button>
      <Button v-if="props.outputPath" variant="outline" @click="emit('openFolder')">
        <span class="i-carbon-folder mr-2" /> Open Folder
      </Button>
      <Button variant="outline" @click="emit('viewTable')">
        <span class="i-carbon-data-table mr-2" /> View Table
      </Button>
      <Button @click="emit('again')">
        <span class="i-carbon-reset mr-2" /> {{ props.outputPath ? 'Export Again' : 'Import Again' }}
      </Button>
    </div>
  </div>
</template>
