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
    <button
      v-for="action in actions"
      :key="action.id"
      class="transfer-action-tile"
      :class="{ 'transfer-action-tile-active': props.modelValue === action.id }"
      @click="handleSelect(action.id)"
    >
      <span class="transfer-action-tile-icon text-lg" :class="action.icon" />
      <span class="transfer-action-tile-title">
        {{ t(`transfer.launcher.actions.${action.id}.title`) || action.title }}
      </span>
      <span class="transfer-action-tile-desc">
        {{ t(`transfer.launcher.actions.${action.id}.desc`) || action.desc }}
      </span>
    </button>
  </div>
</template>

<style scoped>
</style>
