<script setup lang="ts">
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'

type Props = {
  text: string
  label?: string
  side?: 'top' | 'right' | 'bottom' | 'left'
}

withDefaults(defineProps<Props>(), {
  label: undefined,
  side: 'top',
})
</script>

<template>
  <TooltipProvider :delay-duration="200">
    <Tooltip>
      <TooltipTrigger as-child>
        <button
          type="button"
          data-info-tip="true"
          :aria-label="label ?? text"
          class="text-muted-foreground rounded-sm inline-flex shrink-0 h-5 w-5 cursor-pointer transition-colors items-center justify-center hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background"
        >
          <span class="i-lucide-info h-3.5 w-3.5" aria-hidden="true" />
        </button>
      </TooltipTrigger>
      <TooltipContent :side="side" class="text-xs leading-relaxed font-normal max-w-xs">
        {{ text }}
      </TooltipContent>
    </Tooltip>
  </TooltipProvider>
</template>
