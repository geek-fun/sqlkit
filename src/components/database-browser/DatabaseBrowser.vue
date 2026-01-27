<script setup lang="ts">
import type { TableInfo } from '@/store/databaseStore'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useConnectionStore, useDatabaseStore } from '@/store'

export interface TreeNodeMetadata extends TableInfo {
  database: string
  schema?: string
}

export interface TreeNode {
  id: string
  name: string
  type: 'connection' | 'database' | 'schema' | 'table' | 'view' | 'column'
  children?: TreeNode[]
  isExpanded?: boolean
  isLoading?: boolean
  parentId?: string
  metadata?: TreeNodeMetadata
}

const props = defineProps<{
  connectionId?: string
  selectedDatabase?: string
}>()

const emit = defineEmits<{
  (e: 'selectTable', table: TableInfo, database: string, schema?: string): void
  (e: 'createScript', table: TableInfo, database: string, schema?: string): void
  (e: 'selectTopN', table: TableInfo, database: string, schema?: string, n?: number): void
  (e: 'viewStructure', table: TableInfo, database: string, schema?: string): void
  (e: 'exportData', table: TableInfo, database: string, schema?: string): void
  (e: 'update:selectedDatabase', database: string): void
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

const activeConnection = computed(() => {
  const connId = props.connectionId || connectionStore.activeConnectionId
  return connId ? connectionStore.getConnectionById(connId) : connectionStore.activeConnection
})
const connectionId = computed(() => props.connectionId || connectionStore.activeConnectionId)

// Build tree structure from database metadata
const treeNodes = computed<TreeNode[]>(() => {
  if (!connectionId.value || !databaseStore.metadata[connectionId.value]) {
    return []
  }

  const metadata = databaseStore.metadata[connectionId.value]
  const nodes: TreeNode[] = []

  for (const database of metadata.databases) {
    const dbNode: TreeNode = {
      id: `db-${database}`,
      name: database,
      type: 'database',
      children: [],
      isExpanded: expandedNodes.value.has(`db-${database}`),
    }

    // Add schemas if they exist for this database
    const schemas = metadata.schemas[database] || []
    if (schemas.length > 0) {
      for (const schema of schemas) {
        const schemaNode: TreeNode = {
          id: `schema-${database}-${schema}`,
          name: schema,
          type: 'schema',
          parentId: dbNode.id,
          children: [],
          isExpanded: expandedNodes.value.has(`schema-${database}-${schema}`),
        }

        // Add tables for this schema
        const tablesKey = `${database}.${schema}`
        const tables = metadata.tables[tablesKey] || []
        schemaNode.children = tables.map(table => ({
          id: `table-${database}-${schema}-${table.name}`,
          name: table.name,
          type: table.table_type?.toLowerCase() === 'view' ? ('view' as const) : ('table' as const),
          parentId: schemaNode.id,
          metadata: { database, schema, ...table },
        }))

        dbNode.children!.push(schemaNode)
      }
    }
    else {
      // No schemas - add tables directly under database
      const tables = metadata.tables[database] || []
      dbNode.children = tables.map(table => ({
        id: `table-${database}--${table.name}`,
        name: table.name,
        type: table.table_type?.toLowerCase() === 'view' ? ('view' as const) : ('table' as const),
        parentId: dbNode.id,
        metadata: { database, ...table },
      }))
    }

    nodes.push(dbNode)
  }

  return nodes
})

// Separate tables and views from filtered nodes
const tablesAndViews = computed(() => {
  const currentDb = props.selectedDatabase || activeConnection.value?.database
  if (!currentDb || !connectionId.value || !databaseStore.metadata[connectionId.value]) {
    return { tables: [], views: [] }
  }

  const metadata = databaseStore.metadata[connectionId.value]
  const schemas = metadata.schemas[currentDb] || []
  const allItems: TreeNode[] = []

  if (schemas.length > 0) {
    // Has schemas - collect from all schemas
    for (const schema of schemas) {
      const tablesKey = `${currentDb}.${schema}`
      const tables = metadata.tables[tablesKey] || []
      allItems.push(...tables.map((table): TreeNode => ({
        id: `table-${currentDb}-${schema}-${table.name}`,
        name: table.name,
        type: table.table_type?.toLowerCase() === 'view' ? ('view' as const) : ('table' as const),
        parentId: `schema-${currentDb}-${schema}`,
        metadata: { database: currentDb, schema, ...table },
      })))
    }
  }
  else {
    // No schemas - get tables directly
    const tables = metadata.tables[currentDb] || []
    allItems.push(...tables.map((table): TreeNode => ({
      id: `table-${currentDb}--${table.name}`,
      name: table.name,
      type: table.table_type?.toLowerCase() === 'view' ? ('view' as const) : ('table' as const),
      parentId: `db-${currentDb}`,
      metadata: { database: currentDb, ...table },
    })))
  }

  // Filter by search query
  const query = searchQuery.value.toLowerCase().trim()
  const filtered = query
    ? allItems.filter(item => item.name.toLowerCase().includes(query))
    : allItems

  return {
    tables: filtered.filter(item => item.type === 'table'),
    views: filtered.filter(item => item.type === 'view'),
  }
})

async function toggleNode(node: TreeNode) {
  const nodeId = node.id

  if (expandedNodes.value.has(nodeId)) {
    expandedNodes.value.delete(nodeId)
  }
  else {
    expandedNodes.value.add(nodeId)

    // Lazy load data when expanding
    if (node.type === 'database' && connectionId.value) {
      const database = node.name
      const metadata = databaseStore.metadata[connectionId.value]

      // Only fetch if this is the connected database or no database was specified in connection
      const connectedDb = activeConnection.value?.database
      if (!connectedDb || connectedDb === database) {
        // Load schemas if not already loaded
        if (!metadata?.schemas[database]) {
          await databaseStore.fetchSchemas(connectionId.value, database)
        }

        // Load tables if no schemas exist
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

      // Only fetch if this is the connected database or no database was specified in connection
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

function handleContextAction(action: 'createScript' | 'selectTopN' | 'viewStructure' | 'exportData') {
  if (!contextMenuNode.value || !contextMenuNode.value.metadata) {
    return
  }

  const metadata = contextMenuNode.value.metadata

  switch (action) {
    case 'createScript':
      emit('createScript', metadata, metadata.database, metadata.schema)
      break
    case 'selectTopN':
      emit('selectTopN', metadata, metadata.database, metadata.schema, 100)
      break
    case 'viewStructure':
      emit('viewStructure', metadata, metadata.database, metadata.schema)
      break
    case 'exportData':
      emit('exportData', metadata, metadata.database, metadata.schema)
      break
  }

  showContextMenu.value = false
  contextMenuNode.value = null
}

async function refreshTree() {
  if (connectionId.value) {
    const connection = activeConnection.value
    if (!connection?.isConnected) {
      console.warn('Connection not established, cannot refresh')
      return
    }

    databaseStore.clearMetadata(connectionId.value)
    await databaseStore.fetchDatabases(connectionId.value)

    // Auto-load tables for connected database
    const connectedDb = connection.database
    if (connectedDb) {
      // Fetch schemas first
      await databaseStore.fetchSchemas(connectionId.value, connectedDb)

      // Then fetch tables
      const metadata = databaseStore.metadata[connectionId.value]
      const schemas = metadata?.schemas[connectedDb] || []

      if (schemas.length > 0) {
        // Fetch tables for each schema
        for (const schema of schemas) {
          await databaseStore.fetchTables(connectionId.value, connectedDb, schema)
        }
      }
      else {
        // No schemas - fetch tables directly
        await databaseStore.fetchTables(connectionId.value, connectedDb)
      }
    }
  }
}

// Load databases when connection changes
watch(connectionId, async (newId) => {
  if (newId) {
    // Check if connection is actually connected
    const connection = activeConnection.value
    if (!connection?.isConnected) {
      console.warn('Connection not established yet, skipping data fetch')
      return
    }

    await databaseStore.fetchDatabases(newId)

    // Auto-load tables for connected database
    const connectedDb = connection.database
    if (connectedDb) {
      // Fetch schemas first
      await databaseStore.fetchSchemas(newId, connectedDb)

      // Then fetch tables
      const metadata = databaseStore.metadata[newId]
      const schemas = metadata?.schemas[connectedDb] || []

      if (schemas.length > 0) {
        // Fetch tables for each schema
        for (const schema of schemas) {
          await databaseStore.fetchTables(newId, connectedDb, schema)
        }
      }
      else {
        // No schemas - fetch tables directly
        await databaseStore.fetchTables(newId, connectedDb)
      }
    }
  }
}, { immediate: true })

// Watch for connection status changes
watch(() => activeConnection.value?.isConnected, async (isConnected) => {
  if (isConnected && connectionId.value) {
    // Connection just became connected, fetch data
    await databaseStore.fetchDatabases(connectionId.value)

    // Auto-load tables for connected database
    const connectedDb = activeConnection.value?.database
    if (connectedDb) {
      // Fetch schemas first
      await databaseStore.fetchSchemas(connectionId.value, connectedDb)

      // Then fetch tables
      const metadata = databaseStore.metadata[connectionId.value]
      const schemas = metadata?.schemas[connectedDb] || []

      if (schemas.length > 0) {
        // Fetch tables for each schema
        for (const schema of schemas) {
          await databaseStore.fetchTables(connectionId.value, connectedDb, schema)
        }
      }
      else {
        // No schemas - fetch tables directly
        await databaseStore.fetchTables(connectionId.value, connectedDb)
      }
    }
  }
})

// Watch for database selection changes
watch(() => props.selectedDatabase, async (newDb) => {
  if (newDb && connectionId.value) {
    // Fetch schemas first
    await databaseStore.fetchSchemas(connectionId.value, newDb)

    // Then fetch tables
    const metadata = databaseStore.metadata[connectionId.value]
    const schemas = metadata?.schemas[newDb] || []

    if (schemas.length > 0) {
      // Fetch tables for each schema
      for (const schema of schemas) {
        await databaseStore.fetchTables(connectionId.value, newDb, schema)
      }
    }
    else {
      // No schemas - fetch tables directly
      await databaseStore.fetchTables(connectionId.value, newDb)
    }
  }
})

// Close context menu on click outside
function closeContextMenu() {
  showContextMenu.value = false
  contextMenuNode.value = null
}

function getIcon(type: 'table' | 'view' | 'database' | 'column' | 'schema') {
  switch (type) {
    case 'database':
      return `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3"></ellipse><path d="M3 5V19A9 3 0 0 0 21 19V5"></path><path d="M3 12A9 3 0 0 0 21 12"></path></svg>`
    case 'table':
      return `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v18"/><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 9h18"/><path d="M3 15h18"/></svg>`
    case 'view':
      return `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></svg>`
    case 'schema':
      return `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg>`
    default:
      return `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>`
  }
}
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

    <!-- Database selector (only show if no database specified in connection) -->
    <div v-if="activeConnection && !activeConnection.database" class="px-2 py-1 border-b">
      <Select :model-value="props.selectedDatabase" @update:model-value="(val) => emit('update:selectedDatabase', val)">
        <SelectTrigger class="text-xs h-7">
          <SelectValue :placeholder="t('components.databaseBrowser.selectDatabase')" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem
            v-for="db in treeNodes"
            :key="db.id"
            :value="db.name"
          >
            {{ db.name }}
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
            @click="selectNode(table)"
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
            @click="selectNode(view)"
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
        </button>
        <div v-if="showSavedQueries" class="py-1">
          <div class="text-xs text-muted-foreground px-2 py-2">
            {{ t('components.databaseBrowser.comingSoon') }}
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
      </div>
    </div>
  </div>
</template>

<style scoped>
.tree-node {
  user-select: none;
}
</style>
