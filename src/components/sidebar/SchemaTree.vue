<script setup lang="ts">
import type { ObjectInfo } from '@/datasources/browseApi'
import type { TableInfo } from '@/store/databaseStore'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ConnectionStatus, DatabaseType, useConnectionStore, useDatabaseStore } from '@/store'
import { getDbCapabilities } from './dbCapabilities'
import TreeGroup from './TreeGroup.vue'

type Props = {
  connectionId: string | null
  selectedDatabase: string | null
  selectedSchema: string | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'selectTable', table: TableInfo, database: string, schema?: string): void
  (e: 'createScript', table: TableInfo, database: string, schema?: string): void
  (e: 'selectTopN', table: TableInfo, database: string, schema?: string, n?: number): void
  (e: 'viewStructure', table: TableInfo, database: string, schema?: string): void
  (e: 'exportData', table: TableInfo, database: string, schema?: string): void
  (e: 'showErDiagram', database: string, schema?: string): void
  (e: 'dropTable', table: TableInfo, database: string, schema?: string): void
  (e: 'truncateTable', table: TableInfo, database: string, schema?: string): void
  (e: 'update:selectedDatabase', database: string): void
  (e: 'update:selectedSchema', schema: string): void
  (e: 'openListingTab', type: string, database: string, schema?: string): void
  (e: 'openDdlTab', name: string, type: string, database: string, schema?: string): void
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()
const databaseStore = useDatabaseStore()

const searchQuery = ref('')
const showViews = ref(false)
const showProcedures = ref(false)
const showFunctions = ref(false)

// Expanded database nodes (Mode B)
const expandedDatabases = ref<Set<string>>(new Set())
const loadingDatabases = ref<Set<string>>(new Set())

// Context menu
const contextMenuTable = ref<{ table: TableInfo, database: string, schema?: string, x: number, y: number } | null>(null)

const activeConnection = computed(() =>
  props.connectionId ? connectionStore.getConnectionById(props.connectionId) : null,
)

const isActiveConnectionConnected = computed(() =>
  props.connectionId
    ? connectionStore.getConnectionStatus(props.connectionId) === ConnectionStatus.CONNECTED
    : false,
)

const supportsSchemas = computed(() => {
  if (!props.selectedDatabase || !props.connectionId)
    return false
  const meta = databaseStore.metadata[props.connectionId]
  if (!meta)
    return false
  const schemas = meta.schemas[props.selectedDatabase]
  return schemas !== undefined && schemas.length > 0
})

const availableSchemas = computed<string[]>(() => {
  if (!props.selectedDatabase || !props.connectionId || !supportsSchemas.value)
    return []
  return databaseStore.metadata[props.connectionId]?.schemas[props.selectedDatabase] ?? []
})

const capabilities = computed(() => {
  if (!activeConnection.value)
    return getDbCapabilities(DatabaseType.POSTGRESQL)
  return getDbCapabilities(activeConnection.value.type)
})

const tables = computed<TableInfo[]>(() => {
  const currentDb = props.selectedDatabase
  if (!currentDb || !props.connectionId || !databaseStore.metadata[props.connectionId])
    return []
  const meta = databaseStore.metadata[props.connectionId]
  const schemas = meta.schemas[currentDb] || []
  const allItems = schemas.length > 0
    ? (() => {
        const schema = props.selectedSchema || schemas[0]
        const tablesKey = `${currentDb}.${schema}`
        return meta.tables[tablesKey] || []
      })()
    : meta.tables[currentDb] || []
  const query = searchQuery.value.toLowerCase().trim()
  return query ? allItems.filter(item => item.name.toLowerCase().includes(query)) : allItems
})

const schemaObjects = computed(() => {
  if (!props.connectionId || !props.selectedDatabase || !props.selectedSchema)
    return null
  return databaseStore.getSchemaObjects(props.connectionId, props.selectedDatabase, props.selectedSchema)
})

const allDatabases = computed(() => {
  if (!props.connectionId || !databaseStore.metadata[props.connectionId])
    return []
  return databaseStore.metadata[props.connectionId].databases
})

