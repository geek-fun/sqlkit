<!--
  Visual Role: Active transfer monitor.
  Shows a thin progress bar, inline phase indicators, and monospaced time/row estimates.
-->
<script setup lang="ts">
import type { TransferProgress } from '@/types/transfer'

import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n()

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
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <div role="status" aria-live="polite" class="text-xs text-foreground font-medium flex gap-1.5 items-center">
        <span class="i-carbon-circle-dash text-primary h-3.5 w-3.5" :class="{ 'animate-spin': props.isRunning }" />
        <span>{{ props.progress?.phase || t('transfer.progress.preparing') }}</span>
      </div>
      <div class="text-[11px] text-muted-foreground font-mono tabular-nums" aria-live="off">
        {{ formatTime(elapsedSeconds) }} elapsed
        <span v-if="remainingSeconds > 0" class="text-muted-foreground/70 ml-1.5">
          ~{{ formatTime(remainingSeconds) }} left
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
      <ProgressBar :value="percent" class="bg-muted/40 h-1" />
    </div>

    <div class="text-xs font-mono flex items-center justify-between tabular-nums" aria-hidden="true">
      <div>
        <span class="text-foreground font-medium">{{ processedRows.toLocaleString() }}</span>
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
      class="text-[11px] text-destructive tracking-wide font-medium px-2 py-1 rounded-sm bg-destructive/10 flex gap-1.5 uppercase items-center"
      role="alert"
      aria-live="assertive"
    >
      <span class="i-carbon-warning-alt h-3 w-3" />
      {{ props.progress.errorCount }} errors occurred
    </div>

    <div class="pt-2 flex gap-2 justify-end">
      <Button
        variant="ghost"
        size="sm"
        class="text-xs text-muted-foreground h-8 hover:text-foreground"
        :disabled="!props.isRunning"
        @click="emit('runInBackground')"
      >
        {{ t('transfer.progress.runInBackground') }}
      </Button>
      <Button
        variant="outline"
        size="sm"
        class="text-xs text-destructive border-destructive/30 h-8 hover:text-destructive hover:bg-destructive/10"
        :disabled="!props.isRunning"
        @click="emit('cancel')"
      >
        {{ t('transfer.progress.cancel') }}
      </Button>
    </div>
  </div>
</template>
