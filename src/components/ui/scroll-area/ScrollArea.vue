<script setup lang="ts">
import type { ScrollAreaRootProps } from 'radix-vue'
import type { HTMLAttributes } from 'vue'
import { ScrollAreaRoot, ScrollAreaViewport } from 'radix-vue'
import { computed, ref, unref } from 'vue'
import { cn } from '@/lib/utils'
import ScrollBar from './ScrollBar.vue'

const props = defineProps<
  ScrollAreaRootProps & {
    class?: HTMLAttributes['class']
  }
>()

const viewportRef = ref<InstanceType<typeof ScrollAreaViewport> | null>(null)
const viewportElement = computed<HTMLElement | null>(() => {
  const raw = viewportRef.value?.viewportElement
  return raw ? (unref(raw) ?? null) : null
})

defineExpose({ viewportElement })
</script>

<template>
  <ScrollAreaRoot :class="cn('relative overflow-hidden', props.class)">
    <ScrollAreaViewport ref="viewportRef" class="pr-1.5 rounded-[inherit] h-full w-full">
      <slot />
    </ScrollAreaViewport>
    <ScrollBar />
    <slot name="scrollbar" />
  </ScrollAreaRoot>
</template>
