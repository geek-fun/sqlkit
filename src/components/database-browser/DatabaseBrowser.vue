<script setup lang="ts">
import type { SavedQueryInfo } from '@/datasources/fileApi'
import type { DatabaseSchema, TableInfo } from '@/store/databaseStore'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { deleteQueryFile, listSavedQueryFiles } from '@/datasources'
import { ConnectionStatus, DatabaseType, useConnectionStore, useDatabaseStore } from '@/store'

export type TreeNodeMetadata = TableInfo & {
  database: string
  schema?: string
}

export type TreeNodeType = 'connection' | 'database' | 'schema' | 'table' | 'view' | 'column'

export type TreeNode = {
  id: string
  name: string
  type: TreeNodeType
  children?: TreeNode[]
  isExpanded?: boolean
  isLoading?: boolean
  parentId?: string
  metadata?: TreeNodeMetadata
}

const props = defineProps<{
  connectionId?: string
  selectedDatabase?: string
  selectedSchema?: string
}>()

const emit = defineEmits<{
  (e: 'selectTable', table: TableInfo, database: string, schema?: string): void
  (e: 'createScript', table: TableInfo, database: string, schema?: string): void
  (e: 'selectTopN', table: TableInfo, database: string, schema?: string, n?: number): void
  (e: 'viewStructure', table: TableInfo, database: string, schema?: string): void
  (e: 'exportData', table: TableInfo, database: string, schema?: string): void
  (e: 'dropTable', table: TableInfo, database: string, schema?: string): void
  (e: 'truncateTable', table: TableInfo, database: string, schema?: string): void
  (e: 'update:selectedDatabase', database: string): void
  (e: 'update:selectedSchema', schema: string): void
  (e: 'openSavedQuery', filePath: string): void
  (e: 'createNewQuery'): void
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()
const databaseStore = useDatabaseStore()

const searchQuery = ref('')
const expandedNodes = ref<Set<string>>(new Set())
const selectedNodeId = ref<string | null>(null)
const contextMenuNode = ref<TreeNode | null>(null)
const contextMenuPosition = ref({ x: 0, y: 0 })
const showContextMenu = ref(false)
const showTables = ref(true)
const showViews = ref(false)
const showSavedQueries = ref(false)
const savedQueryFiles = ref<SavedQueryInfo[]>([])
const savedQueriesLoading = ref(false)
const savedQueryContextMenu = ref<SavedQueryInfo | null>(null)
const savedQueryContextMenuPosition = ref({ x: 0, y: 0 })
const showSavedQueryContextMenu = ref(false)

const activeConnection = computed(() => {
  const connId = props.connectionId || connectionStore.activeConnectionId
  return connId ? connectionStore.getConnectionById(connId) : connectionStore.activeConnection
})
const connectionId = computed(() => props.connectionId || connectionStore.activeConnectionId)
const isActiveConnectionConnected = computed(() =>
  connectionId.value
    ? connectionStore.getConnectionStatus(connectionId.value) === ConnectionStatus.CONNECTED
    : false,
)

const supportsSchemas = computed(() => {
  const type = activeConnection.value?.type
  return type === DatabaseType.POSTGRESQL || type === DatabaseType.SQLSERVER
})

const availableSchemas = computed<string[]>(() => {
  if (!props.selectedDatabase || !connectionId.value || !supportsSchemas.value) {
    return []
  }
  return databaseStore.metadata[connectionId.value]?.schemas[props.selectedDatabase] ?? []
})

function createTableNode(database: string, schema: string | undefined, table: TableInfo, parentId: string): TreeNode {
  return {
    id: `table-${database}-${schema || ''}-${table.name}`,
    name: table.name,
    type: table.table_type?.toLowerCase() === 'view' ? 'view' : 'table',
    parentId,
    metadata: { database, schema, ...table },
  }
}

function createSchemaNode(database: string, schema: string, tables: TableInfo[], parentId: string): TreeNode {
  const schemaId = `schema-${database}-${schema}`
  return {
    id: schemaId,
    name: schema,
    type: 'schema',
    parentId,
    isExpanded: expandedNodes.value.has(schemaId),
    children: tables.map(table => createTableNode(database, schema, table, schemaId)),
  }
}

function createDatabaseNode(database: DatabaseSchema, metadata: { schemas: Record<string, string[]>, tables: Record<string, TableInfo[]> }): TreeNode {
  const dbId = `db-${database.name}`
  const schemas = metadata.schemas[database.name] || []

  const children = schemas.length > 0
    ? schemas.map(schema => createSchemaNode(
        database.name,
        schema,
        metadata.tables[`${database.name}.${schema}`] || [],
        dbId,
      ))
    : (metadata.tables[database.name] || []).map(table => createTableNode(database.name, undefined, table, dbId))

  return {
    id: dbId,
    name: database.name,
    type: 'database',
    isExpanded: expandedNodes.value.has(dbId),
    children,
  }
}

const userDatabaseNodes = computed<TreeNode[]>(() => {
  if (!connectionId.value || !databaseStore.metadata[connectionId.value]) {
    return []
  }

  const metadata = databaseStore.metadata[connectionId.value]
  return metadata.databases
    .filter(db => !db.is_system)
    .map(database => createDatabaseNode(database, metadata))
})

const systemDatabaseNodes = computed<TreeNode[]>(() => {
  if (!connectionId.value || !databaseStore.metadata[connectionId.value]) {
    return []
  }

  const metadata = databaseStore.metadata[connectionId.value]
  return metadata.databases
    .filter(db => db.is_system)
    .map(database => createDatabaseNode(database, metadata))
})

const tablesAndViews = computed(() => {
  const currentDb = props.selectedDatabase
  if (!currentDb || !connectionId.value || !databaseStore.metadata[connectionId.value]) {
    return { tables: [], views: [] }
  }

  const metadata = databaseStore.metadata[connectionId.value]
  const schemas = metadata.schemas[currentDb] || []

  const allItems: TreeNode[] = schemas.length > 0
    ? (() => {
        const schema = props.selectedSchema || schemas[0]
        const tablesKey = `${currentDb}.${schema}`
        return (metadata.tables[tablesKey] || []).map(table =>
          createTableNode(currentDb, schema, table, `schema-${currentDb}-${schema}`),
        )
      })()
    : (metadata.tables[currentDb] || []).map(table =>
        createTableNode(currentDb, undefined, table, `db-${currentDb}`),
      )

  const query = searchQuery.value.toLowerCase().trim()
  const filtered = query
    ? allItems.filter(item => item.name.toLowerCase().includes(query))
    : allItems

  return {
    tables: filtered.filter(item => item.type === 'table'),
    views: filtered.filter(item => item.type === 'view'),
  }
})

const filteredSavedQueries = computed(() => {
  const query = searchQuery.value.toLowerCase().trim()
  return query
    ? savedQueryFiles.value.filter(file =>
        file.file_name.toLowerCase().includes(query)
        || file.folder.toLowerCase().includes(query),
      )
    : savedQueryFiles.value
})

async function fetchSavedQueryFiles() {
  savedQueriesLoading.value = true
  try {
    savedQueryFiles.value = await listSavedQueryFiles()
  }
  catch (error) {
    console.error('Failed to fetch saved queries:', error)
    savedQueryFiles.value = []
  }
  finally {
    savedQueriesLoading.value = false
  }
}

function handleSavedQueryClick(file: SavedQueryInfo) {
  emit('openSavedQuery', file.file_path)
}

function handleSavedQueryContextMenu(event: MouseEvent, file: SavedQueryInfo) {
  event.preventDefault()
  savedQueryContextMenu.value = file
  savedQueryContextMenuPosition.value = { x: event.clientX, y: event.clientY }
  showSavedQueryContextMenu.value = true
}

function handleSavedQueryOpen() {
  if (savedQueryContextMenu.value) {
    emit('openSavedQuery', savedQueryContextMenu.value.file_path)
  }
  closeSavedQueryContextMenu()
}

async function handleSavedQueryDelete() {
  if (!savedQueryContextMenu.value)
    return

  try {
    await deleteQueryFile(savedQueryContextMenu.value.file_path)
    savedQueryFiles.value = savedQueryFiles.value.filter(
      f => f.file_path !== savedQueryContextMenu.value!.file_path,
    )
  }
  catch (error) {
    console.error('Failed to delete saved query:', error)
  }
  closeSavedQueryContextMenu()
}

async function handleSavedQueryReveal() {
  if (!savedQueryContextMenu.value)
    return

  try {
    await revealItemInDir(savedQueryContextMenu.value.file_path)
  }
  catch (error) {
    console.error('Failed to reveal file:', error)
  }
  closeSavedQueryContextMenu()
}

function closeSavedQueryContextMenu() {
  showSavedQueryContextMenu.value = false
  savedQueryContextMenu.value = null
}

async function toggleNode(node: TreeNode) {
  const nodeId = node.id

  if (expandedNodes.value.has(nodeId)) {
    expandedNodes.value = new Set([...expandedNodes.value].filter(id => id !== nodeId))
  }
  else {
    expandedNodes.value = new Set([...expandedNodes.value, nodeId])

    if (node.type === 'database' && connectionId.value) {
      const database = node.name
      const metadata = databaseStore.metadata[connectionId.value]

      const connectedDb = activeConnection.value?.database
      if (!connectedDb || connectedDb === database) {
        if (!metadata?.schemas[database]) {
          await databaseStore.fetchSchemas(connectionId.value, database)
        }

        if (!metadata?.schemas[database] || metadata.schemas[database].length === 0) {
          if (!metadata?.tables[database]) {
            await databaseStore.fetchTables(connectionId.value, database)
          }
        }
      }
    }
    else if (node.type === 'schema' && connectionId.value) {
      const parts = node.id.split('-')
      const database = parts[1]
      const schema = parts.slice(2).join('-')
      const tablesKey = `${database}.${schema}`
      const metadata = databaseStore.metadata[connectionId.value]

      const connectedDb = activeConnection.value?.database
      if (!connectedDb || connectedDb === database) {
        if (!metadata?.tables[tablesKey]) {
          await databaseStore.fetchTables(connectionId.value, database, schema)
        }
      }
    }
  }
}

function selectNode(node: TreeNode) {
  selectedNodeId.value = node.id
}

function handleTableClick(node: TreeNode) {
  selectNode(node)
  if ((node.type === 'table' || node.type === 'view') && node.metadata) {
    emit('selectTable', node.metadata, node.metadata.database, node.metadata.schema)
  }
}

function handleDoubleClick(node: TreeNode) {
  if (node.type === 'table' || node.type === 'view') {
    const metadata = node.metadata
    if (metadata) {
      emit('viewStructure', metadata, metadata.database, metadata.schema)
    }
  }
  else {
    toggleNode(node)
  }
}

function handleContextMenu(event: MouseEvent, node: TreeNode) {
  if (node.type !== 'table' && node.type !== 'view') {
    return
  }

  event.preventDefault()
  contextMenuNode.value = node
  contextMenuPosition.value = { x: event.clientX, y: event.clientY }
  showContextMenu.value = true
}

type ContextAction = 'createScript' | 'selectTopN' | 'viewStructure' | 'exportData' | 'dropTable' | 'truncateTable'

const contextActionEmitters: Record<ContextAction, (metadata: TreeNodeMetadata) => void> = {
  createScript: metadata => emit('createScript', metadata, metadata.database, metadata.schema),
  selectTopN: metadata => emit('selectTopN', metadata, metadata.database, metadata.schema, 100),
  viewStructure: metadata => emit('viewStructure', metadata, metadata.database, metadata.schema),
  exportData: metadata => emit('exportData', metadata, metadata.database, metadata.schema),
  dropTable: metadata => emit('dropTable', metadata, metadata.database, metadata.schema),
  truncateTable: metadata => emit('truncateTable', metadata, metadata.database, metadata.schema),
}

function handleContextAction(action: ContextAction) {
  if (!contextMenuNode.value || !contextMenuNode.value.metadata) {
    return
  }

  contextActionEmitters[action](contextMenuNode.value.metadata)
  showContextMenu.value = false
  contextMenuNode.value = null
}

async function fetchTablesForSchemas(connId: string, database: string, schemas: string[]) {
  if (schemas.length > 0) {
    await Promise.all(schemas.map(schema => databaseStore.fetchTables(connId, database, schema)))
  }
  else {
    await databaseStore.fetchTables(connId, database)
  }
}

async function refreshTree() {
  if (!connectionId.value) {
    return
  }

  const connection = activeConnection.value
  if (!connection?.isConnected) {
    console.warn('Connection not established, cannot refresh')
    return
  }

  databaseStore.clearMetadata(connectionId.value)
  await databaseStore.fetchDatabases(connectionId.value)

  const connectedDb = props.selectedDatabase
    || connectionStore.getCurrentDatabase(connectionId.value)
    || connection.database
  if (connectedDb) {
    await databaseStore.fetchSchemas(connectionId.value, connectedDb)

    const metadata = databaseStore.metadata[connectionId.value]
    const schemas = metadata?.schemas[connectedDb] || []

    await fetchTablesForSchemas(connectionId.value, connectedDb, schemas)
  }

  // Also refresh saved queries
  await fetchSavedQueryFiles()
}

async function loadDatabaseData(connId: string, database: string) {
  await databaseStore.fetchSchemas(connId, database)

  const metadata = databaseStore.metadata[connId]
  const schemas = metadata?.schemas[database] || []

  await fetchTablesForSchemas(connId, database, schemas)
}

watch(connectionId, async (newId) => {
  if (!newId) {
    return
  }

  const connection = activeConnection.value
  if (!connection?.isConnected) {
    console.warn('Connection not established yet, skipping data fetch')
    return
  }

  await databaseStore.fetchDatabases(newId)

  const connectedDb = connection.database || connectionStore.getCurrentDatabase(newId)
  if (connectedDb) {
    await loadDatabaseData(newId, connectedDb)
  }
  // Auto-select the initial database in the parent if none is selected yet.
  if (!props.selectedDatabase && connectedDb) {
    emit('update:selectedDatabase', connectedDb)
  }
}, { immediate: true })

watch(() => activeConnection.value?.isConnected, async (isConnected) => {
  if (!isConnected || !connectionId.value) {
    return
  }

  await databaseStore.fetchDatabases(connectionId.value)

  const connectedDb = activeConnection.value?.database || connectionStore.getCurrentDatabase(connectionId.value)
  if (connectedDb) {
    await loadDatabaseData(connectionId.value, connectedDb)
  }
  // Auto-select the initial database in the parent if none is selected yet.
  if (!props.selectedDatabase && connectedDb) {
    emit('update:selectedDatabase', connectedDb)
  }
})

watch(() => props.selectedDatabase, async (newDb, oldDb) => {
  if (!newDb || !connectionId.value || newDb === oldDb) {
    return
  }

  const meta = databaseStore.metadata[connectionId.value]
  if (meta) {
    delete meta.schemas[newDb]
    const staleKeys = Object.keys(meta.tables).filter(k => k === newDb || k.startsWith(`${newDb}.`))
    staleKeys.forEach(k => delete meta.tables[k])
  }

  await loadDatabaseData(connectionId.value, newDb)
})

watch(() => props.selectedSchema, async (newSchema, oldSchema) => {
  if (!newSchema || !props.selectedDatabase || !connectionId.value || newSchema === oldSchema) {
    return
  }

  const tablesKey = `${props.selectedDatabase}.${newSchema}`
  const meta = databaseStore.metadata[connectionId.value]
  if (!meta?.tables[tablesKey]) {
    await databaseStore.fetchTables(connectionId.value, props.selectedDatabase, newSchema)
  }
})

watch(showSavedQueries, async (isExpanded) => {
  if (isExpanded && savedQueryFiles.value.length === 0) {
    await fetchSavedQueryFiles()
  }
})

function closeContextMenu() {
  showContextMenu.value = false
  contextMenuNode.value = null
  closeSavedQueryContextMenu()
}

type IconType = 'table' | 'view' | 'database' | 'column' | 'schema' | 'file'

const iconMap: Record<IconType, string> = {
  database: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3"></ellipse><path d="M3 5V19A9 3 0 0 0 21 19V5"></path><path d="M3 12A9 3 0 0 0 21 12"></path></svg>`,
  table: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v18"/><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 9h18"/><path d="M3 15h18"/></svg>`,
  view: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></svg>`,
  schema: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg>`,
  column: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>`,
  file: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>`,
}

const getIcon = (type: IconType) => iconMap[type] || iconMap.column

defineExpose({ fetchSavedQueryFiles, refreshTree })
</script>

<template>
  <div class="database-browser flex flex-col h-full" @click="closeContextMenu">
    <!-- Header -->
    <div class="p-2 border-b">
      <div class="mb-2 flex gap-2 items-center">
        <h3 class="text-sm font-semibold flex-1">
          {{ t('components.databaseBrowser.title') }}
        </h3>
        <Button
          variant="ghost"
          size="icon"
          class="h-6 w-6"
          :title="t('components.databaseBrowser.refresh')"
          @click="refreshTree"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
            <path d="M21 3v5h-5" />
            <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" />
            <path d="M8 16H3v5" />
          </svg>
        </Button>
      </div>

      <!-- Search -->
      <div class="relative">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="text-muted-foreground left-2 top-1/2 absolute -translate-y-1/2"
        >
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.3-4.3" />
        </svg>
        <Input
          v-model="searchQuery"
          :placeholder="t('components.databaseBrowser.search')"
          class="text-xs pl-7 h-7"
        />
      </div>
    </div>

    <!-- Database selector: always visible so the user can switch databases -->
    <div v-if="isActiveConnectionConnected" class="px-2 py-1 border-b">
      <Select :model-value="props.selectedDatabase" @update:model-value="(val) => emit('update:selectedDatabase', val)">
        <SelectTrigger class="text-xs h-7">
          <SelectValue :placeholder="t('components.databaseBrowser.selectDatabase')" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup v-if="userDatabaseNodes.length > 0">
            <SelectLabel class="text-xs text-muted-foreground">
              {{ t('components.databaseBrowser.userDatabases') }}
            </SelectLabel>
            <SelectItem
              v-for="db in userDatabaseNodes"
              :key="db.id"
              :value="db.name"
            >
              {{ db.name }}
            </SelectItem>
          </SelectGroup>
          <SelectGroup v-if="systemDatabaseNodes.length > 0">
            <SelectLabel class="text-xs text-muted-foreground">
              {{ t('components.databaseBrowser.systemDatabases') }}
            </SelectLabel>
            <SelectItem
              v-for="db in systemDatabaseNodes"
              :key="db.id"
              :value="db.name"
            >
              {{ db.name }}
            </SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    </div>

    <!-- Schema selector: only for databases with schema concept (PG, MSSQL) -->
    <div v-if="isActiveConnectionConnected && supportsSchemas && availableSchemas.length > 0" class="px-2 py-1 border-b">
      <Select :model-value="props.selectedSchema || availableSchemas[0]" @update:model-value="(val) => emit('update:selectedSchema', val)">
        <SelectTrigger class="text-xs h-7">
          <SelectValue :placeholder="t('components.databaseBrowser.selectSchema')" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem
            v-for="schema in availableSchemas"
            :key="schema"
            :value="schema"
          >
            {{ schema }}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>

    <!-- Tree view -->
    <div class="flex-1 overflow-auto">
      <!-- TABLES Section -->
      <div class="border-b">
        <button
          class="text-xs text-muted-foreground font-semibold px-2 py-1.5 flex gap-2 w-full uppercase items-center hover:bg-accent/50"
          @click="showTables = !showTables"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="transition-transform"
            :class="{ 'rotate-90': showTables }"
          >
            <path d="m9 18 6-6-6-6" />
          </svg>
          {{ t('components.databaseBrowser.sections.tables') }}
        </button>
        <div v-if="showTables" class="py-1">
          <div
            v-for="table in tablesAndViews.tables"
            :key="table.id"
            class="tree-node text-sm px-2 py-1 flex gap-2 cursor-pointer items-center hover:bg-accent"
            :class="{ 'bg-accent': selectedNodeId === table.id }"
            @click="handleTableClick(table)"
            @dblclick="handleDoubleClick(table)"
            @contextmenu="handleContextMenu($event, table)"
          >
            <span class="opacity-70 flex-shrink-0" v-html="getIcon('table')" />
            <span class="truncate">{{ table.name }}</span>
          </div>
          <div v-if="tablesAndViews.tables.length === 0 && !databaseStore.loading" class="text-xs text-muted-foreground px-2 py-2">
            {{ t('components.databaseBrowser.noTables') }}
          </div>
        </div>
      </div>

      <!-- VIEWS Section -->
      <div class="border-b">
        <button
          class="text-xs text-muted-foreground font-semibold px-2 py-1.5 flex gap-2 w-full uppercase items-center hover:bg-accent/50"
          @click="showViews = !showViews"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="transition-transform"
            :class="{ 'rotate-90': showViews }"
          >
            <path d="m9 18 6-6-6-6" />
          </svg>
          {{ t('components.databaseBrowser.sections.views') }}
        </button>
        <div v-if="showViews" class="py-1">
          <div
            v-for="view in tablesAndViews.views"
            :key="view.id"
            class="tree-node text-sm px-2 py-1 flex gap-2 cursor-pointer items-center hover:bg-accent"
            :class="{ 'bg-accent': selectedNodeId === view.id }"
            @click="handleTableClick(view)"
            @dblclick="handleDoubleClick(view)"
            @contextmenu="handleContextMenu($event, view)"
          >
            <span class="opacity-70 flex-shrink-0" v-html="getIcon('view')" />
            <span class="truncate">{{ view.name }}</span>
          </div>
          <div v-if="tablesAndViews.views.length === 0 && !databaseStore.loading" class="text-xs text-muted-foreground px-2 py-2">
            {{ t('components.databaseBrowser.noViews') }}
          </div>
        </div>
      </div>

      <!-- SAVED QUERIES Section -->
      <div class="border-b">
        <button
          class="text-xs text-muted-foreground font-semibold px-2 py-1.5 flex gap-2 w-full uppercase items-center hover:bg-accent/50"
          @click="showSavedQueries = !showSavedQueries"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="transition-transform"
            :class="{ 'rotate-90': showSavedQueries }"
          >
            <path d="m9 18 6-6-6-6" />
          </svg>
          {{ t('components.databaseBrowser.sections.savedQueries') }}
          <Button
            variant="ghost"
            size="icon"
            class="ml-auto h-4 w-4"
            :title="t('components.databaseBrowser.newQuery')"
            @click.stop="emit('createNewQuery')"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M5 12h14" />
              <path d="M12 5v14" />
            </svg>
          </Button>
        </button>
        <div v-if="showSavedQueries" class="py-1">
          <!-- Loading state -->
          <div v-if="savedQueriesLoading" class="text-xs text-muted-foreground px-2 py-2 flex gap-2 items-center">
            <svg class="h-3 w-3 animate-spin" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
            </svg>
            {{ t('components.databaseBrowser.loading') }}
          </div>
          <!-- File list -->
          <div
            v-for="file in filteredSavedQueries"
            :key="file.file_path"
            class="tree-node text-sm px-2 py-1 flex flex-col cursor-pointer hover:bg-accent"
            :class="{ 'bg-accent': savedQueryContextMenu?.file_path === file.file_path }"
            @click="handleSavedQueryClick(file)"
            @contextmenu="handleSavedQueryContextMenu($event, file)"
          >
            <div class="flex gap-2 items-center">
              <span class="opacity-70 flex-shrink-0" v-html="getIcon('file')" />
              <span class="truncate">{{ file.file_name }}</span>
            </div>
            <span class="text-xs text-muted-foreground pl-[calc(16px+8px)] truncate">{{ file.folder }}</span>
          </div>
          <!-- Empty state -->
          <div v-if="filteredSavedQueries.length === 0 && !savedQueriesLoading" class="text-xs text-muted-foreground px-2 py-2">
            {{ t('components.databaseBrowser.noSavedQueries') }}
          </div>
        </div>
      </div>

      <!-- Loading state -->
      <div v-if="databaseStore.loading" class="text-sm text-muted-foreground py-4 text-center">
        <svg
          class="mx-auto mb-2 h-5 w-5 animate-spin"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
        >
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
        </svg>
        <p>{{ t('components.databaseBrowser.loading') }}</p>
      </div>
    </div>

    <!-- Context Menu -->
    <div
      v-if="showContextMenu"
      class="text-popover-foreground border rounded-md bg-popover w-48 shadow-md fixed z-50"
      :style="{ left: `${contextMenuPosition.x}px`, top: `${contextMenuPosition.y}px` }"
    >
      <div class="p-1">
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleContextAction('selectTopN')"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <path d="m3 16 4 4 4-4" />
            <path d="M7 20V4" />
            <path d="M11 4h4" />
            <path d="M11 8h7" />
            <path d="M11 12h10" />
          </svg>
          {{ t('components.databaseBrowser.contextMenu.selectTopN') }}
        </div>
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleContextAction('viewStructure')"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <rect width="18" height="18" x="3" y="3" rx="2" />
            <path d="M3 9h18" />
            <path d="M9 21V9" />
          </svg>
          {{ t('components.databaseBrowser.contextMenu.viewStructure') }}
        </div>
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleContextAction('createScript')"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
            <path d="M14 2v4a2 2 0 0 0 2 2h4" />
            <path d="M10 12a1 1 0 0 0-1 1v1a1 1 0 0 1-1 1 1 1 0 0 1 1 1v1a1 1 0 0 0 1 1" />
            <path d="M14 18a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1 1 1 0 0 1-1-1v-1a1 1 0 0 0-1-1" />
          </svg>
          {{ t('components.databaseBrowser.contextMenu.createScript') }}
        </div>
        <div class="my-1 bg-border h-px" />
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleContextAction('exportData')"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" x2="12" y1="15" y2="3" />
          </svg>
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

    <!-- Saved Query Context Menu -->
    <div
      v-if="showSavedQueryContextMenu"
      class="text-popover-foreground border rounded-md bg-popover w-48 shadow-md fixed z-50"
      :style="{ left: `${savedQueryContextMenuPosition.x}px`, top: `${savedQueryContextMenuPosition.y}px` }"
    >
      <div class="p-1">
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleSavedQueryOpen"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <path d="M5 12h14" />
            <path d="M12 5v14" />
          </svg>
          {{ t('components.databaseBrowser.savedQueryActions.open') }}
        </div>
        <div class="my-1 bg-border h-px" />
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleSavedQueryReveal"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
            <path d="M21 3v5h-5" />
          </svg>
          {{ t('components.databaseBrowser.savedQueryActions.reveal') }}
        </div>
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="handleSavedQueryDelete"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <path d="M3 6h18" />
            <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
            <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
          </svg>
          {{ t('components.databaseBrowser.savedQueryActions.delete') }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tree-node {
  user-select: none;
}
</style>
