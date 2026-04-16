<script setup lang="ts">
import type { ColumnMapping } from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'
import { ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

import { useTransferStore } from '@/store/transferStore'
import ConnectionSelector from '../shared/ConnectionSelector.vue'
import TableSelector from '../shared/TableSelector.vue'

export type ColumnInfo = {
  name: string
  data_type?: string
  nullable?: boolean
  default_value?: string
  is_primary_key?: boolean
}

const transferStore = useTransferStore()

const connectionId = ref('')
const database = ref('')
const schema = ref('')
const table = ref('')
const mappings = ref<ColumnMapping[]>([])
const targetColumns = ref<ColumnInfo[]>([])
const loadingColumns = ref(false)

async function fetchTargetColumns() {
  if (!connectionId.value || !database.value || !table.value)
    return

  loadingColumns.value = true
  try {
    const result = await invoke<ColumnInfo[]>('list_columns', {
      connectionId: connectionId.value,
      database: database.value,
      schema: schema.value,
      table: table.value,
    })
    targetColumns.value = result || []
  }
  catch (error) {
    console.error('Failed to fetch columns:', error)
  }
  finally {
    loadingColumns.value = false
  }
}

function autoMap() {
  mappings.value = mappings.value.map((m) => {
    const match = targetColumns.value.find((tc: ColumnInfo) => tc.name.toLowerCase() === m.sourceColumn.toLowerCase())
    return {
      ...m,
      targetColumn: match?.name,
      targetType: match?.data_type,
    }
  })
}

function clearAll() {
  mappings.value = mappings.value.map(m => ({
    ...m,
    targetColumn: undefined,
  }))
}

watch([connectionId, database, schema, table], () => {
  transferStore.importRequest = {
    ...transferStore.importRequest,
    connectionId: connectionId.value || undefined,
    database: database.value || undefined,
    schema: schema.value || undefined,
    table: table.value,
    columnMappings: mappings.value,
  }
  fetchTargetColumns()
})

watch(table, () => {
  autoMap()
})

watch(() => transferStore.importRequest.columnMappings, (newMappings) => {
  if (newMappings && newMappings.length > 0) {
    mappings.value = newMappings
  }
}, { immediate: true })
</script>

<template>
  <div class="space-y-6">
    <ConnectionSelector
      v-model:connection-id="connectionId"
      v-model:database="database"
      v-model:schema="schema"
      show-schema
    />

    <TableSelector
      v-model:table="table"
      :connection-id="connectionId"
      :database="database"
      :schema="schema"
    />

    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <Label>Column Mapping</Label>
        <div class="flex gap-2">
          <Button variant="outline" size="sm" @click="autoMap">
            Auto-Map
          </Button>
          <Button variant="outline" size="sm" @click="clearAll">
            Clear All
          </Button>
        </div>
      </div>

      <div v-if="loadingColumns" class="text-sm text-muted-foreground">
        Loading target columns...
      </div>

      <div v-else class="border rounded">
        <table class="text-sm w-full">
          <thead class="border-b">
            <tr>
              <th class="p-2 text-left">
                Source Column
              </th>
              <th class="p-2 text-left">
                Target Column
              </th>
              <th class="p-2 text-left">
                Type
              </th>
              <th class="p-2 text-left">
                Status
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="mapping in mappings" :key="mapping.sourceColumn" class="border-b">
              <td class="p-2">
                {{ mapping.sourceColumn }}
              </td>
              <td class="p-2">
                <Select v-model="mapping.targetColumn">
                  <SelectTrigger class="w-full">
                    <SelectValue placeholder="Select column" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="">
                      (Skip)
                    </SelectItem>
                    <SelectItem
                      v-for="col in targetColumns"
                      :key="col.name"
                      :value="col.name"
                    >
                      {{ col.name }}
                    </SelectItem>
                  </SelectContent>
                </Select>
              </td>
              <td class="text-muted-foreground p-2">
                {{ mapping.targetType || '-' }}
              </td>
              <td class="p-2">
                <span v-if="mapping.targetColumn" class="text-sm text-green-600">
                  ✓ Mapped
                </span>
                <span v-else class="text-sm text-muted-foreground">
                  ⊘ Skipped
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="text-sm text-muted-foreground">
        {{ mappings.filter(m => m.targetColumn).length }} of {{ mappings.length }} columns mapped
      </div>
    </div>
  </div>
</template>
