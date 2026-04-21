<script setup lang="ts">
import type { TableInfo } from '@/store/databaseStore'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { Badge } from '@/components/ui/badge'
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
  <div class="space-y-4">
    <div class="space-y-2.5">
      <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Table</Label>
      <Select v-model="selectedTable" :disabled="!tables.length || loading">
        <SelectTrigger>
          <SelectValue placeholder="Select table" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem
            v-for="t in tables"
            :key="t.name"
            :value="t.name"
          >
            <div class="flex w-full items-center justify-between">
              <span>{{ t.name }}</span>
              <Badge v-if="t.rowCount" variant="outline" class="ml-2">
                {{ t.rowCount.toLocaleString() }} rows
              </Badge>
            </div>
          </SelectItem>
        </SelectContent>
      </Select>
    </div>

    <div v-if="selectedTableInfo" class="text-sm text-muted-foreground">
      <span v-if="selectedTableInfo.rowCount">
        {{ selectedTableInfo.rowCount.toLocaleString() }} rows
      </span>
      <span v-if="selectedTableInfo.table_type" class="ml-2">
        ({{ selectedTableInfo.table_type }})
      </span>
    </div>
  </div>
</template>
