<script setup lang="ts">
import type { RadioGroupItemProps } from 'radix-vue'
import { RadioGroupIndicator, RadioGroupItem } from 'radix-vue'
import { cn } from '@/lib/utils'

type Props = {
  class?: string
} & RadioGroupItemProps

const props = defineProps<Props>()
</script>

<template>
  <RadioGroupItem
    v-bind="props"
    :class="cn(
      'aspect-square h-4 w-4 rounded-full border border-primary text-primary ring-offset-background focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50',
      props.class,
    )"
  >
    <RadioGroupIndicator
      :class="cn(
        'flex items-center justify-center',
      )"
    >
      <svg
        class="text-current h-2.5 w-2.5 fill-current"
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="currentColor"
      >
        <circle cx="12" cy="12" r="10" />
      </svg>
    </RadioGroupIndicator>
  </RadioGroupItem>
</template>
