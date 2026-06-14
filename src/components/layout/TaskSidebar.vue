<script setup lang="ts">
import { ref, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { useTransferStore } from '@/store/transferStore'
import TaskCard from '@/components/transfer/tasks/TaskCard.vue'

defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const router = useRouter()
const transferStore = useTransferStore()

const MIN_WIDTH = 320
const MAX_WIDTH = 600
const DEFAULT_WIDTH = 420

const currentWidth = ref(DEFAULT_WIDTH)
const isResizing = ref(false)

const startResize = (e: MouseEvent) => {
  isResizing.value = true
  document.addEventListener('mousemove', onResize)
  document.addEventListener('mouseup', stopResize)
  e.preventDefault()
}

const onResize = (e: MouseEvent) => {
  if (!isResizing.value)
    return
  const newWidth = window.innerWidth - e.clientX - 48
  currentWidth.value = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, newWidth))
}

const stopResize = () => {
  isResizing.value = false
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
}

const handleGoToTask = (task: any) => {
  transferStore.openTask(task.id)
  emit('close')
  router.push({
    path: '/transfer',
    query: { tab: task.kind, taskId: task.id },
  })
}

const handleDismiss = (taskId: string) => {
  transferStore.removeTask(taskId)
}

const hasDismissable = () =>
  transferStore.runningTasks.some(t => t.status !== 'running')

onUnmounted(() => {
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
})
</script>

<template>
  <div class="task-sidebar-resizable" :style="{ width: `${currentWidth}px` }">
    <div class="resize-handle" @mousedown="startResize" />

    <div class="flex flex-col h-full">
      <!-- Header -->
      <div class="px-4 py-3 border-b border-border flex items-center justify-between">
        <div class="flex gap-2 items-center">
          <span class="i-carbon-task text-muted-foreground h-4 w-4" />
          <span class="text-sm tracking-wide font-semibold">Tasks</span>
        </div>
        <div class="flex gap-1 items-center">
          <Button
            v-if="hasDismissable()"
            variant="ghost"
            size="sm"
            class="text-[11px] tracking-wide h-7"
            @click="transferStore.clearCompletedTasks()"
          >
            Clear
          </Button>
          <button
            class="text-muted-foreground rounded inline-flex h-7 w-7 transition-colors items-center justify-center hover:text-foreground hover:bg-muted"
            @click="emit('close')"
          >
            <span class="i-carbon-close h-4 w-4" />
          </button>
        </div>
      </div>

      <!-- Task list -->
      <div class="flex-1 overflow-y-auto p-4 space-y-2">
        <div
          v-if="transferStore.runningTasks.length === 0"
          class="text-muted-foreground py-10 flex flex-col gap-3 items-center justify-center"
        >
          <div class="p-3 border border-border/40 rounded-full bg-muted/30">
            <span class="i-carbon-task-complete opacity-40 h-6 w-6 block" />
          </div>
          <div class="flex flex-col gap-1 items-center">
            <span class="text-xs text-foreground/60 font-medium">No active tasks</span>
            <span class="text-[11px] text-muted-foreground/60">Export, import, and migration jobs will appear here</span>
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
    </div>
  </div>
</template>

<style scoped>
.task-sidebar-resizable {
  height: 100%;
  display: flex;
  flex-direction: column;
  border-left: 1px solid hsl(var(--border));
  position: relative;
  background: hsl(var(--background));
}

.resize-handle {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 4px;
  cursor: col-resize;
  z-index: 10;
  background: transparent;
  transition: background 0.15s;
}

.resize-handle:hover {
  background: hsl(var(--primary) / 0.3);
}
</style>
