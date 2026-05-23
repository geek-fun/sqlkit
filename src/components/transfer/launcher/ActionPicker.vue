<script setup lang="ts">
import type { LauncherAction } from './types'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  modelValue: LauncherAction
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: LauncherAction): void
}>()

const { t } = useI18n()

const actions: { id: LauncherAction, icon: string, title: string, desc: string }[] = [
  { id: 'backup', icon: 'i-carbon-save', title: 'Backup', desc: 'Export database to a file' },
  { id: 'restore', icon: 'i-carbon-cloud-upload', title: 'Restore', desc: 'Import from a backup file' },
  { id: 'migrate', icon: 'i-carbon-data-enrichment', title: 'Migrate', desc: 'Transfer to another server' },
  { id: 'export', icon: 'i-carbon-document-export', title: 'Export', desc: 'Export table data to CSV/Excel' },
]

function handleSelect(id: LauncherAction) {
  emit('update:modelValue', id)
}
</script>

<template>
  <div class="gap-3 grid grid-cols-2 md:grid-cols-4">
    <div
      v-for="action in actions"
      :key="action.id"
      class="p-4 border rounded-lg cursor-pointer transition-all hover:bg-muted/50"
      :class="[
        props.modelValue === action.id
          ? 'border-primary ring-1 ring-primary bg-primary/5'
          : 'border-border',
      ]"
      @click="handleSelect(action.id)"
    >
      <div class="mb-1 flex gap-2 items-center">
        <span class="text-xl" :class="[action.icon, props.modelValue === action.id ? 'text-primary' : 'text-muted-foreground']" />
        <h4 class="text-sm font-medium">
          {{ t(`transfer.launcher.actions.${action.id}.title`) || action.title }}
        </h4>
      </div>
      <p class="text-xs text-muted-foreground">
        {{ t(`transfer.launcher.actions.${action.id}.desc`) || action.desc }}
      </p>
    </div>
  </div>
</template>
