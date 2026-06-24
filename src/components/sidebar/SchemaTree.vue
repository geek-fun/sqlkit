<script setup lang="ts">
import type { ObjectInfo } from '@/datasources/browseApi'
import type { TableInfo } from '@/store/databaseStore'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Input } from '@/components/ui/input'
import { ConnectionStatus, useConnectionStore, useDatabaseStore } from '@/store'
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
const expandedSchemas = ref<Set<string>>(new Set())

// Mode B state (all databases view)
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

const allDatabases = computed(() => {
  if (!props.connectionId || !databaseStore.metadata[props.connectionId])
    return []
  const seen = new Set<string>()
  return databaseStore.metadata[props.connectionId].databases.filter((db) => {
    if (seen.has(db.name))
      return false
    seen.add(db.name)
    return true
  })
})

// Per-schema tables with search filter
function getTablesForSchema(schema: string): TableInfo[] {
  if (!props.connectionId || !props.selectedDatabase)
    return []
  const key = `${props.selectedDatabase}.${schema}`
  const allItems = databaseStore.metadata[props.connectionId]?.tables[key] ?? []
  const query = searchQuery.value.toLowerCase().trim()
  return query ? allItems.filter(item => item.name.toLowerCase().includes(query)) : allItems
}

// Per-schema objects (views, procedures, functions)
function getObjectsForSchema(schema: string) {
  if (!props.connectionId || !props.selectedDatabase)
    return null
  return databaseStore.getSchemaObjects(props.connectionId, props.selectedDatabase, schema)
}

// Flat tables for non-schema databases (MySQL, SQLite, etc.)
const flatTables = computed<TableInfo[]>(() => {
  const currentDb = props.selectedDatabase
  if (!currentDb || !props.connectionId || !databaseStore.metadata[props.connectionId])
    return []
  const meta = databaseStore.metadata[props.connectionId]
  const allItems = meta.tables[currentDb] || []
  const query = searchQuery.value.toLowerCase().trim()
  return query ? allItems.filter(item => item.name.toLowerCase().includes(query)) : allItems
})

// Toggle schema node (expand/collapse + lazy load tables + set selectedSchema)
async function toggleSchema(schema: string) {
  emit('update:selectedSchema', schema)

  if (expandedSchemas.value.has(schema)) {
    expandedSchemas.value = new Set([...expandedSchemas.value].filter(s => s !== schema))
    return
  }
  expandedSchemas.value = new Set([...expandedSchemas.value, schema])

  // Lazy load tables for this schema
  if (props.connectionId && props.selectedDatabase) {
    const key = `${props.selectedDatabase}.${schema}`
    const meta = databaseStore.metadata[props.connectionId]
    if (!meta?.tables[key])
      await databaseStore.fetchTables(props.connectionId, props.selectedDatabase, schema)
  }
}

// Group toggle handler (lazy load schema objects for views/procedures/functions)
async function handleGroupToggle(open: boolean, schema: string) {
  if (open && props.connectionId && props.selectedDatabase)
    await databaseStore.fetchSchemaObjects(props.connectionId, props.selectedDatabase, schema)
}

// Table click handlers (now take schema from tree context)
function handleTableClick(table: TableInfo, schema: string | undefined) {
  emit('selectTable', table, props.selectedDatabase!, schema)
}

function handleDoubleClick(table: TableInfo, schema: string | undefined) {
  emit('viewStructure', table, props.selectedDatabase!, schema)
}