const selectedSchemaValue = computed({
  get: () => props.selectedSchema || availableSchemas.value[0] || '',
  set: (val: string) => emit('update:selectedSchema', val),
})

// Mode B helpers
function getDbSchemas(dbName: string): string[] {
  if (!props.connectionId)
    return []
  const meta = databaseStore.metadata[props.connectionId]
  if (!meta)
    return []
  return meta.schemas[dbName] || []
}

function getTablesForDbSchema(dbName: string, schema?: string): TableInfo[] {
  if (!props.connectionId)
    return []
  const meta = databaseStore.metadata[props.connectionId]
  if (!meta)
    return []
  const key = schema ? `${dbName}.${schema}` : dbName
  return meta.tables[key] || []
}

// Toggle functions
async function toggleViews() {
  showViews.value = !showViews.value
  if (showViews.value && props.connectionId && props.selectedDatabase && props.selectedSchema)
    await databaseStore.fetchSchemaObjects(props.connectionId, props.selectedDatabase, props.selectedSchema)
}

async function toggleProcedures() {
  showProcedures.value = !showProcedures.value
  if (showProcedures.value && props.connectionId && props.selectedDatabase && props.selectedSchema)
    await databaseStore.fetchSchemaObjects(props.connectionId, props.selectedDatabase, props.selectedSchema)
}

async function toggleFunctions() {
  showFunctions.value = !showFunctions.value
  if (showFunctions.value && props.connectionId && props.selectedDatabase && props.selectedSchema)
    await databaseStore.fetchSchemaObjects(props.connectionId, props.selectedDatabase, props.selectedSchema)
}

// Table click handlers
function handleTableClick(table: TableInfo) {
  const schema = props.selectedSchema || (supportsSchemas.value ? availableSchemas.value[0] : undefined)
  emit('selectTable', table, props.selectedDatabase!, schema)
}

function handleDoubleClick(table: TableInfo) {
  const schema = props.selectedSchema || (supportsSchemas.value ? availableSchemas.value[0] : undefined)
  emit('viewStructure', table, props.selectedDatabase!, schema)
}

function handleContextMenu(event: MouseEvent, table: TableInfo) {
  event.preventDefault()
  const schema = props.selectedSchema || (supportsSchemas.value ? availableSchemas.value[0] : undefined)
  contextMenuTable.value = { table, database: props.selectedDatabase!, schema, x: event.clientX, y: event.clientY }
}

// Context menu action handler
type ContextAction = 'selectTopN' | 'viewStructure' | 'createScript' | 'exportData' | 'showErDiagram' | 'dropTable' | 'truncateTable'

function handleContextAction(action: ContextAction) {
  if (!contextMenuTable.value)
    return
  const { table, database, schema } = contextMenuTable.value
  const handlers: Record<ContextAction, () => void> = {
    selectTopN: () => emit('selectTopN', table, database, schema, 100),
    viewStructure: () => emit('viewStructure', table, database, schema),
    createScript: () => emit('createScript', table, database, schema),
    exportData: () => emit('exportData', table, database, schema),
    showErDiagram: () => emit('showErDiagram', database, schema),
    dropTable: () => emit('dropTable', table, database, schema),
    truncateTable: () => emit('truncateTable', table, database, schema),
  }
  handlers[action]()
  contextMenuTable.value = null
}

// View/Procedure/Function click handlers
function handleViewClick(view: ObjectInfo) {
  if (props.connectionId && props.selectedDatabase)
    emit('openDdlTab', view.name, 'VIEW', props.selectedDatabase, props.selectedSchema ?? undefined)
}

function handleProcedureClick(proc: ObjectInfo) {
  if (props.connectionId && props.selectedDatabase)
    emit('openDdlTab', proc.name, 'PROCEDURE', props.selectedDatabase, props.selectedSchema ?? undefined)
}

