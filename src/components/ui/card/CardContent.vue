<script setup lang="ts">
import type { PrimitiveProps } from 'radix-vue'
import type { HTMLAttributes } from 'vue'
import { Primitive } from 'radix-vue'
import { computed } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<PrimitiveProps & { class?: HTMLAttributes['class'] }>(),
  {
    as: 'div',
  },
)

const delegatedProps = computed(() => {
  const { class: _, ...delegated } = props
  return delegated
})
</script>

<template>
  <Primitive v-bind="delegatedProps" :class="cn('p-6 pt-0', props.class)">
    <slot />
  </Primitive>
</template>
