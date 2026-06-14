<script setup lang="ts">
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useTransferStore } from '@/store/transferStore'
import { useAppStore, LanguageType } from '@/store/appStore'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import ThemeToggle from './ThemeToggle.vue'

defineProps<{
  hideAiButton?: boolean
}>()

const emit = defineEmits<{
  toggleAi: []
  toggleTaskManager: []
}>()

const router = useRouter()
const transferStore = useTransferStore()
const appStore = useAppStore()
const { taskCount } = storeToRefs(transferStore)
const { languageType } = storeToRefs(appStore)

function openSettings() {
  router.push('/settings')
}

function openGitHub() {
  openUrl('https://github.com/geek-fun/sqlkit')
}

function toggleLanguage() {
  const next = languageType.value === LanguageType.ZH_CN ? LanguageType.EN_US : LanguageType.ZH_CN
  appStore.setLanguageType(next)
  localStorage.setItem('lang', next)
  window.location.reload()
}
</script>

<template>
  <div class="h-10 flex items-center gap-1 px-2 border-b bg-muted/30 shrink-0 overflow-hidden">
    <div class="flex-1" />

    <TooltipProvider>
      <!-- AI Assistant toggle -->
      <Tooltip v-if="!hideAiButton">
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="h-8 w-8" @click="emit('toggleAi')">
            <span class="i-carbon-chat-bot h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{{ $t('pages.settings.ai.featureRouting.sidebarAssistant.name') }}</TooltipContent>
      </Tooltip>

      <!-- Task Manager toggle -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="relative h-8 w-8" @click="emit('toggleTaskManager')">
            <span class="i-carbon-list-boxes h-4 w-4" />
            <span
              v-if="taskCount > 0"
              class="text-[9px] text-white font-bold px-1 rounded-full bg-red-500 flex h-4 min-w-4 items-center justify-center absolute -right-0.5 -top-0.5"
            >
              {{ taskCount > 9 ? '9+' : taskCount }}
            </span>
          </Button>
        </TooltipTrigger>
        <TooltipContent>{{ $t('transfer.tasks.title') }}</TooltipContent>
      </Tooltip>

      <!-- Theme -->
      <ThemeToggle />

      <!-- Language switch -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="h-8 w-8" @click="toggleLanguage">
            <span class="i-carbon-language h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{{ languageType === LanguageType.ZH_CN ? 'English' : '中文' }}</TooltipContent>
      </Tooltip>

      <!-- GitHub -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="h-8 w-8" @click="openGitHub">
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 0C5.37 0 0 5.37 0 12c0 5.3 3.438 9.8 8.205 11.387.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61-.546-1.387-1.333-1.756-1.333-1.756-1.09-.745.083-.729.083-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 21.795 24 17.295 24 12 24 5.37 18.627 0 12 0z" />
            </svg>
          </Button>
        </TooltipTrigger>
        <TooltipContent>GitHub</TooltipContent>
      </Tooltip>

      <!-- Settings -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="h-8 w-8" @click="openSettings">
            <span class="i-carbon-settings h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{{ $t('pages.settings.title') }}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  </div>
</template>
