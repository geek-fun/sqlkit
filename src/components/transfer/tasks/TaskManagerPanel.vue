<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'

import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'

import { useTransferStore } from '@/store/transferStore'
import TaskCard from './TaskCard.vue'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const { t } = useI18n()
const transferStore = useTransferStore()
const router = useRouter()

const isOpen = computed({
  get: () => props.open,
  set: val => emit('update:open', val),
})

function handleClearCompleted() {
  transferStore.clearCompletedTasks()
}

function handleGoToTask(task: any) {
  transferStore.openTask(task.id)
  isOpen.value = false
  router.push({
    path: '/transfer',
    query: { tab: task.kind, taskId: task.id },
  })
}

function handleDismiss(taskId: string) {
  transferStore.removeTask(taskId)
}
</script>

<template>
  <Dialog v-model:open="isOpen">
    <DialogContent class="p-4 border-border/40 flex flex-col max-h-[80vh] sm:max-w-[520px]">
      <DialogTitle class="flex items-center justify-between">
        <div class="flex gap-2 items-center">
          <span class="i-carbon-task text-muted-foreground h-4 w-4" />
          <span class="text-sm tracking-wide font-semibold uppercase">{{ t('transfer.tasks.title') }}</span>
        </div>
        <Button
          v-if="transferStore.runningTasks.some(t => t.status !== 'running')"
          variant="ghost"
          size="sm"
          class="text-[11px] tracking-wide h-8 uppercase"
          @click="handleClearCompleted"
        >
          {{ t('transfer.tasks.clearCompleted') }}
        </Button>
      </DialogTitle>

      <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border mt-2 pr-2 flex-1 overflow-y-auto space-y-2">
        <div v-if="transferStore.runningTasks.length === 0" class="text-muted-foreground py-10 flex flex-col gap-3 items-center justify-center">
          <div class="p-3 border border-border/40 rounded-full bg-muted/30">
            <span class="i-carbon-task-complete opacity-40 h-6 w-6 block" />
          </div>
          <div class="flex flex-col gap-1 items-center">
            <span class="text-xs text-foreground/60 font-medium">{{ t('transfer.tasks.noTasks') }}</span>
            <span class="text-[11px] text-muted-foreground/60">{{ t('transfer.tasks.emptyDescription') }}</span>
          </div>
        </div>

        <TaskCard
          v-for="task in transferStore.runningTasks"
          :key="task.id"
          :task="task"
          @go-to-task="handleGoToTask(task)"
          @dismiss="handleDismiss(task.id)"
        />
      </div>
    </DialogContent>
  </Dialog>
</template>
