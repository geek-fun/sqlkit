<script setup lang="ts">
import type { TabsContentProps } from 'radix-vue'
import { TabsContent } from 'radix-vue'
import { cn } from '@/lib/utils'

type Props = {
  class?: string
} & TabsContentProps

const props = defineProps<Props>()
</script>

<template>
  <TabsContent
    v-bind="props"
    :class="cn(
      'mt-2 ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
      props.class,
    )"
  >
    <slot />
  </TabsContent>
</template>
