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
}>()

const cardClasses = computed(() => {
  switch (props.variant) {
    case 'highlight':
      return 'border-primary/20 bg-primary/5 shadow-sm'
    case 'ghost':
      return 'border-dashed bg-transparent shadow-none'
    default:
      return 'shadow-sm'
  }
})

const stepLabel = computed(() =>
  props.stepNumber ? String(props.stepNumber).padStart(2, '0') : null,
)
</script>

<template>
  <Card class="transition-all duration-200 overflow-hidden" :class="cardClasses">
    <div class="px-4 py-2.5 border-b border-border/40 flex items-center gap-3">
      <span v-if="stepLabel" class="text-[10px] font-bold tracking-widest text-muted-foreground uppercase">
        STEP {{ stepLabel }}
      </span>
      <span class="text-[10px] font-bold tracking-widest text-foreground uppercase">{{ title }}</span>
      <span v-if="summary" class="text-xs text-muted-foreground font-normal px-2 py-0.5 rounded bg-muted">
        {{ summary }}
      </span>
    </div>
    <CardContent class="px-4 pb-4 pt-3">
      <slot />
    </CardContent>
  </Card>
</template>
