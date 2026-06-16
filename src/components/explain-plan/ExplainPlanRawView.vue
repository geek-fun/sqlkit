<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  raw: string
  format: 'json' | 'text'
}>()

const displayContent = computed(() => {
  if (props.format === 'json') {
    try {
      return JSON.stringify(JSON.parse(props.raw), null, 2)
    }
    catch {
      return props.raw
    }
  }
  return props.raw
})
</script>

<template>
  <pre class="text-xs leading-relaxed font-mono m-3 p-3 border rounded bg-muted/30 whitespace-pre overflow-auto">{{ displayContent }}</pre>
</template>
