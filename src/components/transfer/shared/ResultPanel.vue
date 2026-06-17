<!--
  Visual Role: Post-transfer summary panel.
  Presents completion status, compact stat cards, and scrollable error logs (if any).
-->
<script setup lang="ts">
import type { TransferResult } from '@/types/transfer'

import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n()

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
  <div v-if="props.result" class="flex flex-col space-y-4">
    <!-- Status Header: Compact horizontal row -->
    <div class="px-3 py-2 border border-border/40 rounded-md bg-card flex gap-3 shadow-none items-center">
      <div
        class="rounded-full flex h-6 w-6 items-center justify-center"
        :class="isSuccess ? 'bg-green-500/10 text-green-600' : 'bg-destructive/10 text-destructive'"
      >
        <span v-if="isSuccess" class="i-carbon-checkmark-filled h-3.5 w-3.5" />
        <span v-else class="i-carbon-warning-filled h-3.5 w-3.5" />
      </div>
      <div class="flex flex-1 items-center justify-between">
        <h3 class="text-sm tracking-wide font-medium">
          {{ isSuccess ? t('transfer.result.success') : t('transfer.result.failed') }}
        </h3>
        <p class="text-[11px] text-muted-foreground font-mono tabular-nums">
          {{ formatDuration(durationSeconds) }}
        </p>
      </div>
    </div>

    <!-- Statistics Grid -->
    <div class="gap-3 grid grid-cols-2 sm:grid-cols-4">
      <Card class="border-border/40 rounded-md bg-muted/20 shadow-none">
        <CardContent class="p-3 flex flex-col justify-center">
          <span class="text-[10px] text-muted-foreground tracking-widest font-medium uppercase">{{ t('transfer.result.processed') }}</span>
          <span class="text-lg font-mono font-semibold mt-0.5 tabular-nums">{{ props.result.processedRows.toLocaleString() }}</span>
        </CardContent>
      </Card>

      <Card v-if="props.result.skippedRows > 0" class="border-border/40 rounded-md bg-muted/20 shadow-none">
        <CardContent class="p-3 flex flex-col justify-center">
          <span class="text-[10px] text-muted-foreground tracking-widest font-medium uppercase">{{ t('transfer.result.skipped') }}</span>
          <span class="text-lg font-mono font-semibold mt-0.5 tabular-nums">{{ props.result.skippedRows.toLocaleString() }}</span>
        </CardContent>
      </Card>

      <Card v-if="props.result.outputSizeBytes" class="border-border/40 rounded-md bg-muted/20 shadow-none">
        <CardContent class="p-3 flex flex-col justify-center">
          <span class="text-[10px] text-muted-foreground tracking-widest font-medium uppercase">{{ t('transfer.import.fileSize') }}</span>
          <span class="text-lg font-mono font-semibold mt-0.5 tabular-nums">{{ formatFileSize(props.result.outputSizeBytes) }}</span>
        </CardContent>
      </Card>

      <Card v-if="props.result.errorCount > 0" class="border-destructive/20 rounded-md bg-destructive/5 shadow-none">
        <CardContent class="p-3 flex flex-col justify-center">
          <span class="text-[10px] text-destructive/80 tracking-widest font-medium uppercase">{{ t('transfer.result.errorCount') }}</span>
          <span class="text-lg text-destructive font-mono font-semibold mt-0.5 tabular-nums">{{ props.result.errorCount.toLocaleString() }}</span>
        </CardContent>
      </Card>
    </div>

    <!-- Error Log -->
    <div v-if="props.result.errors.length > 0" class="space-y-1.5">
      <div class="px-1 flex gap-2 items-center">
        <span class="i-carbon-list text-muted-foreground h-3.5 w-3.5" />
        <h4 class="text-[11px] text-muted-foreground tracking-wide font-medium uppercase">
          Log Details
        </h4>
      </div>
      <div class="border border-border/40 rounded-md bg-muted/40">
        <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border text-[11px] leading-snug font-mono p-3 max-h-40 overflow-auto">
          <div
            v-for="(error, index) in props.result.errors.slice(0, 10)"
            :key="index"
            class="mb-1.5 flex items-start last:mb-0"
          >
            <span v-if="error.rowNumber" class="text-destructive/80 mr-2 whitespace-nowrap">
              [Row {{ String(error.rowNumber).padStart(3, '0') }}]
            </span>
            <span class="text-muted-foreground">{{ error.message }}</span>
          </div>
          <div v-if="props.result.errors.length > 10" class="text-muted-foreground/60 mt-2 italic">
            ...and {{ props.result.errors.length - 10 }} more errors
          </div>
        </div>
      </div>
    </div>

    <!-- Actions -->
    <div class="pt-3 border-t border-border/40 flex flex-wrap gap-2 items-center justify-end">
      <Button v-if="props.outputPath" variant="outline" size="sm" class="text-xs h-8" @click="emit('openFile')">
        <span class="i-carbon-document mr-1.5" /> {{ t('transfer.result.openFile') }}
      </Button>
      <Button v-if="props.outputPath" variant="outline" size="sm" class="text-xs h-8" @click="emit('openFolder')">
        <span class="i-carbon-folder mr-1.5" /> {{ t('transfer.result.openFolder') }}
      </Button>
      <Button variant="outline" size="sm" class="text-xs h-8" @click="emit('viewTable')">
        <span class="i-carbon-data-table mr-1.5" /> {{ t('transfer.result.viewTable') }}
      </Button>
      <Button size="sm" class="text-xs h-8" @click="emit('again')">
        <span class="i-carbon-reset mr-1.5" /> {{ props.outputPath ? t('transfer.result.exportAgain') : t('transfer.result.importAgain') }}
      </Button>
    </div>
  </div>
</template>
