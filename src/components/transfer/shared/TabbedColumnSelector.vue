<!--
  Visual Role: Tabbed column selection for multiple tables.
  Shows columns for each selected table in separate tabs.
  Includes Select All/Deselect All for columns within each tab.
-->
<script setup lang="ts">
import type { ColumnInfo, TableColumns } from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Checkbox } from '@/components/ui/checkbox'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { toast } from '@/composables/useNotifications'
import { ConnectionStatus, useConnectionStore } from '@/store/connectionStore'

const props = defineProps<{
  connectionId?: string
  database?: string
  schema?: string
  selectedTables?: string[]
  tableColumns?: TableColumns[]
}>()

const emit = defineEmits<{
  'update:tableColumns': [value: TableColumns[]]
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()

const tableColumnsMap = ref<Map<string, { columns: ColumnInfo[], selected: string[] }>>(new Map())
const tableErrors = ref<Map<string, string>>(new Map())
const loadingTables = ref<Set<string>>(new Set())
const activeTab = ref<string>('')

const isConnected = computed(() => {
  if (!props.connectionId)
    return false
  return connectionStore.getConnectionStatus(props.connectionId) === ConnectionStatus.CONNECTED
})

const tableColumnsValue = computed({
  get: () => props.tableColumns || [],
  set: val => emit('update:tableColumns', val),
})

// Compute fetch params that depend on connection status
const columnFetchParams = computed(() => {
  if (!isConnected.value || !props.database || !props.connectionId)
    return null
  return {
    connectionId: props.connectionId,
    database: props.database,
    schema: props.schema,
    selectedTables: props.selectedTables || [],
  }
})

// Watch for connection/database changes - reset everything
watch([() => props.connectionId, () => props.database], ([newConnId, newDb], [oldConnId, oldDb]) => {
  if (newConnId !== oldConnId || newDb !== oldDb) {
    // Reset all state when connection or database changes
    tableColumnsMap.value.clear()
    tableErrors.value.clear()
    loadingTables.value.clear()
    activeTab.value = ''
    syncToParent()
  }
})

// Watch fetch params instead of selectedTables directly
watch(columnFetchParams, (params, oldParams) => {
  if (!params)
    return

  const oldTables = oldParams?.selectedTables || []
  const newTables = params.selectedTables

  const addedTables = newTables.filter(t => !oldTables.includes(t))
  const removedTables = oldTables.filter(t => !newTables.includes(t))

  // Remove columns and errors for unselected tables
  removedTables.forEach((tableName) => {
    tableColumnsMap.value.delete(tableName)
    tableErrors.value.delete(tableName)
  })

  // Load columns for newly selected tables (connection is now guaranteed ready)
  for (const tableName of addedTables) {
    fetchColumnsForTable(tableName)
  }

  // Set first table as active tab if current tab is removed or no active tab
  if (newTables.length > 0) {
    if (!newTables.includes(activeTab.value)) {
      activeTab.value = newTables[0]
    }
  }
  else {
    activeTab.value = ''
  }

  // Emit updated state
  syncToParent()
}, { immediate: true, deep: true })

async function fetchColumnsForTable(tableName: string) {
  loadingTables.value.add(tableName)
  try {
    const result = await invoke<ColumnInfo[]>('list_columns', {
      connectionId: props.connectionId,
      database: props.database,
      schema: props.schema || null,
      tableName,
    })
    const columns = result || []
    tableColumnsMap.value.set(tableName, {
      columns,
      selected: columns.map(c => c.name), // Select all by default
    })
    // Clear any previous error for this table
    tableErrors.value.delete(tableName)
  }
  catch (error) {
    console.error(`Failed to fetch columns for ${tableName}:`, error)
    const errorMsg = String(error)
    tableErrors.value.set(tableName, errorMsg)
    tableColumnsMap.value.set(tableName, { columns: [], selected: [] })
    toast.error(t('transfer.export.columns.loadFailed', 'Failed to load columns'), { description: errorMsg })
  }
  finally {
    loadingTables.value.delete(tableName)
  }
}

function syncToParent() {
  const result: TableColumns[] = []
  props.selectedTables?.forEach((tableName) => {
    const data = tableColumnsMap.value.get(tableName)
    if (data) {
      result.push({
        tableName,
        columns: data.columns,
        selectedColumns: data.selected,
      })
    }
  })
  tableColumnsValue.value = result
}

const getTableData = (tableName: string) => tableColumnsMap.value.get(tableName) ?? null
const isLoadingTable = (tableName: string) => loadingTables.value.has(tableName)
const getTableError = (tableName: string) => tableErrors.value.get(tableName)

// Active tab helpers (used by select/deselect/toggle which operate on active tab)
const getCurrentTableData = computed(() => activeTab.value ? getTableData(activeTab.value) : null)

function selectAllColumns() {
  if (!getCurrentTableData.value)
    return
  getCurrentTableData.value.selected = getCurrentTableData.value.columns.map(c => c.name)
  syncToParent()
}

function deselectAllColumns() {
  if (!getCurrentTableData.value)
    return
  getCurrentTableData.value.selected = []
  syncToParent()
}

function toggleColumn(columnName: string) {
  if (!getCurrentTableData.value)
    return
  const current = [...getCurrentTableData.value.selected]
  const index = current.indexOf(columnName)
  if (index > -1) {
    current.splice(index, 1)
  }
  else {
    current.push(columnName)
  }
  getCurrentTableData.value.selected = current
  syncToParent()
}

const isColumnSelected = (columnName: string) => getCurrentTableData.value?.selected.includes(columnName) ?? false

const currentSelectionCount = computed(() => getCurrentTableData.value?.selected.length ?? 0)
const currentTotalCount = computed(() => getCurrentTableData.value?.columns.length ?? 0)
const allCurrentSelected = computed(() => currentTotalCount.value > 0 && currentSelectionCount.value === currentTotalCount.value)
const someCurrentSelected = computed(() => currentSelectionCount.value > 0 && currentSelectionCount.value < currentTotalCount.value)

// Total summary across all tables
const totalSelectedColumns = computed(() => {
  let count = 0
  tableColumnsMap.value.forEach((data) => {
    count += data.selected.length
  })
  return count
})

const totalAvailableColumns = computed(() => {
  let count = 0
  tableColumnsMap.value.forEach((data) => {
    count += data.columns.length
  })
  return count
})

const selectedTablesCount = computed(() => props.selectedTables?.length ?? 0)

const showTotalSummary = computed(() => selectedTablesCount.value > 1)
</script>

<template>
  <div class="flex flex-col h-[280px]">
    <!-- Content Area -->
    <div class="flex flex-1 flex-col">
      <!-- Empty State: Show structure with tabs -->
      <div v-if="!selectedTables?.length" class="flex flex-1 flex-col">
        <!-- Empty Table Structure -->
        <div class="border border-border/40 rounded-md flex-1 overflow-hidden">
          <table class="text-xs w-full">
            <thead class="text-[10px] text-muted-foreground tracking-wide border-b border-border/40 bg-muted/40 uppercase">
              <tr>
                <th scope="col" class="font-medium px-2 py-1.5 text-left w-8">
                  <Checkbox :checked="false" class="h-3.5 w-3.5" disabled />
                </th>
                <th scope="col" class="font-medium px-2 py-1.5 text-left">
                  Column Name
                </th>
                <th scope="col" class="font-medium px-2 py-1.5 text-left">
                  Type
                </th>
                <th scope="col" class="font-medium px-2 py-1.5 text-center w-16">
                  Nullable
                </th>
                <th scope="col" class="font-medium px-2 py-1.5 text-center w-16">
                  Key
                </th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td colspan="5" class="text-[11px] text-muted-foreground px-2 py-6 text-center italic">
                  <span class="i-carbon-data-base mr-1.5 opacity-50" /> {{ t('transfer.export.selectTablesFirst', 'Select tables above to configure columns') }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Selected Tables: Show tabs and content -->
      <Tabs v-else v-model="activeTab" class="flex shrink flex-col h-full">
        <!-- Tabs List -->
        <TabsList class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border p-1 border border-border/40 rounded-lg bg-muted/50 shrink-0 flex-nowrap gap-1 h-9 justify-start overflow-x-auto">
          <TabsTrigger
            v-for="tableName in selectedTables"
            :key="tableName"
            :value="tableName"
            class="text-xs font-medium px-3 rounded-md flex gap-1.5 h-7 transition-colors items-center justify-center data-[state=active]:text-primary data-[state=active]:bg-background data-[state=active]:shadow-sm"
          >
            <span class="font-mono max-w-[100px] truncate">{{ tableName }}</span>
            <span v-if="tableColumnsMap.get(tableName)" class="text-[10px] text-muted-foreground font-mono">
              {{ tableColumnsMap.get(tableName)?.selected.length }}
            </span>
          </TabsTrigger>
        </TabsList>

        <!-- Tab Content -->
        <TabsContent
          v-for="tableName in selectedTables"
          :key="tableName"
          :value="tableName"
          class="mt-2 shrink overflow-hidden"
        >
          <div v-if="getTableError(tableName)" class="p-4 border border-red-200 rounded-md bg-red-50 flex-1 dark:border-red-800 dark:bg-red-900/20">
            <div class="flex gap-3 items-start">
              <span class="i-carbon-warning-alt text-lg text-red-600 shrink-0 dark:text-red-400" />
              <div>
                <p class="text-sm text-red-800 font-medium dark:text-red-200">
                  {{ t('transfer.export.columns.loadFailed', 'Failed to load columns') }}
                </p>
                <p class="text-xs text-red-700 mt-1 dark:text-red-300">
                  {{ getTableError(tableName) }}
                </p>
              </div>
            </div>
          </div>

          <div v-else-if="isLoadingTable(tableName)" class="text-[11px] text-muted-foreground p-4 border rounded-md border-dashed bg-muted/20 flex flex-1 items-center justify-center">
            <span class="i-carbon-circle-dash mr-1.5 animate-spin" /> Loading columns...
          </div>

          <div v-else-if="getTableData(tableName)?.columns.length === 0" class="border border-border/40 rounded-md flex flex-1 flex-col overflow-hidden">
            <table class="text-xs w-full">
              <thead class="text-[10px] text-muted-foreground tracking-wide border-b border-border/40 bg-muted/40 uppercase">
                <tr>
                  <th scope="col" class="font-medium px-2 py-1.5 text-left w-8">
                    <Checkbox :checked="false" class="h-3.5 w-3.5" disabled />
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-left">
                    Column Name
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-left">
                    Type
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-center w-16">
                    Nullable
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-center w-16">
                    Key
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td colspan="5" class="text-[11px] text-muted-foreground px-2 py-6 text-center italic">
                    <span class="i-carbon-data-base mr-1.5 opacity-50" /> No columns found
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div v-else class="border border-border/40 rounded-md overflow-hidden">
            <!-- Static header table -->
            <table class="text-xs w-full">
              <thead class="text-[10px] text-muted-foreground tracking-wide border-b border-border/40 bg-muted/40 uppercase">
                <tr>
                  <th scope="col" class="font-medium px-2 py-1.5 text-left w-8">
                    <Checkbox
                      :checked="allCurrentSelected"
                      :indeterminate="someCurrentSelected"
                      class="h-3.5 w-3.5"
                      @update:checked="allCurrentSelected ? deselectAllColumns() : selectAllColumns()"
                    />
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-left">
                    Column Name
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-left">
                    Type
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-center w-16">
                    Nullable
                  </th>
                  <th scope="col" class="font-medium px-2 py-1.5 text-center w-16">
                    Key
                  </th>
                </tr>
              </thead>
            </table>
            <!-- Scrollable body table -->
            <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border max-h-[180px] overflow-y-auto">
              <table class="text-xs w-full">
                <tbody>
                  <tr
                    v-for="col in getTableData(tableName)?.columns"
                    :key="col.name"
                    class="border-b border-border/40 transition-colors last:border-0 hover:bg-muted/40"
                    :class="isColumnSelected(col.name) ? 'bg-primary/[0.03]' : ''"
                  >
                    <td class="px-2 py-1.5 align-middle w-8">
                      <Checkbox
                        :checked="isColumnSelected(col.name)"
                        class="h-3.5 w-3.5"
                        @update:checked="toggleColumn(col.name)"
                      />
                    </td>
                    <th scope="row" class="font-mono font-normal px-2 py-1.5 text-left align-middle max-w-[140px] truncate">
                      {{ col.name }}
                    </th>
                    <td class="text-[10px] text-muted-foreground font-mono px-2 py-1.5 text-left align-middle max-w-[100px] truncate uppercase">
                      {{ col.data_type || '-' }}
                    </td>
                    <td class="text-[10px] text-muted-foreground px-2 py-1.5 text-center align-middle w-16">
                      <span v-if="col.nullable" class="i-carbon-checkmark text-green-600" />
                      <span v-else class="i-carbon-close text-red-500" />
                    </td>
                    <td class="text-[10px] px-2 py-1.5 text-center align-middle w-16">
                      <span v-if="col.is_primary_key" class="text-[10px] text-yellow-600 tracking-wide font-medium px-1 py-0.5 rounded bg-yellow-500/10 uppercase">
                        PK
                      </span>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div class="text-[10px] text-muted-foreground font-mono mt-2 shrink-0">
            {{ currentSelectionCount }} of {{ currentTotalCount }} columns selected for {{ tableName }}
          </div>
        </TabsContent>
      </Tabs>
    </div>

    <!-- Total Summary Footer -->
    <div v-if="showTotalSummary" class="text-[10px] text-muted-foreground font-mono pt-2 border-t border-border/40 shrink-0">
      Total: {{ totalSelectedColumns }} of {{ totalAvailableColumns }} columns across {{ selectedTablesCount }} tables
    </div>
  </div>
</template>
