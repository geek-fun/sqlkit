<!--
  Visual Role: Outer container for transfer steps.
  Provides a clean, compact header with optional icon/step number and hairline borders.
-->
<script setup lang="ts">
import { computed } from 'vue'
import { Card, CardContent } from '@/components/ui/card'

const props = defineProps<{
  title: string
  stepNumber?: number
  icon?: string
  iconClass?: string
  summary?: string
  variant?: 'default' | 'highlight' | 'ghost'
  minHeight?: string
}>()

const cardClasses = computed(() => {
  const baseClasses = {
    default: 'border-border/40 shadow-none',
    highlight: 'border-primary/40 bg-primary/[0.03] shadow-none',
    ghost: 'border-dashed border-border/60 bg-transparent shadow-none',
  }
  return baseClasses[props.variant || 'default']
})

const stepLabel = computed(() =>
  props.stepNumber ? String(props.stepNumber).padStart(2, '0') : null,
)
</script>

<template>
  <Card
    class="rounded-md transition-all duration-200 overflow-hidden flex flex-col h-full"
    :class="cardClasses"
    :style="{ minHeight }"
  >
    <div class="px-3 py-2 border-b border-border/40 flex gap-2.5 items-center">
      <span v-if="icon" class="text-muted-foreground h-3.5 w-3.5" :class="[icon, iconClass]" />
      <span v-if="stepLabel" class="text-[10px] text-muted-foreground tracking-wide font-mono px-1.5 rounded-sm bg-muted">
        {{ stepLabel }}
      </span>
      <span class="text-xs text-foreground tracking-wide font-medium flex-1 uppercase">{{ title }}</span>
      <span v-if="summary" class="text-[11px] text-muted-foreground font-mono">
        {{ summary }}
      </span>
    </div>
    <CardContent class="px-3 pb-3 pt-2.5 flex flex-col flex-1">
      <slot />
    </CardContent>
  </Card>
</template>
