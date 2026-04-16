<script setup lang="ts">
import { computed } from 'vue'

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
    <DialogContent class="max-w-md">
      <DialogTitle class="flex items-center justify-between">
        <span>Tasks</span>
        <Button
          v-if="transferStore.runningTasks.some(t => t.status !== 'running')"
          variant="ghost"
          size="sm"
          @click="handleClearCompleted"
        >
          Clear Completed
        </Button>
      </DialogTitle>

      <div class="mt-4 max-h-400px overflow-auto space-y-4">
        <div v-if="transferStore.runningTasks.length === 0" class="text-muted-foreground py-8 text-center">
          No transfer tasks
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
