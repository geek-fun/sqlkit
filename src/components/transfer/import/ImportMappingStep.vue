<script setup lang="ts">
import type { ColumnMapping } from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
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

const targetParams = computed(() => {
  return {
    connectionId: connectionId.value,
    database: database.value,
    schema: schema.value,
    table: table.value,
  }
})

watch(targetParams, (params, oldParams) => {
  transferStore.importRequest = {
    ...transferStore.importRequest,
    connectionId: params.connectionId || undefined,
    database: params.database || undefined,
    schema: params.schema || undefined,
    table: params.table,
    columnMappings: mappings.value,
  }

  if (params && JSON.stringify(params) !== JSON.stringify(oldParams)) {
    if (params.connectionId && params.database && params.table) {
      fetchTargetColumns()
    }
  }
}, { deep: true })

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
          <caption class="sr-only">
            Column mapping between source and target database
          </caption>
          <thead class="border-b bg-muted/50">
            <tr>
              <th scope="col" class="font-medium p-2 text-left">
                Source Column
              </th>
              <th scope="col" class="font-medium p-2 text-left">
                Target Column
              </th>
              <th scope="col" class="font-medium p-2 text-left">
                Type
              </th>
              <th scope="col" class="font-medium p-2 text-left">
                Status
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="mapping in mappings" :key="mapping.sourceColumn" class="border-b transition-colors hover:bg-muted/50">
              <th scope="row" class="font-normal p-2 text-left align-middle">
                {{ mapping.sourceColumn }}
              </th>
              <td class="p-2 align-middle">
                <Select v-model="mapping.targetColumn">
                  <SelectTrigger
                    class="w-full"
                    :aria-label="`Target column for ${mapping.sourceColumn}`"
                  >
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
              <td class="text-muted-foreground p-2 align-middle">
                {{ mapping.targetType || '-' }}
              </td>
              <td class="p-2 align-middle">
                <span v-if="mapping.targetColumn" class="text-sm text-green-600 font-medium flex items-center">
                  <span class="sr-only">Status: </span>
                  <span aria-hidden="true" class="mr-1">✓</span> Mapped
                </span>
                <span v-else class="text-sm text-muted-foreground font-medium flex items-center">
                  <span class="sr-only">Status: </span>
                  <span aria-hidden="true" class="mr-1">⊘</span> Skipped
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
