<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useTransferStore } from '@/store/transferStore'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'

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
  <div class="h-10 flex items-center gap-1 px-2 border-b bg-muted/30 shrink-0 overflow-hidden pl-17.5" data-tauri-drag-region>
    <span class="text-xs font-semibold text-muted-foreground select-none">SqlKit</span>
    <div class="flex-1" />

    <TooltipProvider>
      <Tooltip v-if="!hideAiButton">
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="h-8 w-8" @click="emit('toggleAi')">
            <span class="i-carbon-chat-bot h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{{ $t('pages.settings.ai.featureRouting.sidebarAssistant.name') }}</TooltipContent>
      </Tooltip>

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
    </TooltipProvider>
  </div>
</template>
