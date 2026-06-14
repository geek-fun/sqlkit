<script setup lang="ts">
import { ref, watch } from 'vue'
import AiAssistantSidebar from './AiAssistantSidebar.vue'
import AppHeader from './AppHeader.vue'
import AppSidebar from './AppSidebar.vue'
import TaskSidebar from './TaskSidebar.vue'

const props = defineProps<{
  hideAiButton?: boolean
}>()

type SidePanel = 'none' | 'ai' | 'tasks'
const STORAGE_KEY = 'sqlkit-ai-panel-open'

const sidePanel = ref<SidePanel>(
  localStorage.getItem(STORAGE_KEY) === 'true' ? 'ai' : 'none',
)

watch(sidePanel, (val) => {
  localStorage.setItem(STORAGE_KEY, String(val === 'ai'))
})

function toggleAi() {
  sidePanel.value = sidePanel.value === 'ai' ? 'none' : 'ai'
}

function toggleTaskManager() {
  sidePanel.value = sidePanel.value === 'tasks' ? 'none' : 'tasks'
}
</script>

<template>
  <div class="flex flex-col h-screen overflow-hidden">
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
          v-if="sidePanel === 'ai'"
          @close="sidePanel = 'none'"
        />
        <TaskSidebar
          v-if="sidePanel === 'tasks'"
          @close="sidePanel = 'none'"
        />
      </main>
    </div>
  </div>
</template>
