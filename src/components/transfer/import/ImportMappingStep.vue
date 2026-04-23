<script setup lang="ts">
import type { ColumnMapping } from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
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
      schema: schema.value || null,
      tableName: table.value,
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
  <div class="space-y-4">
    <div class="gap-2.5 grid grid-cols-2">
      <div class="col-span-2">
        <ConnectionSelector
          v-model:connection-id="connectionId"
          v-model:database="database"
          v-model:schema="schema"
          show-schema
        />
      </div>
      <div class="col-span-2">
        <TableSelector
          v-model:table="table"
          :connection-id="connectionId"
          :database="database"
          :schema="schema"
        />
      </div>
    </div>

    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <div class="text-xs tracking-wide font-semibold flex gap-2 items-center">
          <div class="i-carbon-flow-data" />
          COLUMN MAPPING
        </div>
        <div class="flex gap-2">
          <Button variant="outline" size="sm" class="text-[11px] px-2 h-6" @click="autoMap">
            Auto-Map
          </Button>
          <Button variant="ghost" size="sm" class="text-[11px] px-2 h-6" @click="clearAll">
            Clear All
          </Button>
        </div>
      </div>

      <div v-if="loadingColumns" class="text-[11px] text-muted-foreground py-2 text-center border border-border/40 rounded-sm border-dashed">
        Loading target columns...
      </div>

      <div v-else class="border border-border/40 rounded-sm overflow-hidden">
        <table class="text-xs w-full">
          <caption class="sr-only">
            Column mapping between source and target database
          </caption>
          <thead class="text-[10px] text-muted-foreground tracking-wide border-b border-border/40 bg-muted/40 uppercase">
            <tr>
              <th scope="col" class="font-medium px-2 py-1.5 text-left">
                Source Column
              </th>
              <th scope="col" class="font-medium px-2 py-1.5 text-left w-[40%]">
                Target Column
              </th>
              <th scope="col" class="font-medium px-2 py-1.5 text-left">
                Type
              </th>
              <th scope="col" class="font-medium px-2 py-1.5 text-left w-20">
                Status
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="mappings.length === 0" class="border-b border-border/40">
              <td colspan="4" class="text-[11px] text-muted-foreground px-2 py-4 text-center italic">
                No columns detected
              </td>
            </tr>
            <tr v-for="mapping in mappings" :key="mapping.sourceColumn" class="border-b border-border/40 transition-colors last:border-0 hover:bg-muted/40">
              <th scope="row" class="font-mono font-normal px-2 py-1 text-left align-middle max-w-[120px] truncate" :title="mapping.sourceColumn">
                {{ mapping.sourceColumn }}
              </th>
              <td class="px-2 py-1 align-middle">
                <Select v-model="mapping.targetColumn">
                  <SelectTrigger
                    class="text-xs font-mono h-7 w-full"
                    :aria-label="`Target column for ${mapping.sourceColumn}`"
                  >
                    <SelectValue placeholder="Skip" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="" class="text-xs text-muted-foreground italic">
                      (Skip)
                    </SelectItem>
                    <SelectItem
                      v-for="col in targetColumns"
                      :key="col.name"
                      :value="col.name"
                      class="text-xs font-mono"
                    >
                      {{ col.name }}
                    </SelectItem>
                  </SelectContent>
                </Select>
              </td>
              <td class="text-[10px] text-muted-foreground font-mono px-2 py-1 align-middle max-w-[80px] truncate uppercase" :title="mapping.targetType">
                {{ mapping.targetType || '-' }}
              </td>
              <td class="px-2 py-1 align-middle">
                <span v-if="mapping.targetColumn" class="text-[10px] text-green-600 tracking-wide font-medium px-1.5 py-0.5 rounded-sm bg-green-500/10 flex w-fit uppercase items-center">
                  <span class="sr-only">Status: </span>
                  <div class="i-carbon-checkmark mr-1" aria-hidden="true" /> Mapped
                </span>
                <span v-else class="text-[10px] text-muted-foreground tracking-wide font-medium px-1.5 py-0.5 rounded-sm bg-muted flex w-fit uppercase items-center">
                  <span class="sr-only">Status: </span>
                  <div class="i-carbon-subtract mr-1" aria-hidden="true" /> Skipped
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="text-[11px] text-muted-foreground tracking-wide flex uppercase justify-end">
        <span class="font-mono mr-1 tabular-nums">{{ mappings.filter(m => m.targetColumn).length }}</span> of
        <span class="font-mono mx-1 tabular-nums">{{ mappings.length }}</span> columns mapped
      </div>
    </div>
  </div>
</template>
