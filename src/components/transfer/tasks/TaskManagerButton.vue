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
  <Button variant="outline" size="sm" class="border-border/40 gap-2 h-8" @click="emit('click')">
    <span class="i-carbon-list-boxes h-3.5 w-3.5" />
    <span class="text-xs">Tasks</span>
    <Badge v-if="transferStore.taskCount > 0" variant="default" class="text-[10px] font-mono ml-1.5 px-1 rounded-full flex h-4 min-w-4 items-center justify-center tabular-nums">
      {{ transferStore.taskCount }}
    </Badge>
  </Button>
</template>
