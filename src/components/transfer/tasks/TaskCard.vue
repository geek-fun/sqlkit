<script setup lang="ts">
import type { BackgroundTask } from '@/types/transfer'

import { computed } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'

import { ProgressBar } from '@/components/ui/progress'

const props = defineProps<{
  task: BackgroundTask
}>()

const emit = defineEmits<{
  goToTask: []
  dismiss: []
}>()

const statusColor = computed(() => {
  switch (props.task.status) {
    case 'running': return 'default'
    case 'completed': return 'outline'
    case 'failed': return 'destructive'
    default: return 'outline'
  }
})

const statusLabel = computed(() => {
  switch (props.task.status) {
    case 'running': return 'Running'
    case 'completed': return 'Completed'
    case 'failed': return 'Failed'
    default: return 'Pending'
  }
})

const progressPercent = computed(() =>
  props.task.progress.total > 0
    ? (props.task.progress.complete / props.task.progress.total) * 100
    : 0,
)

const icon = computed(() => {
  switch (props.task.kind) {
    case 'export': return 'i-carbon-document-export text-blue-500/80'
    case 'import': return 'i-carbon-document-import text-purple-500/80'
    default: return 'i-carbon-data-refinery text-orange-500/80'
  }
})

const timeAgo = computed(() => {
  const start = new Date(props.task.startTime)
  const now = new Date()
  const diffMs = now.getTime() - start.getTime()
  const diffSec = Math.floor(diffMs / 1000)
  if (diffSec < 60)
    return `${diffSec}s ago`
  const diffMin = Math.floor(diffSec / 60)
  if (diffMin < 60)
    return `${diffMin}m ago`
  const diffHour = Math.floor(diffMin / 60)
  return `${diffHour}h ago`
})
</script>

<template>
  <Card class="p-3 border-border/40 shadow-sm transition-all hover:border-border/60">
    <div class="flex flex-col space-y-2.5">
      <div class="flex items-center justify-between">
        <div class="flex gap-2 items-center">
          <span :class="icon" class="h-3.5 w-3.5" />
          <span class="text-xs font-medium">{{ props.task.label }}</span>
        </div>
        <Badge
          :variant="statusColor"
          class="text-[10px] tracking-wide font-mono px-1.5 py-0.5 rounded-sm uppercase" :class="[
            props.task.status === 'completed' ? 'bg-green-500/10 text-green-600 border-transparent' : '',
          ]"
        >
          {{ statusLabel }}
        </Badge>
      </div>

      <ProgressBar
        v-if="props.task.status === 'running'"
        :value="progressPercent"
        class="bg-muted h-1"
      />

      <div class="text-[11px] text-muted-foreground font-mono flex items-center justify-between tabular-nums">
        <span>
          {{ props.task.runtime.complete.toLocaleString() }}
          <span v-if="props.task.runtime.total > 0">
            / {{ props.task.runtime.total.toLocaleString() }} rows
          </span>
        </span>
        <span>{{ timeAgo }}</span>
      </div>

      <div v-if="props.task.error" class="text-[11px] text-destructive">
        {{ props.task.error }}
      </div>

      <div class="pt-1 flex gap-2 items-center justify-end">
        <Button
          v-if="props.task.status !== 'running'"
          variant="ghost"
          size="sm"
          class="text-[11px] text-muted-foreground tracking-wide px-2.5 h-7 uppercase"
          @click="emit('dismiss')"
        >
          Dismiss
        </Button>
        <Button
          variant="outline"
          size="sm"
          class="text-[11px] px-2.5 border-border/40 flex gap-1.5 h-7"
          @click="emit('goToTask')"
        >
          <span class="i-carbon-arrow-right h-3 w-3" />
          <span>View</span>
        </Button>
      </div>
    </div>
  </Card>
</template>