function handleContextMenu(event: MouseEvent, table: TableInfo, schema: string | undefined) {
  event.preventDefault()
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

// Per-schema view/procedure/function click handlers
function handleViewClick(view: ObjectInfo, schema: string) {
  if (props.connectionId && props.selectedDatabase)
    emit('openDdlTab', view.name, 'VIEW', props.selectedDatabase, schema)
}

function handleProcedureClick(proc: ObjectInfo, schema: string) {
  if (props.connectionId && props.selectedDatabase)
    emit('openDdlTab', proc.name, 'PROCEDURE', props.selectedDatabase, schema)
}

function handleFunctionClick(fn: ObjectInfo, schema: string) {
  if (props.connectionId && props.selectedDatabase)
    emit('openDdlTab', fn.name, 'FUNCTION', props.selectedDatabase, schema)
}

function handleOpenListingTab(type: 'VIEW' | 'PROCEDURE' | 'FUNCTION', schema: string) {
  if (props.connectionId && props.selectedDatabase)
    emit('openListingTab', type, props.selectedDatabase, schema)
}

// Mode B helpers
function getDbSchemas(dbName: string): string[] {
  if (!props.connectionId)
    return []
  const meta = databaseStore.metadata[props.connectionId]
  if (!meta)
    return []
  return meta.schemas[dbName] || []
}

// For MySQL-like databases, schemas = database names → skip schema layer
function hasRealSchemas(dbName: string): boolean {
  const schemas = getDbSchemas(dbName)
  if (schemas.length === 0)
    return false
  // If the only schema is the db name itself, it's not a real schema hierarchy
  return schemas.some(s => s !== dbName)
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

function getObjectsForDbSchema(dbName: string, schema: string) {
  if (!props.connectionId)
    return null
  return databaseStore.getSchemaObjects(props.connectionId, dbName, schema)
}

async function handleGroupToggleForDb(open: boolean, dbName: string, schema?: string) {
  if (open && props.connectionId && schema)
    await databaseStore.fetchSchemaObjects(props.connectionId, dbName, schema)
}

// Mode B: expand/collapse database node (ONLY expand/collapse, do NOT select database)
async function toggleDatabaseNode(dbName: string) {
  if (expandedDatabases.value.has(dbName)) {
    expandedDatabases.value = new Set([...expandedDatabases.value].filter(n => n !== dbName))
    return
  }
  expandedDatabases.value = new Set([...expandedDatabases.value, dbName])
  if (!props.connectionId)
    return
  const connId = props.connectionId
  loadingDatabases.value = new Set([...loadingDatabases.value, dbName])
  try {
    await databaseStore.fetchSchemas(connId, dbName)
    const meta = databaseStore.metadata[connId]
    const schemas = meta?.schemas[dbName] || []
    // For MySQL-like DBs, schemas mirror db names — fetch tables without schema
    // so they're stored at meta.tables[dbName] and match the v-else template read.
    const hasReal = schemas.some(s => s !== dbName)
    if (hasReal)
      await Promise.all(schemas.map(s => databaseStore.fetchTables(connId, dbName, s)))
    else
      await databaseStore.fetchTables(connId, dbName)
  }
  finally {
    loadingDatabases.value = new Set([...loadingDatabases.value].filter(n => n !== dbName))
  }
}

// Watchers — fetch databases but do NOT auto-select; user picks from selector
watch(() => props.connectionId, async (newId) => {
  if (!newId)
    return
  const conn = connectionStore.getConnectionById(newId)
  if (!conn?.isConnected)
    return
  await databaseStore.fetchDatabases(newId)
}, { immediate: true })

watch(() => activeConnection.value?.isConnected, async (isConnected) => {
  if (!isConnected || !props.connectionId)
    return
  await databaseStore.fetchDatabases(props.connectionId)
})

watch(() => props.selectedDatabase, async (newDb, oldDb) => {
  const connId = props.connectionId
  if (!newDb || !connId || newDb === oldDb)
    return
  // Reset expanded schemas on database change
  expandedSchemas.value = new Set()
  await databaseStore.fetchSchemas(connId, newDb)
  const meta = databaseStore.metadata[connId]
  const schemas = meta?.schemas[newDb] || []
  if (schemas.length > 0) {
    await Promise.all(schemas.map(s => databaseStore.fetchTables(connId, newDb, s)))
    // Auto-expand first schema
    expandedSchemas.value = new Set([schemas[0]])
    emit('update:selectedSchema', schemas[0])
  }
  else {
    await databaseStore.fetchTables(connId, newDb)
  }
})

// Auto-expand all schemas when searching
watch(searchQuery, (query) => {
  if (query && availableSchemas.value.length > 0)
    expandedSchemas.value = new Set(availableSchemas.value)
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
    if (schemas.length > 0) {
      await Promise.all(schemas.map(s => databaseStore.fetchTables(connId, connDb, s)))
      expandedSchemas.value = new Set([schemas[0]])
      emit('update:selectedSchema', schemas[0])
    }
    else {
      await databaseStore.fetchTables(connId, connDb)
    }
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

    <!-- Mode A: Database selected — show schema hierarchy -->
    <div v-else-if="isActiveConnectionConnected && props.selectedDatabase" class="flex-1 overflow-auto">
      <!-- Schema-aware: schema nodes with nested groups (DBeaver-style hierarchy) -->
      <template v-if="supportsSchemas && availableSchemas.length > 0">
        <div v-for="schema in availableSchemas" :key="schema">
          <!-- Schema node (expandable, left-aligned, no border) -->
          <button
            class="text-sm px-2 py-1 flex gap-1.5 w-full cursor-pointer items-center hover:bg-accent/40"
            @click="toggleSchema(schema)"
          >
            <span class="i-carbon-chevron-right shrink-0 h-3 w-3 transition-transform" :class="{ 'rotate-90': expandedSchemas.has(schema) }" />
            <span
              class="shrink-0 h-3.5 w-3.5"
              :class="expandedSchemas.has(schema)
                ? 'i-carbon-folder-open text-sky-500'
                : 'i-carbon-folder text-muted-foreground'"
            />
            <span class="text-left flex-1 truncate">{{ schema }}</span>
          </button>

          <!-- Schema content (when expanded) -->
          <div v-if="expandedSchemas.has(schema)" class="ml-3">
            <!-- TABLES group (open by default) -->
            <TreeGroup
              :label="t('sidebar.groups.tables')"
              icon="i-carbon-folder"
              icon-color="text-green-600"
              :count="getTablesForSchema(schema).length"
              :default-open="true"
            >
              <div
                v-for="table in getTablesForSchema(schema)"
                :key="table.name"
                class="text-sm ml-2 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                @click="handleTableClick(table, schema)"
                @dblclick="handleDoubleClick(table, schema)"
                @contextmenu="handleContextMenu($event, table, schema)"
              >
                <span class="i-carbon-table text-green-500 shrink-0 h-3.5 w-3.5" />
                <span class="text-left truncate">{{ table.name }}</span>
              </div>
              <div v-if="getTablesForSchema(schema).length === 0" class="text-xs text-muted-foreground ml-2 px-2 py-1">
                {{ t('sidebar.noObjects') }}
              </div>
            </TreeGroup>

            <!-- VIEWS group -->
            <TreeGroup
              :label="t('sidebar.groups.views')"
              icon="i-carbon-folder"
              icon-color="text-purple-600"
              :count="getObjectsForSchema(schema)?.views.length ?? 0"
              :default-open="false"
              @toggle="(open: boolean) => handleGroupToggle(open, schema)"
            >
              <div
                v-for="view in getObjectsForSchema(schema)?.views ?? []"
                :key="view.name"
                class="text-sm ml-2 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                @click="handleViewClick(view, schema)"
                @dblclick="handleOpenListingTab('VIEW', schema)"
              >
                <span
                  class="shrink-0 h-3.5 w-3.5"
                  :class="view.object_type?.toUpperCase().includes('MATERIALIZED')
                    ? 'i-carbon-view text-indigo-500'
                    : 'i-carbon-view text-purple-500'"
                />
                <span class="text-left truncate">{{ view.name }}</span>
              </div>
              <div v-if="!getObjectsForSchema(schema)?.views || getObjectsForSchema(schema)!.views.length === 0" class="text-xs text-muted-foreground ml-2 px-2 py-1">
                {{ t('sidebar.noObjects') }}
              </div>
            </TreeGroup>

            <!-- PROCEDURES group -->
            <TreeGroup
              :label="t('sidebar.groups.procedures')"
              icon="i-carbon-folder"
              icon-color="text-blue-600"
              :count="getObjectsForSchema(schema)?.procedures.length ?? 0"
              :default-open="false"
              @toggle="(open: boolean) => handleGroupToggle(open, schema)"
            >
              <div
                v-for="proc in getObjectsForSchema(schema)?.procedures ?? []"
                :key="proc.name"
                class="text-sm ml-2 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                @click="handleProcedureClick(proc, schema)"
                @dblclick="handleOpenListingTab('PROCEDURE', schema)"
              >
                <span class="i-carbon-document text-blue-500 shrink-0 h-3.5 w-3.5" />
                <span class="text-left truncate">{{ proc.name }}</span>
              </div>
              <div v-if="!getObjectsForSchema(schema)?.procedures || getObjectsForSchema(schema)!.procedures.length === 0" class="text-xs text-muted-foreground ml-2 px-2 py-1">
                {{ t('sidebar.noObjects') }}
              </div>
            </TreeGroup>

            <!-- FUNCTIONS group -->
            <TreeGroup
              :label="t('sidebar.groups.functions')"
              icon="i-carbon-folder"
              icon-color="text-amber-600"
              :count="getObjectsForSchema(schema)?.functions.length ?? 0"
              :default-open="false"
              @toggle="(open: boolean) => handleGroupToggle(open, schema)"
            >
              <div
                v-for="fn in getObjectsForSchema(schema)?.functions ?? []"
                :key="fn.name"
                class="text-sm ml-2 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                @click="handleFunctionClick(fn, schema)"
                @dblclick="handleOpenListingTab('FUNCTION', schema)"
              >
                <span class="i-carbon-function-math text-amber-500 shrink-0 h-3.5 w-3.5" />
                <span class="text-left truncate">{{ fn.name }}</span>
              </div>
              <div v-if="!getObjectsForSchema(schema)?.functions || getObjectsForSchema(schema)!.functions.length === 0" class="text-xs text-muted-foreground ml-2 px-2 py-1">
                {{ t('sidebar.noObjects') }}
              </div>
            </TreeGroup>
          </div>
        </div>
      </template>

      <!-- Non-schema-aware: flat groups (MySQL, SQLite, etc.) -->
      <template v-else>
        <TreeGroup
          :label="t('sidebar.groups.tables')"
          icon="i-carbon-folder"
          icon-color="text-green-600"
          :count="flatTables.length"
          :default-open="true"
        >
          <div
            v-for="table in flatTables"
            :key="table.name"
            class="text-sm px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
            @click="handleTableClick(table, undefined)"
            @dblclick="handleDoubleClick(table, undefined)"
            @contextmenu="handleContextMenu($event, table, undefined)"
          >
            <span class="i-carbon-table text-green-500 shrink-0 h-3.5 w-3.5" />
            <span class="text-left truncate">{{ table.name }}</span>
          </div>
          <div v-if="flatTables.length === 0" class="text-xs text-muted-foreground px-2 py-1">
            {{ t('sidebar.noObjects') }}
          </div>
        </TreeGroup>
      </template>
    </div>

    <!-- Mode B: All Databases (no database selected) — expand/collapse only, do NOT select -->
    <div v-else-if="isActiveConnectionConnected && !props.selectedDatabase" class="flex-1 overflow-auto">
      <div v-for="db in allDatabases" :key="db.name">
        <button
          class="text-sm px-2 py-1 flex gap-1.5 w-full cursor-pointer items-center hover:bg-accent/40"
          @click="toggleDatabaseNode(db.name)"
        >
          <span class="i-carbon-chevron-right shrink-0 h-3 w-3 transition-transform" :class="{ 'rotate-90': expandedDatabases.has(db.name) }" />
          <span class="i-carbon-data-base text-yellow-500 shrink-0 h-3.5 w-3.5" />
          <span class="text-left flex-1 truncate">{{ db.name }}</span>
          <span v-if="loadingDatabases.has(db.name)" class="i-carbon-loading h-3 w-3 animate-spin" />
        </button>
        <div v-if="expandedDatabases.has(db.name)" class="py-0.5">
          <!-- Schema-aware: show schema nodes with groups under each -->
          <template v-if="hasRealSchemas(db.name)">
            <div v-for="schema in getDbSchemas(db.name)" :key="schema" class="ml-4">
              <div class="text-xs text-muted-foreground font-medium px-2 py-1 flex gap-1.5 items-center">
                <span class="i-carbon-folder-open text-sky-400 shrink-0 h-3.5 w-3.5" />
                <span class="text-left">{{ schema }}</span>
              </div>
              <TreeGroup :label="t('sidebar.groups.tables')" icon="i-carbon-folder" icon-color="text-green-600" :count="getTablesForDbSchema(db.name, schema).length" :default-open="true">
                <div
                  v-for="table in getTablesForDbSchema(db.name, schema)"
                  :key="table.name"
                  class="text-sm ml-4 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                  @click="emit('selectTable', table, db.name, schema)"
                  @dblclick="emit('viewStructure', table, db.name, schema)"
                  @contextmenu="(e) => { e.preventDefault(); contextMenuTable = { table, database: db.name, schema, x: e.clientX, y: e.clientY } }"
                >
                  <span class="i-carbon-table text-green-500 shrink-0 h-3.5 w-3.5" />
                  <span class="text-left truncate">{{ table.name }}</span>
                </div>
              </TreeGroup>
              <TreeGroup :label="t('sidebar.groups.views')" icon="i-carbon-folder" icon-color="text-purple-600" :count="getObjectsForDbSchema(db.name, schema)?.views.length ?? 0" :default-open="false" @toggle="(open: boolean) => handleGroupToggleForDb(open, db.name, schema)">
                <div
                  v-for="view in getObjectsForDbSchema(db.name, schema)?.views ?? []"
                  :key="view.name"
                  class="text-sm ml-4 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                  @click="emit('openDdlTab', view.name, 'VIEW', db.name, schema)"
                >
                  <span class="shrink-0 h-3.5 w-3.5" :class="view.object_type?.toUpperCase().includes('MATERIALIZED') ? 'i-carbon-view text-indigo-500' : 'i-carbon-view text-purple-500'" />
                  <span class="text-left truncate">{{ view.name }}</span>
                </div>
              </TreeGroup>
              <TreeGroup :label="t('sidebar.groups.procedures')" icon="i-carbon-folder" icon-color="text-blue-600" :count="getObjectsForDbSchema(db.name, schema)?.procedures.length ?? 0" :default-open="false" @toggle="(open: boolean) => handleGroupToggleForDb(open, db.name, schema)">
                <div
                  v-for="proc in getObjectsForDbSchema(db.name, schema)?.procedures ?? []"
                  :key="proc.name"
                  class="text-sm ml-4 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                  @click="emit('openDdlTab', proc.name, 'PROCEDURE', db.name, schema)"
                >
                  <span class="i-carbon-document text-blue-500 shrink-0 h-3.5 w-3.5" />
                  <span class="text-left truncate">{{ proc.name }}</span>
                </div>
              </TreeGroup>
              <TreeGroup :label="t('sidebar.groups.functions')" icon="i-carbon-folder" icon-color="text-amber-600" :count="getObjectsForDbSchema(db.name, schema)?.functions.length ?? 0" :default-open="false" @toggle="(open: boolean) => handleGroupToggleForDb(open, db.name, schema)">
                <div
                  v-for="fn in getObjectsForDbSchema(db.name, schema)?.functions ?? []"
                  :key="fn.name"
                  class="text-sm ml-4 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                  @click="emit('openDdlTab', fn.name, 'FUNCTION', db.name, schema)"
                >
                  <span class="i-carbon-function-math text-amber-500 shrink-0 h-3.5 w-3.5" />
                  <span class="text-left truncate">{{ fn.name }}</span>
                </div>
              </TreeGroup>
            </div>
          </template>
          <!-- Non-schema-aware: flat groups directly under database -->
          <template v-else>
            <TreeGroup :label="t('sidebar.groups.tables')" icon="i-carbon-folder" icon-color="text-green-600" :count="getTablesForDbSchema(db.name).length" :default-open="true">
              <div
                v-for="table in getTablesForDbSchema(db.name)"
                :key="table.name"
                class="text-sm ml-4 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                @click="emit('selectTable', table, db.name)"
                @dblclick="emit('viewStructure', table, db.name)"
                @contextmenu="(e) => { e.preventDefault(); contextMenuTable = { table, database: db.name, x: e.clientX, y: e.clientY } }"
              >
                <span class="i-carbon-table text-green-500 shrink-0 h-3.5 w-3.5" />
                <span class="text-left truncate">{{ table.name }}</span>
              </div>
            </TreeGroup>
            <TreeGroup :label="t('sidebar.groups.views')" icon="i-carbon-folder" icon-color="text-purple-600" :count="getObjectsForDbSchema(db.name, db.name)?.views.length ?? 0" :default-open="false" @toggle="(open: boolean) => handleGroupToggleForDb(open, db.name, db.name)">
              <div
                v-for="view in getObjectsForDbSchema(db.name, db.name)?.views ?? []"
                :key="view.name"
                class="text-sm ml-4 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                @click="emit('openDdlTab', view.name, 'VIEW', db.name)"
              >
                <span class="shrink-0 h-3.5 w-3.5" :class="view.object_type?.toUpperCase().includes('MATERIALIZED') ? 'i-carbon-view text-indigo-500' : 'i-carbon-view text-purple-500'" />
                <span class="text-left truncate">{{ view.name }}</span>
              </div>
            </TreeGroup>
            <TreeGroup :label="t('sidebar.groups.procedures')" icon="i-carbon-folder" icon-color="text-blue-600" :count="getObjectsForDbSchema(db.name, db.name)?.procedures.length ?? 0" :default-open="false" @toggle="(open: boolean) => handleGroupToggleForDb(open, db.name, db.name)">
              <div
                v-for="proc in getObjectsForDbSchema(db.name, db.name)?.procedures ?? []"
                :key="proc.name"
                class="text-sm ml-4 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                @click="emit('openDdlTab', proc.name, 'PROCEDURE', db.name)"
              >
                <span class="i-carbon-document text-blue-500 shrink-0 h-3.5 w-3.5" />
                <span class="text-left truncate">{{ proc.name }}</span>
              </div>
            </TreeGroup>
            <TreeGroup :label="t('sidebar.groups.functions')" icon="i-carbon-folder" icon-color="text-amber-600" :count="getObjectsForDbSchema(db.name, db.name)?.functions.length ?? 0" :default-open="false" @toggle="(open: boolean) => handleGroupToggleForDb(open, db.name, db.name)">
              <div
                v-for="fn in getObjectsForDbSchema(db.name, db.name)?.functions ?? []"
                :key="fn.name"
                class="text-sm ml-4 px-2 py-0.5 flex gap-1.5 cursor-pointer items-center hover:bg-accent/40"
                @click="emit('openDdlTab', fn.name, 'FUNCTION', db.name)"
              >
                <span class="i-carbon-function-math text-amber-500 shrink-0 h-3.5 w-3.5" />
                <span class="text-left truncate">{{ fn.name }}</span>
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
