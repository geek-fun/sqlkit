<script setup lang="ts">
import type { LauncherTarget } from './types'
import type { DatabaseSchema } from '@/store/databaseStore'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Label } from '@/components/ui/label'

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useConnectionStore } from '@/store/connectionStore'

const props = defineProps<{
  modelValue: LauncherTarget
  sourceConnectionId?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: LauncherTarget): void
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()

const connections = computed(() => connectionStore.connections)

const state = computed({
  get: () => props.modelValue,
  set: val => emit('update:modelValue', val),
})

const databases = ref<DatabaseSchema[]>([])
const isLoadingDb = ref(false)
const dbReqId = ref(0)

async function loadDatabases(connectionId: string) {
  const myId = ++dbReqId.value
  try {
    isLoadingDb.value = true
    const result = await invoke<DatabaseSchema[]>('list_databases', { connectionId })
    if (myId !== dbReqId.value)
      return
    databases.value = result.filter(db => !db.is_system)
  }
  catch (e) {
    if (myId !== dbReqId.value)
      return
    console.error('Failed to load databases', e)
    databases.value = []
  }
  finally {
    if (myId === dbReqId.value)
      isLoadingDb.value = false
  }
}

watch(() => state.value.connectionId, (newId) => {
  state.value = { ...state.value, database: undefined }
  if (newId) {
    loadDatabases(newId)
  }
})
</script>

<template>
  <div class="transfer-console-section">
    <div class="transfer-console-section-header">
      <span class="i-carbon-data-base text-xs" />
      Target Destination
    </div>
    <div class="transfer-console-section-body space-y-3">
      <div class="space-y-1.5">
        <Label class="transfer-mono-label text-muted-foreground">{{ t('transfer.launcher.connection') }}</Label>
        <Select :model-value="state.connectionId || ''" @update:model-value="(v) => state = { ...state, connectionId: v }">
          <SelectTrigger>
            <SelectValue :placeholder="t('transfer.launcher.selectConnection')" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="conn in connections" :key="conn.id" :value="conn.id || ''">
              {{ conn.name }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div class="space-y-1.5">
        <Label class="transfer-mono-label text-muted-foreground">{{ t('transfer.launcher.database') }}</Label>
        <div class="flex gap-2 items-center">
          <Select :model-value="state.database || ''" :disabled="!state.connectionId || isLoadingDb" @update:model-value="(v) => state = { ...state, database: v }">
            <SelectTrigger>
              <SelectValue :placeholder="t('transfer.launcher.selectDatabase')" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="db in databases" :key="db.name" :value="db.name">
                {{ db.name }}
              </SelectItem>
            </SelectContent>
          </Select>
          <span v-if="isLoadingDb" class="i-carbon-circle-dash text-muted-foreground animate-spin" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
</style>
