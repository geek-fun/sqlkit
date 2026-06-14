<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useTransferStore } from '@/store/transferStore'
import ThemeToggle from './ThemeToggle.vue'

defineProps<{
  hideAiButton?: boolean
}>()

const emit = defineEmits<{
  toggleAi: []
  toggleTaskManager: []
}>()

const transferStore = useTransferStore()
const { taskCount } = storeToRefs(transferStore)
</script>

<template>
  <header class="border-b bg-background/95 w-full top-0 sticky z-40 backdrop-blur supports-[backdrop-filter]:bg-background/60">
    <div class="container flex h-14 items-center">
      <div class="flex flex-1 items-center justify-between">
        <div class="flex gap-2 items-center">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="h-6 w-6"
          >
            <ellipse cx="12" cy="5" rx="9" ry="3" />
            <path d="M3 5v14a9 3 0 0 0 18 0V5" />
          </svg>
          <span class="text-xl font-bold">SQLKit</span>
        </div>
        <div class="flex gap-1 items-center">
          <!-- AI Assistant toggle -->
          <button
            v-if="!hideAiButton"
            class="text-muted-foreground rounded-lg inline-flex h-8 w-8 transition-colors items-center justify-center hover:text-foreground hover:bg-muted"
            :title="$t('pages.settings.ai.featureRouting.sidebarAssistant.name')"
            @click="emit('toggleAi')"
          >
            <span class="i-carbon-chat-bot h-5 w-5" />
          </button>

          <!-- Task Manager toggle -->
          <button
            class="text-muted-foreground rounded-lg inline-flex h-8 w-8 transition-colors items-center justify-center relative hover:text-foreground hover:bg-muted"
            :title="$t('transfer.tasks.title')"
            @click="emit('toggleTaskManager')"
          >
            <span class="i-carbon-list-boxes h-5 w-5" />
            <span
              v-if="taskCount > 0"
              class="text-[9px] text-white font-bold px-1 rounded-full bg-primary flex h-[14px] min-w-[14px] items-center justify-center absolute -right-0.5 -top-0.5"
            >
              {{ taskCount > 9 ? '9+' : taskCount }}
            </span>
          </button>

          <ThemeToggle />
        </div>
      </div>
    </div>
  </header>
</template>
