<!--
  Visual Role: Multi-table selection in a table format.
  Displays table names with row counts using checkboxes for multi-selection.
  Includes Select All/Deselect All controls.
-->
<script setup lang="ts">
import type { TableInfo } from '@/store/databaseStore'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Checkbox } from '@/components/ui/checkbox'
import { toast } from '@/composables/useNotifications'
import { ConnectionStatus, useConnectionStore } from '@/store/connectionStore'

const props = defineProps<{
  connectionId?: string
  database?: string
  schema?: string
  selectedTables?: string[]
}>()

const emit = defineEmits<{
  'update:selectedTables': [value: string[]]
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()

const tables = ref<TableInfo[]>([])
const loading = ref(false)
const error = ref<string>()

const isConnected = computed(() => {
  if (!props.connectionId)
    return false
  return connectionStore.getConnectionStatus(props.connectionId) === ConnectionStatus.CONNECTED
})

const selected = computed({
  get: () => props.selectedTables || [],
  set: val => emit('update:selectedTables', val),
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
    error.value = undefined
  }
  catch (e) {
    console.error('Failed to fetch tables:', e)
    const errorMsg = String(e)
    error.value = errorMsg
    tables.value = []
    toast.error(t('transfer.export.tables.loadFailed', 'Failed to load tables'), { description: errorMsg })
  }
  finally {
    loading.value = false
  }
}

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

function selectAllTables() {
  selected.value = tables.value.map(t => t.name)
}

function deselectAllTables() {
  selected.value = []
}

function toggleTable(tableName: string) {
  const current = [...selected.value]
  const index = current.indexOf(tableName)
  if (index > -1) {
    current.splice(index, 1)
  }
  else {
    current.push(tableName)
  }
  selected.value = current
}

const isTableSelected = (tableName: string) => selected.value.includes(tableName)

const selectionCount = computed(() => selected.value.length)
const totalCount = computed(() => tables.value.length)
const allSelected = computed(() => totalCount.value > 0 && selectionCount.value === totalCount.value)
const someSelected = computed(() => selectionCount.value > 0 && selectionCount.value < totalCount.value)
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Table Content -->
    <div class="flex flex-1 flex-col min-h-0 overflow-hidden">
      <div v-if="error" class="p-4 border border-red-200 rounded-md bg-red-50 flex-1 dark:border-red-800 dark:bg-red-900/20">
        <div class="flex gap-3 items-start">
          <span class="i-carbon-warning-alt text-lg text-red-600 shrink-0 dark:text-red-400" />
          <div>
            <p class="text-sm text-red-800 font-medium dark:text-red-200">
              {{ t('transfer.export.tables.loadFailed', 'Failed to load tables') }}
            </p>
            <p class="text-xs text-red-700 mt-1 dark:text-red-300">
              {{ error }}
            </p>
          </div>
        </div>
      </div>

      <div v-else-if="loading" class="text-[11px] text-muted-foreground p-4 border rounded-md border-dashed bg-muted/20 flex flex-1 items-center justify-center">
        <span class="i-carbon-circle-dash mr-1.5 animate-spin" /> Loading tables...
      </div>

      <div v-else-if="tables.length === 0" class="border border-border/40 rounded-md flex-1 overflow-hidden">
        <table class="text-xs w-full">
          <thead class="text-[10px] text-muted-foreground tracking-wide border-b border-border/40 bg-muted/40 uppercase">
            <tr>
              <th scope="col" class="font-medium px-2 py-1.5 text-left w-8">
                <Checkbox :checked="false" class="h-3.5 w-3.5" disabled />
              </th>
              <th scope="col" class="font-medium px-2 py-1.5 text-left">
                Table Name
              </th>
              <th scope="col" class="font-medium px-2 py-1.5 text-right">
                Rows
              </th>
              <th scope="col" class="font-medium px-2 py-1.5 text-right">
                Type
              </th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td colspan="4" class="text-[11px] text-muted-foreground px-2 py-6 text-center italic">
                <span class="i-carbon-data-base mr-1.5 opacity-50" /> No tables found
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-else class="border border-border/40 rounded-md flex flex-1 flex-col overflow-hidden">
        <table class="text-xs w-full">
          <caption class="sr-only">
            Available tables for export selection
          </caption>
          <thead class="text-[10px] text-muted-foreground tracking-wide border-b border-border/40 bg-muted/40 uppercase top-0 sticky z-10">
            <tr>
              <th scope="col" class="font-medium px-2 py-1.5 text-left w-8">
                <Checkbox
                  :checked="allSelected"
                  :indeterminate="someSelected"
                  class="h-3.5 w-3.5"
                  @update:checked="allSelected ? deselectAllTables() : selectAllTables()"
                />
              </th>
              <th scope="col" class="font-medium px-2 py-1.5 text-left">
                Table Name
              </th>
              <th scope="col" class="font-medium px-2 py-1.5 text-right">
                Rows
              </th>
              <th scope="col" class="font-medium px-2 py-1.5 text-right w-20">
                <span class="flex gap-1 items-center justify-end">
                  <span v-if="selectionCount > 0" class="text-primary font-semibold">{{ selectionCount }}</span>
                  <span v-else>Type</span>
                </span>
              </th>
            </tr>
          </thead>
        </table>
        <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border flex-1 overflow-y-auto">
          <table class="text-xs w-full">
            <tbody>
              <tr
                v-for="table in tables"
                :key="table.name"
                class="border-b border-border/40 transition-colors last:border-0 hover:bg-muted/40"
                :class="isTableSelected(table.name) ? 'bg-primary/[0.03]' : ''"
              >
                <td class="px-2 py-1.5 align-middle w-8">
                  <Checkbox
                    :checked="isTableSelected(table.name)"
                    class="h-3.5 w-3.5"
                    @update:checked="toggleTable(table.name)"
                  />
                </td>
                <th scope="row" class="font-mono font-normal px-2 py-1.5 text-left align-middle max-w-[180px] truncate">
                  {{ table.name }}
                </th>
                <td class="text-[10px] text-muted-foreground font-mono px-2 py-1.5 text-right align-middle tabular-nums">
                  {{ table.rowCount?.toLocaleString() || '-' }}
                </td>
                <td class="text-[10px] text-muted-foreground px-2 py-1.5 text-right align-middle uppercase">
                  {{ table.table_type || 'TABLE' }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- Selection Count Footer -->
    <div v-if="selectionCount > 0" class="text-[10px] text-primary font-mono font-semibold mt-2 flex shrink-0 gap-1 items-center">
      <span class="i-carbon-checkmark-filled h-3 w-3" />
      {{ selectionCount }} tables selected
    </div>
    <div v-else class="text-[10px] text-muted-foreground mt-2 shrink-0">
      Select tables to export
    </div>
  </div>
</template>
