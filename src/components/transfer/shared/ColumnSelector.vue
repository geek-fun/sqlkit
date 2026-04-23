<!--
  Visual Role: Dense column selection grid.
  Uses small checkboxes, tight rows, and muted ghost buttons for bulk actions.
-->
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
      schema: props.schema || null,
      tableName: props.table,
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
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <Label class="text-[11px] text-muted-foreground tracking-wide font-medium uppercase">Columns</Label>
      <div class="flex gap-1.5">
        <Button variant="ghost" size="sm" class="text-[11px] text-muted-foreground px-2 h-6 hover:text-foreground" @click="selectAll">
          Select All
        </Button>
        <Button variant="ghost" size="sm" class="text-[11px] text-muted-foreground px-2 h-6 hover:text-foreground" @click="deselectAll">
          Deselect All
        </Button>
      </div>
    </div>

    <div v-if="loading" class="text-[11px] text-muted-foreground p-6 border rounded-md border-dashed bg-muted/20 flex items-center justify-center">
      <span class="i-carbon-circle-dash mr-1.5 animate-spin" /> Loading columns...
    </div>

    <div v-else class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border pr-2 gap-1.5 grid grid-cols-1 max-h-60 overflow-y-auto md:grid-cols-5 sm:grid-cols-3">
      <label
        v-for="col in availableColumns"
        :key="col.name"
        class="px-2 py-1 rounded-sm flex cursor-pointer transition-colors items-center space-x-2 hover:bg-muted/40"
        :class="[
          isColumnSelected(col.name) ? 'bg-primary/[0.03]' : 'bg-transparent',
        ]"
      >
        <Checkbox
          :id="`col-${col.name}`"
          :checked="isColumnSelected(col.name)"
          class="h-3.5 w-3.5"
          @update:checked="toggleColumn(col.name)"
        />
        <span class="text-[11px] leading-none font-mono truncate peer-disabled:opacity-70 peer-disabled:cursor-not-allowed" :title="col.name">
          {{ col.name }}
        </span>
      </label>
    </div>

    <div class="text-[10px] text-muted-foreground font-mono">
      {{ selectedColumns.length }} of {{ availableColumns.length }} columns selected
    </div>
  </div>
</template>
