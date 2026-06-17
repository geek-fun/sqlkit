<script setup lang="ts">
import type { BackgroundTask } from '@/types/transfer'
import { onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import TaskCard from '@/components/transfer/tasks/TaskCard.vue'
import { useTransferStore } from '@/store/transferStore'

const emit = defineEmits<{
  close: []
}>()

const { t } = useI18n()
const router = useRouter()
const transferStore = useTransferStore()

const MIN_WIDTH = 320
const MAX_WIDTH = 600
const DEFAULT_WIDTH = 420

const currentWidth = ref(DEFAULT_WIDTH)
const isResizing = ref(false)

function startResize(e: MouseEvent) {
  isResizing.value = true
  document.addEventListener('mousemove', onResize)
  document.addEventListener('mouseup', stopResize)
  e.preventDefault()
}

function onResize(e: MouseEvent) {
  if (!isResizing.value)
    return
  const newWidth = window.innerWidth - e.clientX - 48
  currentWidth.value = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, newWidth))
}

function stopResize() {
  isResizing.value = false
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
}

function handleGoToTask(task: BackgroundTask) {
  transferStore.openTask(task.id)
  emit('close')
  router.push({
    path: '/transfer',
    query: { tab: task.kind, taskId: task.id },
  })
}

function handleDismiss(taskId: string) {
  transferStore.removeTask(taskId)
}

function hasDismissable() {
  return transferStore.runningTasks.some(t => t.status === 'completed' || t.status === 'failed')
}

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
      <div class="header-row">
        <div class="flex gap-2 items-center">
          <span class="i-carbon-task text-muted-foreground h-4 w-4" />
          <span class="header-title">{{ $t('aside.tasks') }}</span>
        </div>
        <div class="header-actions">
          <button
            v-if="hasDismissable()"
            class="header-icon-btn"
            @click="transferStore.clearCompletedTasks()"
          >
            <span class="i-carbon-checkmark h-4 w-4" />
          </button>
          <button
            class="header-icon-btn"
            @click="emit('close')"
          >
            <span class="i-carbon-close h-4 w-4" />
          </button>
        </div>
      </div>

      <!-- Task list -->
      <div class="p-4 flex-1 overflow-y-auto space-y-2">
        <div
          v-if="transferStore.runningTasks.length === 0"
          class="text-muted-foreground py-10 flex flex-col gap-3 items-center justify-center"
        >
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

.header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 12px 12px 16px;
  border-bottom: 1px solid hsl(var(--border));
}

.header-title {
  font-size: 16px;
  font-weight: 700;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.header-icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: all 0.15s;
}

.header-icon-btn:hover {
  background: hsl(var(--muted));
  color: hsl(var(--foreground));
}
</style>
