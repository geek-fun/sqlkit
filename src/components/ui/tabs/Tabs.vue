<script setup lang="ts">
import type { TabsRootEmits, TabsRootProps } from 'radix-vue'
import { TabsRoot, useForwardPropsEmits } from 'radix-vue'
import { cn } from '@/lib/utils'

type Props = {
  class?: string
} & TabsRootProps

const props = defineProps<Props>()
const emits = defineEmits<TabsRootEmits>()

const forwarded = useForwardPropsEmits(props, emits)
</script>

<template>
  <TabsRoot
    v-bind="forwarded"
    :class="cn('w-full', props.class)"
  >
    <slot />
  </TabsRoot>
</template>
