<script setup lang="ts">
import type { RadioGroupRootEmits, RadioGroupRootProps } from 'radix-vue'
import { RadioGroupRoot, useForwardPropsEmits } from 'radix-vue'
import { cn } from '@/lib/utils'

type Props = {
  class?: string
} & RadioGroupRootProps

const props = defineProps<Props>()
const emits = defineEmits<RadioGroupRootEmits>()

const forwarded = useForwardPropsEmits(props, emits)
</script>

<template>
  <RadioGroupRoot
    v-bind="forwarded"
    :class="cn('grid gap-2', props.class)"
  >
    <slot />
  </RadioGroupRoot>
</template>
