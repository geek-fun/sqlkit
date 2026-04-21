<script setup lang="ts">
import type { TransferProgress } from '@/types/transfer'

import { computed } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'

import { ProgressBar } from '@/components/ui/progress'

const props = defineProps<{
  progress?: TransferProgress | null
  isRunning: boolean
}>()

const emit = defineEmits<{
  cancel: []
  runInBackground: []
}>()

const percent = computed(() => props.progress?.percent ?? 0)

const processedRows = computed(() => props.progress?.processedRows ?? 0)

const totalRows = computed(() => props.progress?.totalRows ?? 0)

const elapsedSeconds = computed(() =>
  Math.floor((props.progress?.elapsedMs ?? 0) / 1000),
)

const remainingSeconds = computed(() =>
  Math.floor((props.progress?.estimatedRemainingMs ?? 0) / 1000),
)

function formatTime(seconds: number) {
  if (seconds < 60)
    return `${seconds}s`
  const mins = Math.floor(seconds / 60)
  const secs = seconds % 60
  return `${mins}m ${secs}s`
}
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <div role="status" aria-live="polite">
        <Badge variant="outline">
          <span class="sr-only">Current phase: </span>
          {{ props.progress?.phase || 'Preparing' }}
        </Badge>
      </div>
      <div class="text-sm text-muted-foreground" aria-live="off">
        {{ formatTime(elapsedSeconds) }} elapsed
        <span v-if="remainingSeconds > 0" class="ml-2">
          ~{{ formatTime(remainingSeconds) }} remaining
        </span>
      </div>
    </div>

    <div
      role="progressbar"
      :aria-valuenow="Math.round(percent)"
      aria-valuemin="0"
      aria-valuemax="100"
      :aria-label="`Transfer progress: ${Math.round(percent)}%`"
      class="w-full"
    >
      <ProgressBar :value="percent" class="h-2" />
    </div>

    <div class="text-sm flex items-center justify-between" aria-hidden="true">
      <div>
        <span class="font-medium">{{ processedRows.toLocaleString() }}</span>
        <span v-if="totalRows > 0" class="text-muted-foreground">
          / {{ totalRows.toLocaleString() }} rows
        </span>
      </div>
      <div class="text-muted-foreground">
        {{ Math.round(percent) }}%
      </div>
    </div>

    <div
      v-if="props.progress && props.progress.errorCount > 0"
      class="text-sm text-destructive"
      role="alert"
      aria-live="assertive"
    >
      {{ props.progress.errorCount }} errors occurred during transfer.
    </div>

    <div class="flex gap-2 justify-end">
      <Button
        variant="outline"
        :disabled="!props.isRunning"
        @click="emit('runInBackground')"
      >
        Run in Background
      </Button>
      <Button
        variant="destructive"
        :disabled="!props.isRunning"
        @click="emit('cancel')"
      >
        Cancel
      </Button>
    </div>
  </div>
</template>