function handleFunctionClick(fn: ObjectInfo) {
  if (props.connectionId && props.selectedDatabase)
    emit('openDdlTab', fn.name, 'FUNCTION', props.selectedDatabase, props.selectedSchema ?? undefined)
}

function handleOpenListingTab(type: 'VIEW' | 'PROCEDURE' | 'FUNCTION') {
  if (props.connectionId && props.selectedDatabase)
    emit('openListingTab', type, props.selectedDatabase, props.selectedSchema ?? undefined)
}

// Schema selection handler
function handleSchemaSelect(schema: string) {
  emit('update:selectedSchema', schema)
}

// Mode B — expand database node (lazy load)
async function toggleDatabaseNode(dbName: string) {
  if (expandedDatabases.value.has(dbName)) {
    expandedDatabases.value = new Set([...expandedDatabases.value].filter(n => n !== dbName))
    return
  }
  expandedDatabases.value = new Set([...expandedDatabases.value, dbName])
  if (!props.connectionId)
    return
  const connId = props.connectionId
  if (!connId)
    return
  loadingDatabases.value = new Set([...loadingDatabases.value, dbName])
  try {
    await databaseStore.fetchSchemas(connId, dbName)
    const meta = databaseStore.metadata[connId]
    const schemas = meta?.schemas[dbName] || []
    if (schemas.length > 0)
      await Promise.all(schemas.map(s => databaseStore.fetchTables(connId, dbName, s)))
    else
      await databaseStore.fetchTables(connId, dbName)
  }
  finally {
    loadingDatabases.value = new Set([...loadingDatabases.value].filter(n => n !== dbName))
  }
}

// Watchers
watch(() => props.connectionId, async (newId) => {
  if (!newId)
    return
  const conn = connectionStore.getConnectionById(newId)
  if (!conn?.isConnected)
    return
  await databaseStore.fetchDatabases(newId)
  const connDb = conn.database || connectionStore.getCurrentDatabase(newId)
  if (connDb && !props.selectedDatabase)
    emit('update:selectedDatabase', connDb)
}, { immediate: true })

watch(() => activeConnection.value?.isConnected, async (isConnected) => {
  if (!isConnected || !props.connectionId)
    return
  await databaseStore.fetchDatabases(props.connectionId)
  const meta = databaseStore.metadata[props.connectionId]
  const firstDb = meta?.databases.find(db => !db.is_system)?.name
  const connDb = activeConnection.value?.database || connectionStore.getCurrentDatabase(props.connectionId) || firstDb
  if (connDb && !props.selectedDatabase)
    emit('update:selectedDatabase', connDb)
})

watch(() => props.selectedDatabase, async (newDb, oldDb) => {
  const connId = props.connectionId
  if (!newDb || !connId || newDb === oldDb)
    return
  await databaseStore.fetchSchemas(connId, newDb)
  const meta = databaseStore.metadata[connId]
  const schemas = meta?.schemas[newDb] || []
  if (schemas.length > 0)
    await Promise.all(schemas.map(s => databaseStore.fetchTables(connId, newDb, s)))
  else
    await databaseStore.fetchTables(connId, newDb)
})

watch(() => props.selectedSchema, async (newSchema, oldSchema) => {
  if (!newSchema || !props.selectedDatabase || !props.connectionId || newSchema === oldSchema)
    return
  const tablesKey = `${props.selectedDatabase}.${newSchema}`
  const meta = databaseStore.metadata[props.connectionId]
  if (!meta?.tables[tablesKey])
    await databaseStore.fetchTables(props.connectionId, props.selectedDatabase, newSchema)
})

// Refresh function (exposed to parent)
async function refresh() {
  const connId = props.connectionId
  if (!connId)
    return
  const conn = connectionStore.getConnectionById(connId)
  if (!conn?.isConnected)
    return
  databaseStore.clearMetadata(connId)
  await databaseStore.fetchDatabases(connId)
  const connDb = props.selectedDatabase || connectionStore.getCurrentDatabase(connId) || conn.database
  if (connDb) {
    await databaseStore.fetchSchemas(connId, connDb)
    const meta = databaseStore.metadata[connId]
    const schemas = meta?.schemas[connDb] || []
    if (schemas.length > 0)
      await Promise.all(schemas.map(s => databaseStore.fetchTables(connId, connDb, s)))
    else
      await databaseStore.fetchTables(connId, connDb)
  }
}

