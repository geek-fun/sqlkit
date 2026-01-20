<script setup lang="ts">
import type { SelectLabelProps } from 'radix-vue'
import type { HTMLAttributes } from 'vue'
import { SelectLabel } from 'radix-vue'
import { computed } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<SelectLabelProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = computed(() => {
  const { class: _, ...delegated } = props
  return delegated
})
</script>

<template>
  <SelectLabel
    v-bind="delegatedProps"
    :class="cn('py-1.5 pl-8 pr-2 text-sm font-semibold', props.class)"
  >
    <slot />
  </SelectLabel>
</template>
