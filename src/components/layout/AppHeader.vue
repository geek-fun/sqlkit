<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { computed } from 'vue'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { usePlatform } from '@/composables/usePlatform'
import { shouldReserveMacTrafficLightInset, useWindowControls } from '@/composables/useWindowControls'
import { useTransferStore } from '@/store/transferStore'
import WindowControls from './WindowControls.vue'

defineProps<{
  hideAiButton?: boolean
}>()

const emit = defineEmits<{
  toggleAi: []
  toggleTaskManager: []
}>()

const transferStore = useTransferStore()
const { taskCount } = storeToRefs(transferStore)

const windowControls = useWindowControls()
const { isMac, platformReady } = usePlatform()

const trafficLightInset = computed(() => {
  if (!platformReady.value)
    return '78px'
  return shouldReserveMacTrafficLightInset(isMac.value, windowControls.isFullscreen.value, true) ? '78px' : '0px'
})
</script>

<template>
  <div
    class="pr-2 border-b bg-muted/30 flex shrink-0 gap-1 h-10 items-center overflow-hidden"
    :style="{ paddingLeft: trafficLightInset }"
    data-tauri-drag-region
  >
    <WindowControls />
    <span class="text-xs text-muted-foreground font-semibold select-none">SqlKit</span>
    <div class="flex-1" data-tauri-drag-region />

    <TooltipProvider>
      <Tooltip v-if="!hideAiButton">
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="h-8 w-8" @click="emit('toggleAi')">
            <span class="i-carbon-chat-bot h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{{ $t('aside.aiAssistant') }}</TooltipContent>
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