defineExpose({ refresh })
</script>

<template>
  <div class="flex flex-col h-full" @click="contextMenuTable = null">
    <!-- Search -->
    <div v-if="props.selectedDatabase" class="px-2 py-1.5 border-b relative">
      <span class="i-carbon-search text-muted-foreground h-3.5 w-3.5 left-4 top-1/2 absolute -translate-y-1/2" />
      <Input v-model="searchQuery" :placeholder="t('sidebar.search')" class="text-xs pl-7 h-7" />
    </div>

    <!-- Loading state -->
    <div v-if="databaseStore.loading" class="text-sm text-muted-foreground py-4 text-center">
      <span class="i-carbon-loading mx-auto mb-2 h-5 w-5 block animate-spin" />
      <p>{{ t('sidebar.loading') }}</p>
    </div>

    <!-- Mode A: Database selected -->
    <div v-else-if="isActiveConnectionConnected && props.selectedDatabase" class="flex-1 overflow-auto">
      <!-- Schema selector (if schema-aware) -->
      <div v-if="supportsSchemas && availableSchemas.length > 0" class="px-2 py-1 border-b">
        <Select :model-value="selectedSchemaValue" @update:model-value="handleSchemaSelect">
          <SelectTrigger class="text-xs h-7">
            <SelectValue :placeholder="t('sidebar.database.selectDatabase')" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="schema in availableSchemas" :key="schema" :value="schema">
              {{ schema }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      <!-- TABLES group (open by default) -->
      <TreeGroup
        :label="t('sidebar.groups.tables')"
        icon="i-carbon-table"
        icon-color="text-green-500"
        :count="tables.length"
        :default-open="true"
      >
        <div
          v-for="table in tables"
          :key="table.name"
          class="text-sm px-2 py-1 flex gap-2 cursor-pointer items-center hover:bg-accent"
          @click="handleTableClick(table)"
          @dblclick="handleDoubleClick(table)"
          @contextmenu="handleContextMenu($event, table)"
        >
          <span class="i-carbon-table text-green-500 shrink-0 h-3.5 w-3.5" />
          <span class="truncate">{{ table.name }}</span>
        </div>
        <div v-if="tables.length === 0" class="text-xs text-muted-foreground px-2 py-2">
          {{ t('sidebar.noObjects') }}
        </div>
      </TreeGroup>

      <!-- VIEWS group (closed by default) -->
      <TreeGroup
        v-if="capabilities.views"
        :label="t('sidebar.groups.views')"
        icon="i-carbon-view"
        icon-color="text-purple-500"
        :count="schemaObjects?.views.length ?? 0"
        :default-open="false"
        @toggle="toggleViews"
      >
        <div
          v-for="view in schemaObjects?.views ?? []"
          :key="view.name"
          class="text-sm px-2 py-1 flex gap-2 cursor-pointer items-center hover:bg-accent"
          @click="handleViewClick(view)"
          @dblclick="handleOpenListingTab('VIEW')"
        >
          <span
            class="shrink-0 h-3.5 w-3.5"
            :class="view.object_type?.toUpperCase().includes('MATERIALIZED')
              ? 'i-carbon-view text-indigo-500'
              : 'i-carbon-view text-purple-500'"
          />
          <span class="truncate">{{ view.name }}</span>
        </div>
        <div v-if="!schemaObjects?.views || schemaObjects.views.length === 0" class="text-xs text-muted-foreground px-2 py-2">
          {{ t('sidebar.noObjects') }}
        </div>
      </TreeGroup>

      <!-- PROCEDURES group -->
      <TreeGroup
        v-if="capabilities.procedures"
        :label="t('sidebar.groups.procedures')"
        icon="i-carbon-document"
        icon-color="text-blue-500"
        :count="schemaObjects?.procedures.length ?? 0"
        :default-open="false"
        @toggle="toggleProcedures"
      >
        <div
          v-for="proc in schemaObjects?.procedures ?? []"
          :key="proc.name"
          class="text-sm px-2 py-1 flex gap-2 cursor-pointer items-center hover:bg-accent"
          @click="handleProcedureClick(proc)"
          @dblclick="handleOpenListingTab('PROCEDURE')"
        >
          <span class="i-carbon-document text-blue-500 shrink-0 h-3.5 w-3.5" />
          <span class="truncate">{{ proc.name }}</span>
        </div>
        <div v-if="!schemaObjects?.procedures || schemaObjects.procedures.length === 0" class="text-xs text-muted-foreground px-2 py-2">
          {{ t('sidebar.noObjects') }}
        </div>
      </TreeGroup>

      <!-- FUNCTIONS group -->
      <TreeGroup
        v-if="capabilities.functions"
        :label="t('sidebar.groups.functions')"
        icon="i-carbon-function-math"
        icon-color="text-amber-500"
        :count="schemaObjects?.functions.length ?? 0"
        :default-open="false"
        @toggle="toggleFunctions"
      >
        <div
          v-for="fn in schemaObjects?.functions ?? []"
          :key="fn.name"
          class="text-sm px-2 py-1 flex gap-2 cursor-pointer items-center hover:bg-accent"
          @click="handleFunctionClick(fn)"
          @dblclick="handleOpenListingTab('FUNCTION')"
        >
          <span class="i-carbon-function-math text-amber-500 shrink-0 h-3.5 w-3.5" />
          <span class="truncate">{{ fn.name }}</span>
        </div>
        <div v-if="!schemaObjects?.functions || schemaObjects.functions.length === 0" class="text-xs text-muted-foreground px-2 py-2">
          {{ t('sidebar.noObjects') }}
        </div>
      </TreeGroup>
    </div>

    <!-- Mode B: All Databases -->
    <div v-else-if="isActiveConnectionConnected && !props.selectedDatabase" class="flex-1 overflow-auto">
      <div v-for="db in allDatabases" :key="db.name" class="border-b last:border-b-0">
        <button
          class="text-sm px-2 py-1.5 flex gap-2 w-full cursor-pointer items-center hover:bg-accent/50"
          @click="toggleDatabaseNode(db.name)"
        >
          <span class="i-carbon-chevron-right shrink-0 h-3 w-3 transition-transform" :class="{ 'rotate-90': expandedDatabases.has(db.name) }" />
          <span class="i-carbon-data-base text-yellow-500 shrink-0 h-3.5 w-3.5" />
          <span class="flex-1 truncate">{{ db.name }}</span>
          <span v-if="loadingDatabases.has(db.name)" class="i-carbon-loading h-3 w-3 animate-spin" />
        </button>
        <div v-if="expandedDatabases.has(db.name)" class="py-1">
          <!-- For each database, show schema nodes or direct groups -->
          <template v-if="getDbSchemas(db.name).length > 0">
            <div v-for="schema in getDbSchemas(db.name)" :key="schema" class="ml-4 border-b last:border-b-0">
              <div class="text-xs text-muted-foreground font-semibold px-2 py-1 flex gap-2 items-center">
                <span class="i-carbon-folder-open text-sky-400 shrink-0 h-3.5 w-3.5" />
                <span>{{ schema }}</span>
              </div>
              <TreeGroup :label="t('sidebar.groups.tables')" icon="i-carbon-table" icon-color="text-green-500" :count="getTablesForDbSchema(db.name, schema).length" :default-open="true">
                <div
                  v-for="table in getTablesForDbSchema(db.name, schema)"
                  :key="table.name"
                  class="text-sm ml-4 px-2 py-1 flex gap-2 cursor-pointer items-center hover:bg-accent"
                  @click="emit('selectTable', table, db.name, schema)"
                  @dblclick="emit('viewStructure', table, db.name, schema)"
                  @contextmenu="(e) => { e.preventDefault(); contextMenuTable = { table, database: db.name, schema, x: e.clientX, y: e.clientY } }"
                >
                  <span class="i-carbon-table text-green-500 shrink-0 h-3.5 w-3.5" />
                  <span class="truncate">{{ table.name }}</span>
                </div>
              </TreeGroup>
            </div>
          </template>
          <template v-else>
            <TreeGroup :label="t('sidebar.groups.tables')" icon="i-carbon-table" icon-color="text-green-500" :count="getTablesForDbSchema(db.name).length" :default-open="true">
              <div
                v-for="table in getTablesForDbSchema(db.name)"
                :key="table.name"
                class="text-sm ml-4 px-2 py-1 flex gap-2 cursor-pointer items-center hover:bg-accent"
                @click="emit('selectTable', table, db.name)"
                @dblclick="emit('viewStructure', table, db.name)"
                @contextmenu="(e) => { e.preventDefault(); contextMenuTable = { table, database: db.name, x: e.clientX, y: e.clientY } }"
              >
                <span class="i-carbon-table text-green-500 shrink-0 h-3.5 w-3.5" />
                <span class="truncate">{{ table.name }}</span>
              </div>
            </TreeGroup>
          </template>
        </div>
      </div>
      <div v-if="allDatabases.length === 0" class="text-xs text-muted-foreground px-2 py-2">
        {{ t('sidebar.noDatabases') }}
      </div>
    </div>

    <!-- Not connected -->
    <div v-else class="p-4 flex flex-1 items-center justify-center">
      <p class="text-xs text-muted-foreground text-center">
        {{ t('sidebar.noDatabases') }}
      </p>
    </div>

    <!-- Context Menu -->
    <div
      v-if="contextMenuTable"
      class="text-popover-foreground border rounded-md bg-popover w-48 shadow-md fixed z-50"
      :style="{ left: `${contextMenuTable.x}px`, top: `${contextMenuTable.y}px` }"
      @click.stop
    >
      <div class="p-1">
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleContextAction('selectTopN')"
        >
          <span class="i-carbon-arrow-down mr-2 h-3.5 w-3.5" />
          {{ t('components.databaseBrowser.contextMenu.selectTopN') }}
        </div>
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleContextAction('viewStructure')"
        >
          <span class="i-carbon-table mr-2 h-3.5 w-3.5" />
          {{ t('components.databaseBrowser.contextMenu.viewStructure') }}
        </div>
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleContextAction('createScript')"
        >
          <span class="i-carbon-code mr-2 h-3.5 w-3.5" />
          {{ t('components.databaseBrowser.contextMenu.createScript') }}
        </div>
        <div class="my-1 bg-border h-px" />
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleContextAction('showErDiagram')"
        >
          <span class="i-carbon-diagram mr-2 h-3.5 w-3.5" />
          {{ t('components.databaseBrowser.contextMenu.showErDiagram') }}
        </div>
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleContextAction('exportData')"
        >
          <span class="i-carbon-download mr-2 h-3.5 w-3.5" />
          {{ t('components.databaseBrowser.contextMenu.exportData') }}
        </div>
        <div class="my-1 bg-border h-px" />
        <div
          class="text-sm text-destructive px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-destructive-foreground hover:bg-destructive/10"
          @click="handleContextAction('dropTable')"
        >
          <span class="i-carbon-trash-can mr-2 shrink-0 h-3.5 w-3.5" />
          {{ t('components.databaseBrowser.contextMenu.dropTable') }}
        </div>
        <div
          class="text-sm text-destructive px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-destructive-foreground hover:bg-destructive/10"
          @click="handleContextAction('truncateTable')"
        >
          <span class="i-carbon-clean mr-2 shrink-0 h-3.5 w-3.5" />
          {{ t('components.databaseBrowser.contextMenu.truncateTable') }}
        </div>
      </div>
    </div>
  </div>
</template>
