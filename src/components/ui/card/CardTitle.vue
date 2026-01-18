<script setup lang="ts">
import { type HTMLAttributes, computed } from 'vue'
import { Primitive, type PrimitiveProps } from 'radix-vue'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<PrimitiveProps & { class?: HTMLAttributes['class'] }>(),
  {
    as: 'h3',
  },
)

const delegatedProps = computed(() => {
  const { class: _, ...delegated } = props
  return delegated
})
</script>

<template>
  <Primitive v-bind="delegatedProps" :class="cn('text-2xl font-semibold leading-none tracking-tight', props.class)">
    <slot />
  </Primitive>
</template>
