<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'

import { Label } from '@/components/ui/label'

import { ConnectionStatus, useConnectionStore } from '@/store/connectionStore'

export type ColumnInfo = {
  name: string
  data_type?: string
  nullable?: boolean
  default_value?: string
  is_primary_key?: boolean
}

const props = defineProps<{
  connectionId?: string
  database?: string
  schema?: string
  table?: string
  columns?: string[]
}>()

const emit = defineEmits<{
  'update:columns': [value: string[]]
}>()

const connectionStore = useConnectionStore()

const selectedColumns = computed({
  get: () => props.columns || [],
  set: val => emit('update:columns', val),
})

const availableColumns = ref<ColumnInfo[]>([])
const loading = ref(false)

// Check if connection is actually connected
const isConnected = computed(() => {
  if (!props.connectionId)
    return false
  return connectionStore.getConnectionStatus(props.connectionId) === ConnectionStatus.CONNECTED
})

async function fetchColumns() {
  if (!props.connectionId || !props.database || !props.table || !isConnected.value)
    return

  loading.value = true
  try {
    const result = await invoke<ColumnInfo[]>('list_columns', {
      connectionId: props.connectionId,
      database: props.database,
      schema: props.schema,
      table: props.table,
    })
    availableColumns.value = result || []
    if (selectedColumns.value.length === 0 && availableColumns.value.length > 0) {
      selectedColumns.value = availableColumns.value.map((c: ColumnInfo) => c.name)
    }
  }
  catch (error) {
    console.error('Failed to fetch columns:', error)
  }
  finally {
    loading.value = false
  }
}

// Watch for connection status, database, and table changes
const columnFetchParams = computed(() => {
  if (!isConnected.value || !props.database || !props.table)
    return null
  return {
    connectionId: props.connectionId,
    database: props.database,
    schema: props.schema,
    table: props.table,
  }
})

watch(columnFetchParams, (params, oldParams) => {
  if (params && JSON.stringify(params) !== JSON.stringify(oldParams)) {
    fetchColumns()
  }
}, { immediate: true, deep: true })

function selectAll() {
  selectedColumns.value = availableColumns.value.map((c: ColumnInfo) => c.name)
}

function deselectAll() {
  selectedColumns.value = []
}

function toggleColumn(colName: string) {
  const current = [...selectedColumns.value]
  const index = current.indexOf(colName)
  if (index > -1) {
    current.splice(index, 1)
  }
  else {
    current.push(colName)
  }
  selectedColumns.value = current
}

const isColumnSelected = (colName: string) => selectedColumns.value.includes(colName)
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Columns</Label>
      <div class="flex gap-2">
        <Button variant="ghost" size="sm" class="text-xs h-8" @click="selectAll">
          Select All
        </Button>
        <Button variant="ghost" size="sm" class="text-xs h-8" @click="deselectAll">
          Deselect All
        </Button>
      </div>
    </div>

    <div v-if="loading" class="text-sm text-muted-foreground p-8 border rounded-md border-dashed flex items-center justify-center">
      <span class="i-carbon-circle-dash mr-2 animate-spin" /> Loading columns...
    </div>

    <div v-else class="gap-3 grid grid-cols-1 md:grid-cols-4 sm:grid-cols-2">
      <label
        v-for="col in availableColumns"
        :key="col.name"
        class="p-3 border rounded-md flex cursor-pointer transition-colors items-center space-x-3 hover:bg-muted/50"
        :class="[
          isColumnSelected(col.name) ? 'border-primary/50 bg-primary/5' : 'border-border bg-transparent',
        ]"
      >
        <Checkbox
          :id="`col-${col.name}`"
          :checked="isColumnSelected(col.name)"
          @update:checked="toggleColumn(col.name)"
        />
        <span class="text-sm leading-none font-medium peer-disabled:opacity-70 peer-disabled:cursor-not-allowed">
          {{ col.name }}
        </span>
      </label>
    </div>

    <div class="text-xs text-muted-foreground">
      {{ selectedColumns.length }} of {{ availableColumns.length }} columns selected
    </div>
  </div>
</template>
