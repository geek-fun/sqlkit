<script setup lang="ts">
import { ref, watch } from 'vue'
import TaskManagerPanel from '@/components/transfer/tasks/TaskManagerPanel.vue'
import AiAssistantSidebar from './AiAssistantSidebar.vue'
import AppHeader from './AppHeader.vue'
import AppSidebar from './AppSidebar.vue'

const props = defineProps<{
  hideAiButton?: boolean
}>()

const STORAGE_KEY = 'sqlkit-ai-panel-open'

const showAiPanel = ref(localStorage.getItem(STORAGE_KEY) === 'true')
const showTaskManager = ref(false)

watch(showAiPanel, (val) => {
  localStorage.setItem(STORAGE_KEY, String(val))
})

function toggleAi() {
  showAiPanel.value = !showAiPanel.value
}

function toggleTaskManager() {
  showTaskManager.value = !showTaskManager.value
}
</script>

<template>
  <div class="h-screen flex flex-col overflow-hidden">
    <AppHeader
      :hide-ai-button="props.hideAiButton ?? false"
      @toggle-ai="toggleAi"
      @toggle-task-manager="toggleTaskManager"
    />
    <div class="flex flex-1 min-h-0">
      <AppSidebar />
      <main class="flex flex-1 min-w-0 overflow-hidden">
        <div class="flex-1 overflow-auto">
          <slot />
        </div>
        <AiAssistantSidebar
          v-if="showAiPanel"
          :open="showAiPanel"
          @close="showAiPanel = false"
        />
      </main>
    </div>
    <TaskManagerPanel v-model:open="showTaskManager" />
  </div>
</template>
