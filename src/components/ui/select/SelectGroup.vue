<script setup lang="ts">
import type { SelectGroupProps } from 'radix-vue'
import { SelectGroup } from 'radix-vue'

const props = defineProps<SelectGroupProps>()
</script>

<template>
  <SelectGroup v-bind="props" class="p-1 w-full">
    <slot />
  </SelectGroup>
</template>
