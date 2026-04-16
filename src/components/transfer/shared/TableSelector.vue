<script setup lang="ts">
import type { TableInfo } from '@/store/databaseStore'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Label } from '@/components/ui/label'

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const props = defineProps<{
  connectionId?: string
  database?: string
  schema?: string
  table?: string
}>()

const emit = defineEmits<{
  'update:table': [value: string]
}>()

const selectedTable = computed({
  get: () => props.table || '',
  set: val => emit('update:table', val),
})

const tables = ref<TableInfo[]>([])
const loading = ref(false)

async function fetchTables() {
  if (!props.connectionId || !props.database)
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

watch([() => props.connectionId, () => props.database], () => {
  fetchTables()
}, { immediate: true })

const selectedTableInfo = computed(() =>
  tables.value.find(t => t.name === selectedTable.value),
)
</script>

<template>
  <div class="space-y-4">
    <div class="space-y-2">
      <Label>Table</Label>
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
