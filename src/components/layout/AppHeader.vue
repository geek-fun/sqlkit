<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { useTransferStore } from '@/store/transferStore'

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
  <div class="pl-[90px] pr-2 border-b bg-muted/30 flex shrink-0 gap-1 h-10 items-center overflow-hidden" data-tauri-drag-region>
    <span class="text-xs text-muted-foreground font-semibold select-none">SqlKit</span>
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
          <Button variant="ghost" size="icon" class="h-8 w-8 relative" @click="emit('toggleTaskManager')">
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
