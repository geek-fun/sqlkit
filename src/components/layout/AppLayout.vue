<script setup lang="ts">
import { ref, watch } from 'vue'
import AiAssistantSidebar from './AiAssistantSidebar.vue'
import TaskSidebar from './TaskSidebar.vue'
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
  if (showAiPanel.value) showTaskManager.value = false
}

function toggleTaskManager() {
  showTaskManager.value = !showTaskManager.value
  if (showTaskManager.value) showAiPanel.value = false
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
        <TaskSidebar
          v-if="showTaskManager"
          :open="showTaskManager"
          @close="showTaskManager = false"
        />
      </main>
    </div>
  </div>
</template>
