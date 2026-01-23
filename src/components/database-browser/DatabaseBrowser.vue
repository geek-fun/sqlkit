<script setup lang="ts">
import type { TableInfo } from '@/store/databaseStore'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
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

const emit = defineEmits<{
  (e: 'selectTable', table: TableInfo, database: string, schema?: string): void
  (e: 'createScript', table: TableInfo, database: string, schema?: string): void
  (e: 'selectTopN', table: TableInfo, database: string, schema?: string, n?: number): void
  (e: 'viewStructure', table: TableInfo, database: string, schema?: string): void
  (e: 'exportData', table: TableInfo, database: string, schema?: string): void
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

const activeConnection = computed(() => connectionStore.activeConnection)
const connectionId = computed(() => connectionStore.activeConnectionId)

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
          type: 'table' as const,
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
        type: 'table' as const,
        parentId: dbNode.id,
        metadata: { database, ...table },
      }))
    }

    nodes.push(dbNode)
  }

  return nodes
})

// Filter nodes based on search query
const filteredNodes = computed(() => {
  if (!searchQuery.value.trim()) {
    return treeNodes.value
  }

  const query = searchQuery.value.toLowerCase()

  const filterNode = (node: TreeNode): TreeNode | null => {
    if (node.name.toLowerCase().includes(query)) {
      return { ...node, isExpanded: true }
    }

    if (node.children) {
      const filteredChildren = node.children
        .map(filterNode)
        .filter((n): n is TreeNode => n !== null)

      if (filteredChildren.length > 0) {
        return { ...node, children: filteredChildren, isExpanded: true }
      }
    }

    return null
  }

  return treeNodes.value
    .map(filterNode)
    .filter((n): n is TreeNode => n !== null)
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
    else if (node.type === 'schema' && connectionId.value) {
      const parts = node.id.split('-')
      const database = parts[1]
      const schema = parts.slice(2).join('-')
      const tablesKey = `${database}.${schema}`
      const metadata = databaseStore.metadata[connectionId.value]

      if (!metadata?.tables[tablesKey]) {
        await databaseStore.fetchTables(connectionId.value, database, schema)
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
    databaseStore.clearMetadata(connectionId.value)
    await databaseStore.fetchDatabases(connectionId.value)
  }
}

// Load databases when connection changes
watch(connectionId, async (newId) => {
  if (newId) {
    await databaseStore.fetchDatabases(newId)
  }
}, { immediate: true })

// Close context menu on click outside
function closeContextMenu() {
  showContextMenu.value = false
  contextMenuNode.value = null
}

function getNodeIcon(type: TreeNode['type']) {
  switch (type) {
    case 'database':
      return '🗄️'
    case 'schema':
      return '📁'
    case 'table':
      return '📋'
    case 'view':
      return '👁️'
    case 'column':
      return '📊'
    default:
      return '📄'
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

    <!-- Connection info -->
    <div v-if="activeConnection" class="text-xs px-2 py-1 border-b bg-muted/50">
      <span class="font-medium">{{ activeConnection.name }}</span>
      <span class="text-muted-foreground ml-1">({{ activeConnection.host }})</span>
    </div>

    <!-- Tree view -->
    <div class="p-1 flex-1 overflow-auto">
      <template v-if="filteredNodes.length > 0">
        <div
          v-for="node in filteredNodes"
          :key="node.id"
        >
          <!-- Database node -->
          <div
            class="tree-node text-sm px-1 py-0.5 rounded flex gap-1 cursor-pointer items-center hover:bg-accent"
            :class="{ 'bg-accent': selectedNodeId === node.id }"
            @click="selectNode(node)"
            @dblclick="handleDoubleClick(node)"
          >
            <button
              class="flex h-4 w-4 items-center justify-center"
              @click.stop="toggleNode(node)"
            >
              <svg
                v-if="node.children && node.children.length > 0"
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
                :class="{ 'rotate-90': expandedNodes.has(node.id) }"
              >
                <path d="m9 18 6-6-6-6" />
              </svg>
            </button>
            <span>{{ getNodeIcon(node.type) }}</span>
            <span class="truncate">{{ node.name }}</span>
          </div>

          <!-- Children (schemas or tables) -->
          <div v-if="expandedNodes.has(node.id) && node.children" class="pl-4">
            <template v-for="child in node.children" :key="child.id">
              <!-- Schema node -->
              <div
                v-if="child.type === 'schema'"
                class="tree-node text-sm px-1 py-0.5 rounded flex gap-1 cursor-pointer items-center hover:bg-accent"
                :class="{ 'bg-accent': selectedNodeId === child.id }"
                @click="selectNode(child)"
                @dblclick="toggleNode(child)"
              >
                <button
                  class="flex h-4 w-4 items-center justify-center"
                  @click.stop="toggleNode(child)"
                >
                  <svg
                    v-if="child.children && child.children.length > 0"
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
                    :class="{ 'rotate-90': expandedNodes.has(child.id) }"
                  >
                    <path d="m9 18 6-6-6-6" />
                  </svg>
                </button>
                <span>{{ getNodeIcon(child.type) }}</span>
                <span class="truncate">{{ child.name }}</span>
              </div>

              <!-- Tables under schema -->
              <div v-if="child.type === 'schema' && expandedNodes.has(child.id) && child.children" class="pl-4">
                <div
                  v-for="table in child.children"
                  :key="table.id"
                  class="tree-node text-sm px-1 py-0.5 rounded flex gap-1 cursor-pointer items-center hover:bg-accent"
                  :class="{ 'bg-accent': selectedNodeId === table.id }"
                  @click="selectNode(table)"
                  @dblclick="handleDoubleClick(table)"
                  @contextmenu="handleContextMenu($event, table)"
                >
                  <span class="w-4" />
                  <span>{{ getNodeIcon(table.type) }}</span>
                  <span class="truncate">{{ table.name }}</span>
                </div>
              </div>

              <!-- Table node (when no schema) -->
              <div
                v-if="child.type === 'table' || child.type === 'view'"
                class="tree-node text-sm px-1 py-0.5 rounded flex gap-1 cursor-pointer items-center hover:bg-accent"
                :class="{ 'bg-accent': selectedNodeId === child.id }"
                @click="selectNode(child)"
                @dblclick="handleDoubleClick(child)"
                @contextmenu="handleContextMenu($event, child)"
              >
                <span class="w-4" />
                <span>{{ getNodeIcon(child.type) }}</span>
                <span class="truncate">{{ child.name }}</span>
              </div>
            </template>
          </div>
        </div>
      </template>

      <!-- Empty state -->
      <div v-else-if="!databaseStore.loading" class="text-sm text-muted-foreground py-8 text-center">
        <p>{{ t('components.databaseBrowser.empty') }}</p>
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
    <DropdownMenu v-model:open="showContextMenu">
      <DropdownMenuTrigger as-child>
        <div
          v-if="showContextMenu"
          class="fixed z-50"
          :style="{ left: `${contextMenuPosition.x}px`, top: `${contextMenuPosition.y}px` }"
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" class="w-48">
        <DropdownMenuItem @click="handleContextAction('selectTopN')">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <path d="m3 16 4 4 4-4" />
            <path d="M7 20V4" />
            <path d="M11 4h4" />
            <path d="M11 8h7" />
            <path d="M11 12h10" />
          </svg>
          {{ t('components.databaseBrowser.contextMenu.selectTopN') }}
        </DropdownMenuItem>
        <DropdownMenuItem @click="handleContextAction('viewStructure')">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <rect width="18" height="18" x="3" y="3" rx="2" />
            <path d="M3 9h18" />
            <path d="M9 21V9" />
          </svg>
          {{ t('components.databaseBrowser.contextMenu.viewStructure') }}
        </DropdownMenuItem>
        <DropdownMenuItem @click="handleContextAction('createScript')">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
            <path d="M14 2v4a2 2 0 0 0 2 2h4" />
            <path d="M10 12a1 1 0 0 0-1 1v1a1 1 0 0 1-1 1 1 1 0 0 1 1 1v1a1 1 0 0 0 1 1" />
            <path d="M14 18a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1 1 1 0 0 1-1-1v-1a1 1 0 0 0-1-1" />
          </svg>
          {{ t('components.databaseBrowser.contextMenu.createScript') }}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem @click="handleContextAction('exportData')">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" x2="12" y1="15" y2="3" />
          </svg>
          {{ t('components.databaseBrowser.contextMenu.exportData') }}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  </div>
</template>

<style scoped>
.tree-node {
  user-select: none;
}
</style>
