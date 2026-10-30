<script setup lang="ts">
import type { DatabaseType } from '@/store'
import { useDatabaseIcon } from '@/composables/useDatabaseIcon'

const props = defineProps<{
  type: DatabaseType
  size?: number
}>()

const { getDatabaseIcon } = useDatabaseIcon()

const s = props.size ?? 16
</script>

<template>
  <img
    :src="getDatabaseIcon(props.type)"
    :alt="props.type"
    :width="s"
    :height="s"
    class="object-contain"
  >
</template>
