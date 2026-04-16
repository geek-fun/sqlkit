<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'

import { Label } from '@/components/ui/label'

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

const selectedColumns = computed({
  get: () => props.columns || [],
  set: val => emit('update:columns', val),
})

const availableColumns = ref<ColumnInfo[]>([])
const loading = ref(false)

async function fetchColumns() {
  if (!props.connectionId || !props.database || !props.table)
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

watch([() => props.connectionId, () => props.database, () => props.table], () => {
  fetchColumns()
}, { immediate: true })

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
      <Label>Columns</Label>
      <div class="flex gap-2">
        <Button variant="outline" size="sm" @click="selectAll">
          Select All
        </Button>
        <Button variant="outline" size="sm" @click="deselectAll">
          Deselect All
        </Button>
      </div>
    </div>

    <div v-if="loading" class="text-sm text-muted-foreground">
      Loading columns...
    </div>

    <div v-else class="gap-2 grid grid-cols-4">
      <div
        v-for="col in availableColumns"
        :key="col.name"
        class="p-2 border rounded flex items-center space-x-2"
        :class="isColumnSelected(col.name) ? 'bg-secondary' : 'bg-transparent'"
      >
        <Checkbox
          :checked="isColumnSelected(col.name)"
          @update:checked="toggleColumn(col.name)"
        />
        <Label class="text-sm cursor-pointer" @click="toggleColumn(col.name)">
          {{ col.name }}
        </Label>
      </div>
    </div>

    <div class="text-sm text-muted-foreground">
      {{ selectedColumns.length }} of {{ availableColumns.length }} columns selected
    </div>
  </div>
</template>
