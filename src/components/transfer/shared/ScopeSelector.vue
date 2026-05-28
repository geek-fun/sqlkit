<script setup lang="ts">
import type { TransferScope } from '@/types/transfer'

const props = defineProps<{
  scope: TransferScope
  disabled?: boolean
}>()

const emit = defineEmits<{
  'update:scope': [value: TransferScope]
}>()

const scopeOptions: TransferScope[] = ['server', 'database', 'tables']

const scopeLabels: Record<TransferScope, string> = {
  server: 'Server',
  database: 'Database',
  tables: 'Tables',
}

function getChipClasses(option: TransferScope) {
  const isSelected = props.scope === option
  const baseClasses = 'px-3 py-1.5 rounded-md text-xs font-medium transition-colors cursor-pointer select-none'
  const selectedClasses = 'bg-primary/10 text-primary border border-primary/30'
  const unselectedClasses = 'bg-muted/30 text-muted-foreground hover:bg-muted/50 border border-transparent'
  const disabledClasses = 'opacity-50 cursor-not-allowed pointer-events-none'

  return [
    baseClasses,
    isSelected ? selectedClasses : unselectedClasses,
    props.disabled ? disabledClasses : '',
  ].join(' ')
}

function handleSelect(scope: TransferScope) {
  if (!props.disabled) {
    emit('update:scope', scope)
  }
}
</script>

<template>
  <div class="flex gap-1.5">
    <button
      v-for="option in scopeOptions"
      :key="option"
      :class="getChipClasses(option)"
      @click="handleSelect(option)"
    >
      {{ scopeLabels[option] }}
    </button>
  </div>
</template>
