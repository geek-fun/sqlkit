<script setup lang="ts">
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'

import { useTransferStore } from '@/store/transferStore'

const emit = defineEmits<{
  click: []
}>()

const transferStore = useTransferStore()
</script>

<template>
  <Button variant="outline" size="sm" class="gap-2" @click="emit('click')">
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="12" r="10" />
      <polyline points="12 6 12 12 16 14" />
    </svg>
    Tasks
    <Badge v-if="transferStore.taskCount > 0" variant="default" class="ml-1">
      {{ transferStore.taskCount }}
    </Badge>
  </Button>
</template>
