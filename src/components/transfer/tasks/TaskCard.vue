<script setup lang="ts">
import type { BackgroundTask } from '@/types/transfer'

import { computed } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'

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
    case 'completed': return 'default'
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
    case 'export': return '📤'
    case 'import': return '📥'
    default: return '🔄'
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
  <Card>
    <CardContent class="pt-4 space-y-3">
      <div class="flex items-center justify-between">
        <div class="flex gap-2 items-center">
          <span class="text-lg">{{ icon }}</span>
          <span class="text-sm font-medium">{{ props.task.label }}</span>
        </div>
        <Badge :variant="statusColor">
          {{ statusLabel }}
        </Badge>
      </div>

      <ProgressBar
        v-if="props.task.status === 'running'"
        :value="progressPercent"
        class="h-2"
      />

      <div class="text-xs text-muted-foreground flex items-center justify-between">
        <span>
          {{ props.task.runtime.complete.toLocaleString() }}
          <span v-if="props.task.runtime.total > 0">
            / {{ props.task.runtime.total.toLocaleString() }} rows
          </span>
        </span>
        <span>{{ timeAgo }}</span>
      </div>

      <div v-if="props.task.error" class="text-xs text-destructive">
        {{ props.task.error }}
      </div>

      <div class="flex gap-2 justify-end">
        <Button
          v-if="props.task.status !== 'running'"
          variant="ghost"
          size="sm"
          @click="emit('dismiss')"
        >
          Dismiss
        </Button>
        <Button
          variant="outline"
          size="sm"
          @click="emit('goToTask')"
        >
          Go to Task
        </Button>
      </div>
    </CardContent>
  </Card>
</template>
