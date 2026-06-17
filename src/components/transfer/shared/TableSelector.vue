<!--
  Visual Role: Compact table selection dropdown.
  Displays table names alongside row counts using monospaced numerals.
-->
<script setup lang="ts">
import type { TableInfo } from '@/store/databaseStore'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Label } from '@/components/ui/label'

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

import { ConnectionStatus, useConnectionStore } from '@/store/connectionStore'

const props = defineProps<{
  connectionId?: string
  database?: string
  schema?: string
  table?: string
}>()

const emit = defineEmits<{
  'update:table': [value: string]
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()

const selectedTable = computed({
  get: () => props.table || '',
  set: val => emit('update:table', val),
})

const tables = ref<TableInfo[]>([])
const loading = ref(false)

// Check if connection is actually connected
const isConnected = computed(() => {
  if (!props.connectionId)
    return false
  return connectionStore.getConnectionStatus(props.connectionId) === ConnectionStatus.CONNECTED
})

async function fetchTables() {
  if (!props.connectionId || !props.database || !isConnected.value)
    return

  loading.value = true
  try {
    const result = await invoke<TableInfo[]>('list_tables', {
      connectionId: props.connectionId,
      database: props.database,
      schema: props.schema,
    })
    tables.value = result || []
    if (tables.value.length > 0 && !selectedTable.value) {
      selectedTable.value = tables.value[0].name
    }
  }
  catch (error) {
    console.error('Failed to fetch tables:', error)
  }
  finally {
    loading.value = false
  }
}

// Watch for connection status and database changes
const tableFetchParams = computed(() => {
  if (!isConnected.value || !props.database)
    return null
  return {
    connectionId: props.connectionId,
    database: props.database,
    schema: props.schema,
  }
})

watch(tableFetchParams, (params, oldParams) => {
  if (params && JSON.stringify(params) !== JSON.stringify(oldParams)) {
    fetchTables()
  }
}, { immediate: true, deep: true })

const selectedTableInfo = computed(() =>
  tables.value.find(t => t.name === selectedTable.value),
)
</script>

<template>
  <div class="space-y-3">
    <div class="space-y-1.5">
      <Label class="text-[11px] text-muted-foreground tracking-wide font-medium uppercase">{{ t('transfer.connection.tableLabel') }}</Label>
      <Select v-model="selectedTable" :disabled="!tables.length || loading">
        <SelectTrigger class="text-xs border-border/40 bg-muted/20 h-8">
          <SelectValue :placeholder="t('transfer.connection.tablePlaceholder')" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem
            v-for="t in tables"
            :key="t.name"
            :value="t.name"
            class="text-xs"
          >
            <div class="flex gap-4 w-full items-center justify-between">
              <span>{{ t.name }}</span>
              <span v-if="t.rowCount" class="text-[10px] text-muted-foreground font-mono px-1 rounded-sm bg-muted/60 tabular-nums">
                {{ t.rowCount.toLocaleString() }} rows
              </span>
            </div>
          </SelectItem>
        </SelectContent>
      </Select>
    </div>

    <div v-if="selectedTableInfo" class="text-[11px] text-muted-foreground flex gap-2 items-center">
      <span v-if="selectedTableInfo.rowCount" class="font-mono tabular-nums">
        {{ selectedTableInfo.rowCount.toLocaleString() }} rows
      </span>
      <span v-if="selectedTableInfo.table_type" class="text-muted-foreground/60 uppercase">
        {{ selectedTableInfo.table_type }}
      </span>
    </div>
  </div>
</template>
