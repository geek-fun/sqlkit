<script setup lang="ts">
import type { TabsListProps } from 'radix-vue'
import { TabsList } from 'radix-vue'
import { cn } from '@/lib/utils'

type Props = {
  class?: string
} & TabsListProps

const props = defineProps<Props>()
</script>

<template>
  <TabsList
    v-bind="props"
    :class="cn(
      'inline-flex h-10 items-center justify-center rounded-md bg-muted p-1 text-muted-foreground',
      props.class,
    )"
  >
    <slot />
  </TabsList>
</template>
