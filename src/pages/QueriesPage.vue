<script setup lang="ts">
import type { StatementToExecute } from '@/composables/useSqlStatements'
import type { DatabaseType } from '@/store/connectionStore'
import type { TableInfo } from '@/store/databaseStore'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { DatabaseBrowser, DataTableView, DbTypeIcon, QueryResultPanel, QueryTabs } from '@/components/database-browser'
import ListingTab from '@/components/database-browser/ListingTab.vue'
import ErDiagramView from '@/components/er-diagram/ErDiagramView.vue'
import AppLayout from '@/components/layout/AppLayout.vue'
import SQLEditor from '@/components/SQLEditor.vue'
import { Button } from '@/components/ui/button'
import { DestructiveConfirmDialog } from '@/components/ui/destructive-confirm-dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { toast } from '@/composables/useNotifications'
import { usePlatform } from '@/composables/usePlatform'
import { useSqlFormatter } from '@/composables/useSqlFormatter'
import { loadQueryFile, saveQueryFile, saveQueryFileAs } from '@/datasources'
import { ConnectionStatus, useAppStore, useConnectionStore, useDatabaseStore, useTabStore } from '@/store'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const appStore = useAppStore()
const connectionStore = useConnectionStore()
const databaseStore = useDatabaseStore()
const tabStore = useTabStore()
const { modifierKey } = usePlatform()

const showResultPanel = ref(false)
const sidebarWidth = ref(250)
const isResizingSidebar = ref(false)
const selectedDatabase = ref<string>('')
const selectedSchema = ref<string>('')
const queryTabsRef = ref<InstanceType<typeof QueryTabs>>()
const databaseBrowserRef = ref<InstanceType<typeof DatabaseBrowser>>()

const { formatSql, resolveDialect } = useSqlFormatter()

// ── Destructive action dialog state ──
const destructiveDialogOpen = ref(false)
const destructiveAction = ref<{ type: 'drop' | 'truncate', table: TableInfo, database: string, schema?: string } | null>(null)
const isDestructiveActionExecuting = ref(false)
const showOrphanTabDialog = ref(false)
const orphanTabToHandle = ref<string | null>(null)

// Available connections
const availableConnections = computed(() => connectionStore.connections)

// Get active connection ID (can be changed by user)
const selectedConnectionId = ref<string>('')

const activeConnection = computed(() => {
  const id = selectedConnectionId.value || connectionStore.activeConnectionId
  if (id) {
    return connectionStore.getConnectionById(id)
  }
  return null
})

const activeTab = computed(() => tabStore.activeTab)

const editorContent = computed({
  get: () => activeTab.value?.content || '',
  set: (value: string) => {
    if (activeTab.value) {
      tabStore.updateTabContent(activeTab.value.id, value)
    }
  },
})

onMounted(async () => {
  const routeConnId = route.query.connectionId as string | undefined
  const connId = routeConnId || connectionStore.activeConnectionId

  if (connId) {
    tabStore.reconcileTabsForConnection(connId)
    selectedConnectionId.value = connId

    const isConnected = connectionStore.getConnectionStatus(connId) === ConnectionStatus.CONNECTED
    if (isConnected) {
      const currentDb = connectionStore.getCurrentDatabase(connId)
      if (currentDb) {
        selectedDatabase.value = currentDb
      }

      await databaseStore.fetchDatabases(connId)
    }
  }
})

watch(selectedConnectionId, (newConnId, oldConnId) => {
  if (!newConnId || newConnId === oldConnId) {
    return
  }

  selectedDatabase.value = ''
  selectedSchema.value = ''
  databaseStore.resetSelection()

  tabStore.reconcileTabsForConnection(newConnId)
  if (oldConnId) {
    databaseStore.clearMetadata(oldConnId)
  }
}, { flush: 'sync' })

// Async operations after tabs are closed
watch(selectedConnectionId, async (newConnId, oldConnId) => {
  if (!newConnId || newConnId === oldConnId) {
    return
  }

  const alreadyConnected = connectionStore.getConnectionStatus(newConnId) === ConnectionStatus.CONNECTED

  if (alreadyConnected) {
    connectionStore.setActiveConnection(newConnId)
    const currentDb = connectionStore.getCurrentDatabase(newConnId)
    if (currentDb) {
      selectedDatabase.value = currentDb
    }
    await databaseStore.fetchDatabases(newConnId)
    return
  }

  try {
    await connectionStore.connect(newConnId)
    connectionStore.setActiveConnection(newConnId)
    await databaseStore.fetchDatabases(newConnId)

    const currentDb = connectionStore.getCurrentDatabase(newConnId)
    if (currentDb) {
      selectedDatabase.value = currentDb
    }
  }
  catch (error) {
    console.error('Failed to connect:', error)
  }
})

// Persist the selected database back to the store and sync the active tab
watch(selectedDatabase, (db) => {
  const connId = selectedConnectionId.value || connectionStore.activeConnectionId
  if (connId && db) {
    connectionStore.setCurrentDatabase(connId, db)
  }
  // Keep the active tab's database in sync so queries run against the selected DB
  if (db && activeTab.value) {
    activeTab.value.database = db
  }
  // Reset schema when database changes
  selectedSchema.value = ''
})

// When the active tab changes, sync selectedDatabase to reflect that tab's DB
watch(activeTab, (tab) => {
  if (tab?.database && tab.database !== selectedDatabase.value) {
    selectedDatabase.value = tab.database
  }
})

const getConnectionId = () => selectedConnectionId.value || connectionStore.activeConnectionId

const isTableViewConnectionValid = computed(() => {
  if (!activeTab.value?.tableView)
    return true
  if (activeTab.value.orphanFromConnectionId)
    return false

  const connId = getConnectionId()
  if (!connId)
    return false

  const status = connectionStore.getConnectionStatus(connId)
  if (status !== ConnectionStatus.CONNECTED)
    return false

  return true
})

const isErDiagramConnectionValid = computed(() => {
  if (!activeTab.value?.erDiagram)
    return true
  if (activeTab.value.orphanFromConnectionId)
    return false
  const connId = getConnectionId()
  if (!connId)
    return false
  return connectionStore.getConnectionStatus(connId) === ConnectionStatus.CONNECTED
})

function isConnectionActive(connId: string | null | undefined): boolean {
  return connId !== null && connId !== undefined && connectionStore.getConnectionStatus(connId) === ConnectionStatus.CONNECTED
}

function getActiveConnectionId(): string | null {
  const connId = getConnectionId()
  return isConnectionActive(connId) ? connId : null
}

const listingTabObjects = computed(() => {
  const tab = activeTab.value
  const connId = getConnectionId()
  if (!tab?.listingTab || !connId) {
    return null
  }
  const meta = databaseStore.metadata[connId]
  if (!meta) {
    return null
  }
  const objectKey = tab.listingTab.schema
    ? `${tab.listingTab.database}.${tab.listingTab.schema}`
    : tab.listingTab.database
  const schemaObjects = meta.objects[objectKey]
  if (!schemaObjects) {
    return null
  }
  switch (tab.listingTab.type) {
    case 'VIEW': return schemaObjects.views
    case 'PROCEDURE': return schemaObjects.procedures
    case 'FUNCTION': return schemaObjects.functions
    default: return null
  }
})

async function executeQuery(details?: StatementToExecute) {
  if (!activeTab.value || activeTab.value.orphanFromConnectionId) {
    return
  }

  const tabContent = activeTab.value.content || ''
  if (!tabContent.trim()) {
    return
  }

  const sqlToExecute = details?.found ? details.sql : tabContent

  if (!sqlToExecute?.trim()) {
    return
  }

  const connId = getConnectionId()
  if (!connId) {
    return
  }

  showResultPanel.value = true
  await tabStore.executeQuery(activeTab.value.id, sqlToExecute)
}

const explainAnalyzeMode = ref(false)

async function handleExplainQuery(analyze = false) {
  if (!activeTab.value || activeTab.value.orphanFromConnectionId)
    return
  if (!activeTab.value.content.trim())
    return
  const connId = getConnectionId()
  if (!connId)
    return
  showResultPanel.value = true
  await tabStore.explainQuery(activeTab.value.id, analyze)
}

function toggleExplainMode() {
  explainAnalyzeMode.value = !explainAnalyzeMode.value
}

function getActiveDialect(): string | null {
  const connId = selectedConnectionId.value || connectionStore.activeConnectionId
  if (!connId)
    return null
  const conn = connectionStore.getConnectionById(connId)
  if (!conn)
    return null
  const dbType = conn.type as DatabaseType
  return resolveDialect(dbType)
}

function handleFormatSql(sql?: string): string {
  const content = sql ?? activeTab.value?.content ?? ''
  if (!content.trim())
    return content

  const dialect = getActiveDialect() || 'sql'
  const { indentWidth, lineWidth } = appStore.editorConfig

  const result = formatSql(content, dialect, {
    tabWidth: indentWidth,
    expressionWidth: lineWidth,
  })

  if (result.error) {
    toast.error(t('pages.queries.notifications.formatFailed'), {
      description: result.error,
    })
  }

  return result.sql
}

function handleToolbarFormat() {
  if (activeTab.value?.orphanFromConnectionId || !activeTab.value)
    return
  if (activeTab.value.content.trim()) {
    const formatted = handleFormatSql()
    if (formatted !== activeTab.value.content) {
      tabStore.updateTabContent(activeTab.value.id, formatted)
    }
  }
}

function handleNewTab() {
  const connId = getActiveConnectionId()
  const db = connId
    ? (selectedDatabase.value || connectionStore.getCurrentDatabase(connId) || connectionStore.getConnectionById(connId)?.database || undefined)
    : undefined
  tabStore.createTab(connId ?? undefined, db)
}

function handleTabSelect(tabId: string) {
  const tab = tabStore.tabById(tabId)
  if (tab?.orphanFromConnectionId) {
    orphanTabToHandle.value = tabId
    showOrphanTabDialog.value = true
    return
  }
  tabStore.setActiveTab(tabId)
}

function handleOrphanTabClose() {
  if (orphanTabToHandle.value) {
    tabStore.closeTab(orphanTabToHandle.value)
    orphanTabToHandle.value = null
  }
  showOrphanTabDialog.value = false
}

function handleOrphanTabAcknowledge() {
  if (orphanTabToHandle.value) {
    tabStore.setActiveTab(orphanTabToHandle.value)
    orphanTabToHandle.value = null
  }
  showOrphanTabDialog.value = false
}

function handleTabClose(tabId: string) {
  tabStore.closeTab(tabId)
}

function handleTabCloseForce(tabId: string) {
  tabStore.closeTab(tabId)
}

function handleGlobalKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'w') {
    const tab = tabStore.activeTab
    if (tab) {
      e.preventDefault()
      e.stopPropagation()
      queryTabsRef.value?.triggerClose(tab.id)
    }
  }
  if (e.key === 'F6') {
    // Don't fire F6 when user is typing in an input/textarea/contenteditable
    const target = e.target as HTMLElement
    const isInput = target.tagName === 'INPUT'
      || target.tagName === 'TEXTAREA'
      || target.isContentEditable
    if (!isInput) {
      e.preventDefault()
      handleExplainQuery(explainAnalyzeMode.value)
    }
  }
}

onMounted(() => window.addEventListener('keydown', handleGlobalKeydown))
onUnmounted(() => window.removeEventListener('keydown', handleGlobalKeydown))

function handleCreateScript(table: TableInfo, database: string, schema?: string) {
  const schemaPrefix = schema ? `"${schema}".` : ''
  const script = `-- CREATE TABLE script for ${table.name}
-- TODO: Generate actual CREATE TABLE from server
CREATE TABLE ${schemaPrefix}"${table.name}" (
  -- columns will be generated here
);`

  const connId = getActiveConnectionId()
  if (connId) {
    const tab = tabStore.createTab(connId, database, schema)
    tabStore.updateTabContent(tab.id, script)
    tabStore.updateTabName(tab.id, `CREATE_${table.name}.sql`)
  }
}

function handleSelectTopN(table: TableInfo, database: string, schema?: string, n = 100) {
  const schemaPrefix = schema ? `"${schema}".` : ''
  const query = `SELECT * FROM ${schemaPrefix}"${table.name}" LIMIT ${n};`

  const connId = getActiveConnectionId()
  if (connId) {
    const tab = tabStore.createTab(connId, database, schema)
    tabStore.updateTabContent(tab.id, query)
    tabStore.updateTabName(tab.id, `SELECT_${table.name}`)

    tabStore.executeQuery(tab.id)
    showResultPanel.value = true
  }
}

function handleViewStructure(table: TableInfo, database: string, schema?: string) {
  const query = `-- Table structure for ${table.name}
-- Database: ${database}${schema ? `\n-- Schema: ${schema}` : ''}

-- View columns
SELECT column_name, data_type, is_nullable, column_default
FROM information_schema.columns
WHERE table_name = '${table.name}'${schema ? ` AND table_schema = '${schema}'` : ''};`

  const connId = getActiveConnectionId()
  if (connId) {
    const tab = tabStore.createTab(connId, database, schema)
    tabStore.updateTabContent(tab.id, query)
    tabStore.updateTabName(tab.id, `STRUCTURE_${table.name}`)
  }
}

function handleExportData(_table: TableInfo, _database: string, _schema?: string) {
  // TODO: Implement export data functionality
}

function handleShowErDiagram(database: string, schema?: string) {
  const connId = getActiveConnectionId()
  if (connId)
    tabStore.openErDiagramTab(connId, database, schema)
}

function handleDropTable(table: TableInfo, database: string, schema?: string) {
  destructiveAction.value = { type: 'drop', table, database, schema }
  destructiveDialogOpen.value = true
}

function handleTruncateTable(table: TableInfo, database: string, schema?: string) {
  destructiveAction.value = { type: 'truncate', table, database, schema }
  destructiveDialogOpen.value = true
}

async function handleDestructiveConfirm() {
  const action = destructiveAction.value
  if (!action)
    return

  isDestructiveActionExecuting.value = true
  const connId = getActiveConnectionId()
  if (!connId) {
    toast.error('No active connection')
    isDestructiveActionExecuting.value = false
    return
  }

  const schemaPrefix = action.schema ? `"${action.schema}".` : ''
  const qualifiedName = `${schemaPrefix}"${action.table.name}"`
  const sql = action.type === 'drop'
    ? `DROP TABLE IF EXISTS ${qualifiedName};`
    : `TRUNCATE TABLE ${qualifiedName};`

  try {
    await invoke('execute_query', {
      connectionId: connId,
      sql,
    })
    const actionLabel = action.type === 'drop' ? 'dropped' : 'truncated'
    toast.success(`Table ${actionLabel} successfully`)
    destructiveDialogOpen.value = false
    destructiveAction.value = null
    databaseBrowserRef.value?.refreshTree()
  }
  catch (err) {
    toast.error(`Failed to ${action.type} table: ${err instanceof Error ? err.message : String(err)}`)
  }
  finally {
    isDestructiveActionExecuting.value = false
  }
}

function handleSelectTable(table: TableInfo, database: string, schema?: string) {
  const connId = getActiveConnectionId()
  if (!connId)
    return
  tabStore.openTableViewTab(connId, database, table.name, schema)
}

function handleOpenListingTab(type: 'VIEW' | 'PROCEDURE' | 'FUNCTION', database: string, schema?: string) {
  const connId = getActiveConnectionId()
  if (!connId)
    return
  tabStore.openListingTab(connId, database, type, schema)
}

function handleOpenDdlTab(name: string, type: string, database: string, schema?: string) {
  if (type === 'VIEW') {
    const connId = getActiveConnectionId()
    if (!connId)
      return
    tabStore.openTableViewTab(connId, database, name, schema)
    return
  }
  const connId = getActiveConnectionId()
  if (!connId)
    return
  const listingType = type === 'PROCEDURE' ? 'PROCEDURE' : 'FUNCTION'
  tabStore.openListingTab(connId, database, listingType, schema)
}

function handleOpenFromListing(info: { name: string, type: string, schema?: string }, database: string) {
  handleOpenDdlTab(info.name, info.type, database, info.schema)
}

async function handleRefreshListingTab() {
  const tab = activeTab.value
  const connId = getConnectionId()
  if (!tab?.listingTab || !connId) {
    return
  }
  try {
    const schema = tab.listingTab.schema || ''
    if (!schema) {
      return
    }
    await databaseStore.fetchSchemaObjects(connId, tab.listingTab.database, schema)
  }
  catch {
    // ignore
  }
}

async function handleOpenSavedQuery(filePath: string) {
  const existingTab = tabStore.tabByFilePath(filePath)
  if (existingTab) {
    tabStore.setActiveTab(existingTab.id)
    return
  }

  const connId = getActiveConnectionId()
  const db = connId
    ? (selectedDatabase.value || connectionStore.getCurrentDatabase(connId) || connectionStore.getConnectionById(connId)?.database || undefined)
    : undefined

  const tab = tabStore.createTab(connId ?? undefined, db)

  try {
    const result = await loadQueryFile(filePath)
    if (result.success && result.content) {
      tabStore.updateTabContent(tab.id, result.content)
      tabStore.markTabSaved(tab.id, filePath)
    }
    else {
      toast.error(t('pages.queries.notifications.loadFailed'), { description: result.message })
    }
  }
  catch (error) {
    toast.error(t('pages.queries.notifications.loadFailed'), { description: error instanceof Error ? error.message : String(error) })
  }
}

function sanitizeFileName(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 30) || 'query'
}

async function handleSaveQuery() {
  if (!activeTab.value || !activeTab.value.content.trim()) {
    return
  }

  const connectionName = activeConnection.value?.name || 'query'
  const tabName = activeTab.value.name || 'query'
  const defaultFileName = activeTab.value.filePath
    ? undefined
    : `${sanitizeFileName(connectionName)}-${sanitizeFileName(tabName)}.sql`

  try {
    const result = await saveQueryFile(
      activeTab.value.content,
      activeTab.value.filePath,
      defaultFileName,
    )

    if (result.success && result.file_path) {
      tabStore.markTabSaved(activeTab.value.id, result.file_path)
      toast.success(t('pages.queries.notifications.querySaved'), { description: result.file_path })
      databaseBrowserRef.value?.fetchSavedQueryFiles()
    }
    else {
      toast.error(t('pages.queries.notifications.saveFailed'), { description: result.message })
    }
  }
  catch (error) {
    toast.error(t('pages.queries.notifications.saveFailed'), { description: error instanceof Error ? error.message : String(error) })
  }
}

async function handleDownloadQuery() {
  if (!activeTab.value || !activeTab.value.content.trim()) {
    return
  }

  const connectionName = activeConnection.value?.name || 'query'
  const tabName = activeTab.value.name || 'query'
  const defaultFileName = `${sanitizeFileName(connectionName)}-${sanitizeFileName(tabName)}.sql`

  try {
    const result = await saveQueryFileAs(
      activeTab.value.content,
      defaultFileName,
    )
    if (!result) {
      return
    }
    if (result.success && result.file_path) {
      tabStore.markTabSaved(activeTab.value.id, result.file_path)
      toast.success(t('pages.queries.notifications.querySaved'), { description: result.file_path })
      databaseBrowserRef.value?.fetchSavedQueryFiles()
    }
    else {
      toast.error(t('pages.queries.notifications.loadFailed'), { description: result.message })
    }
  }
  catch (error) {
    toast.error(t('pages.queries.notifications.loadFailed'), { description: error instanceof Error ? error.message : String(error) })
  }
}

function startSidebarResize(_e: MouseEvent) {
  isResizingSidebar.value = true
  document.addEventListener('mousemove', handleSidebarResize)
  document.addEventListener('mouseup', stopSidebarResize)
}

function handleSidebarResize(e: MouseEvent) {
  if (!isResizingSidebar.value)
    return
  const newWidth = Math.max(200, Math.min(400, e.clientX))
  sidebarWidth.value = newWidth
}

function stopSidebarResize() {
  isResizingSidebar.value = false
  document.removeEventListener('mousemove', handleSidebarResize)
  document.removeEventListener('mouseup', stopSidebarResize)
}

function closeResultPanel() {
  showResultPanel.value = false
}
</script>

<template>
  <AppLayout>
    <div class="flex flex-col h-full">
      <div class="flex flex-1 overflow-hidden">
        <!-- Database Browser Sidebar -->
        <div
          class="border-r bg-background flex flex-col"
          :style="{ width: `${sidebarWidth}px` }"
        >
          <!-- Connection selector -->
          <div class="p-2 border-b">
            <Select v-model="selectedConnectionId">
              <SelectTrigger class="text-xs h-8 min-w-0">
                <SelectValue :placeholder="t('pages.queries.selectConnection')" class="truncate" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="conn in availableConnections"
                  :key="conn.id"
                  :value="conn.id!"
                >
                  <span>{{ conn.name }}</span>
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <!-- Database Browser -->
          <DatabaseBrowser
            ref="databaseBrowserRef"
            v-model:selected-database="selectedDatabase"
            v-model:selected-schema="selectedSchema"
            :connection-id="selectedConnectionId"
            class="flex-1"
            @select-table="handleSelectTable"
            @create-script="handleCreateScript"
            @select-top-n="handleSelectTopN"
            @view-structure="handleViewStructure"
            @export-data="handleExportData"
            @show-er-diagram="handleShowErDiagram"
            @drop-table="handleDropTable"
            @truncate-table="handleTruncateTable"
            @open-saved-query="handleOpenSavedQuery"
            @create-new-query="handleNewTab"
            @open-listing-tab="handleOpenListingTab"
            @open-ddl-tab="handleOpenDdlTab"
          />
        </div>

        <!-- Resize handle -->
        <div
          class="w-1 cursor-ew-resize transition-colors hover:bg-primary/20"
          @mousedown="startSidebarResize"
        />

        <!-- Editor area -->
        <div class="flex flex-1 flex-col overflow-hidden">
          <!-- Tabs -->
          <QueryTabs
            ref="queryTabsRef"
            :tabs="tabStore.tabs"
            :active-tab-id="tabStore.activeTabId"
            :active-connection-id="getConnectionId() || undefined"
            @select="handleTabSelect"
            @close="handleTabClose"
            @close-force="handleTabCloseForce"
            @new="handleNewTab"
          />

          <!-- Data Table View (shown when the active tab is a table-view tab AND connection matches) -->
          <DataTableView
            v-if="activeTab?.tableView && !activeTab.orphanFromConnectionId && isTableViewConnectionValid"
            :key="`${getConnectionId()}-${activeTab.id}-${activeTab.tableView.database}-${activeTab.tableView.tableName}`"
            :connection-id="getConnectionId() || ''"
            :database="activeTab.tableView.database"
            :schema="activeTab.tableView.schema"
            :table-name="activeTab.tableView.tableName"
            class="flex-1"
          />

          <!-- Invalid table-view state (connection mismatch - tab being closed) -->
          <div
            v-else-if="activeTab?.tableView && !isTableViewConnectionValid"
            class="flex flex-1 items-center justify-center"
          >
            <div class="text-muted-foreground text-center">
              <svg class="mx-auto mb-2 h-8 w-8 animate-spin" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              <p class="text-sm">
                {{ t('pages.queries.status.executing') }}
              </p>
            </div>
          </div>

          <!-- ER Diagram view -->
          <ErDiagramView
            v-else-if="activeTab?.erDiagram && !activeTab.orphanFromConnectionId && isErDiagramConnectionValid"
            :connection-id="getConnectionId() || ''"
            :database="activeTab.erDiagram.database"
            :schema="activeTab.erDiagram.schema"
            class="flex-1"
          />

          <!-- Listing Tab (Views / Procedures / Functions) -->
          <ListingTab
            v-else-if="activeTab?.listingTab && !activeTab.orphanFromConnectionId && listingTabObjects"
            :key="`${getConnectionId()}-${activeTab.id}-${activeTab.listingTab.type}-${activeTab.listingTab.database}`"
            :connection-id="getConnectionId() || ''"
            :database="activeTab.listingTab.database"
            :schema="activeTab.listingTab.schema ?? null"
            :type="activeTab.listingTab.type"
            :objects="listingTabObjects"
            :loading="false"
            :error="null"
            class="flex-1"
            @refresh="handleRefreshListingTab"
            @open-object="(obj: any) => handleOpenFromListing({ name: obj.name, type: obj.object_type, schema: obj.schema }, activeTab?.listingTab?.database || '')"
          />

          <!-- Query editor area (shown for normal query tabs) -->
          <template v-else>
            <!-- Toolbar -->
            <div class="px-2 py-1 border-b bg-muted/30 flex gap-2 items-center">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      variant="ghost"
                      size="sm"
                      class="gap-1 h-7"
                      :disabled="!activeTab || activeTab.orphanFromConnectionId"
                      @click="executeQuery"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polygon points="6 3 20 12 6 21 6 3" />
                      </svg>
                      {{ t('pages.queries.editor.execute') }}
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.queries.shortcuts.execute', { key: modifierKey }) }}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>

              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      variant="ghost"
                      size="sm"
                      class="gap-1 h-7"
                      :class="{ 'text-violet-600': !explainAnalyzeMode, 'text-green-600': explainAnalyzeMode }"
                      :disabled="!activeTab || activeTab.orphanFromConnectionId || activeTab.isExplaining"
                      @click="handleExplainQuery(explainAnalyzeMode)"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M12 20h9" />
                        <path d="M16.376 3.622a1 1 0 0 1 3.002 3.002L7.368 18.635a2 2 0 0 1-.855.506l-2.872.838a.5.5 0 0 1-.62-.62l.838-2.872a2 2 0 0 1 .506-.854z" />
                      </svg>
                      {{ t('pages.queries.editor.explain') }}
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.queries.shortcuts.explain', { key: modifierKey }) }}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      variant="ghost"
                      size="sm"
                      class="p-0 h-7 w-7"
                      :class="explainAnalyzeMode ? 'text-green-600 bg-green-100 dark:text-green-300 dark:bg-green-900/30' : 'text-muted-foreground/50'"
                      :disabled="!activeTab || activeTab.orphanFromConnectionId || activeTab.isExplaining"
                      @click="toggleExplainMode"
                    >
                      <span class="text-xs font-bold">A</span>
                    </Button>
                  </TooltipTrigger>
                    <TooltipContent>
                      <p>{{ t('pages.queries.explain.analyzeToggle', { mode: explainAnalyzeMode ? 'ON' : 'OFF' }) }}</p>
                    </TooltipContent>
                </Tooltip>
              </TooltipProvider>

              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      variant="ghost"
                      size="sm"
                      class="gap-1 h-7"
                      :disabled="!activeTab || activeTab.orphanFromConnectionId"
                      @click="handleToolbarFormat"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M18.37 2.63 14 7l-1.59-1.59a2 2 0 0 0-2.82 0L8 7l9 9 1.59-1.59a2 2 0 0 0 0-2.82L17 10l4.37-4.37a2 2 0 0 0 0-2.82l-1.18-1.18a2 2 0 0 0-2.82 0z" />
                        <line x1="9" y1="13" x2="15" y2="19" />
                        <path d="M3 21h4l10.85-10.85" />
                      </svg>
                      {{ t('pages.queries.editor.format') }}
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.queries.shortcuts.format') }}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>

              <div class="flex-1" />

              <!-- Download / Save As -->
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      variant="ghost"
                      size="sm"
                      class="p-0 h-7 w-7"
                      :disabled="!activeTab || !activeTab.content.trim()"
                      @click="handleDownloadQuery"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                        <polyline points="7 10 12 15 17 10" />
                        <line x1="12" x2="12" y1="15" y2="3" />
                      </svg>
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.queries.shortcuts.saveAs', { key: modifierKey }) }}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>

              <!-- Status info -->
              <div v-if="activeTab" class="text-xs text-muted-foreground flex gap-2 items-center">
                <span v-if="activeConnection">{{ activeConnection.name }}</span>
                <span v-if="activeConnection?.database">/ {{ activeConnection.database }}</span>
              </div>
            </div>

            <!-- Editor -->
            <div class="flex-1 overflow-hidden">
              <SQLEditor
                v-if="activeTab"
                v-model="editorContent"
                height="100%"
                dialect="sql"
                :is-executing="activeTab.isExecuting"
                :font-size="appStore.editorConfig.fontSize"
                :tab-size="appStore.editorConfig.tabSize"
                :word-wrap="appStore.editorConfig.wordWrap"
                :minimap="appStore.editorConfig.showMinimap"
                :show-line-numbers="appStore.editorConfig.showLineNumbers"
                :format-sql="handleFormatSql"
                @execute="(details) => executeQuery(details)"
                @statement-not-found="toast.error(t('pages.queries.notifications.noStatementFound'))"
                @save="handleSaveQuery"
              />
              <div v-else class="flex h-full items-center justify-center">
                <div class="px-6 text-center max-w-md">
                  <!-- Connection info card when connected -->
                  <div v-if="activeConnection && getConnectionId()" class="mb-6 p-6 rounded-lg bg-muted/50">
                    <div class="mb-4 flex gap-3 items-center justify-center">
                      <DbTypeIcon
                        v-if="activeConnection.type"
                        :type="activeConnection.type"
                        :size="24"
                      />
                      <span class="text-lg font-semibold">{{ activeConnection.name }}</span>
                    </div>
                    <div class="text-sm text-muted-foreground space-y-1">
                      <p>{{ activeConnection.host }}:{{ activeConnection.port }}</p>
                      <p v-if="activeConnection.username">
                        {{ t('pages.queries.landing.connectedAs') }}: {{ activeConnection.username }}
                      </p>
                      <p v-if="selectedDatabase">
                        {{ t('pages.queries.landing.currentDatabase') }}: {{ selectedDatabase }}
                      </p>
                    </div>
                  </div>

                  <!-- Welcome message -->
                  <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" class="text-muted-foreground/50 mx-auto mb-4">
                    <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
                    <polyline points="14 2 14 8 20 8" />
                  </svg>
                  <p class="text-muted-foreground mb-4">
                    {{ getConnectionId() ? t('pages.queries.landing.ready') : t('pages.queries.noTab') }}
                  </p>

                  <!-- Action buttons -->
                  <div class="flex flex-col gap-2">
                    <Button v-if="getConnectionId()" variant="default" size="sm" class="w-full" @click="handleNewTab">
                      {{ t('pages.queries.newTab') }}
                    </Button>
                    <Button v-if="!getConnectionId()" variant="outline" size="sm" class="w-full" @click="router.push('/connections')">
                      {{ t('pages.queries.landing.selectConnection') }}
                    </Button>
                  </div>

                  <!-- Quick tips -->
                  <div v-if="getConnectionId()" class="text-xs text-muted-foreground mt-6">
                    <p>{{ t('pages.queries.landing.tip') }}</p>
                  </div>
                </div>
              </div>
            </div>

            <!-- Result Panel -->
            <QueryResultPanel
              v-if="showResultPanel"
              :results="activeTab?.results"
              :error="activeTab?.error"
              :is-executing="activeTab?.isExecuting"
              :execution-time="activeTab?.executionTime"
              :sql="activeTab?.content"
              :connection-id="getConnectionId() ?? undefined"
              :database="activeTab?.database"
              :explain-plan="activeTab?.explainPlan ?? null"
              :is-explaining="activeTab?.isExplaining ?? false"
              :explain-error="activeTab?.explainError ?? null"
              @close="closeResultPanel"
              @refresh="handleNewTab"
            />
          </template>
        </div>
      </div>

      <!-- Status bar -->
      <div class="text-xs text-muted-foreground px-3 py-1 border-t bg-muted/30 flex items-center justify-between">
        <div class="flex gap-4 items-center">
          <span v-if="activeConnection?.database">
            {{ t('pages.queries.status.database') }}: {{ activeConnection.database }}
          </span>
        </div>
        <div class="flex gap-4 items-center">
          <span v-if="activeTab?.results">
            {{ t('pages.queries.status.rows') }}: {{ activeTab.results.rowCount }}
          </span>
          <span v-if="activeTab?.executionTime">
            {{ t('pages.queries.status.time') }}: {{ activeTab.executionTime }}ms
          </span>
          <span v-if="activeTab?.isExecuting" class="text-primary">
            {{ t('pages.queries.status.executing') }}
          </span>
        </div>
      </div>
    </div>

    <!-- Destructive Action Confirmation Dialog (Drop/Truncate Table) -->
    <DestructiveConfirmDialog
      v-if="destructiveAction"
      v-model:open="destructiveDialogOpen"
      :title="destructiveAction.type === 'drop'
        ? t('components.destructiveDialog.dropTable.title')
        : t('components.destructiveDialog.truncateTable.title')"
      :message="destructiveAction.type === 'drop'
        ? t('components.destructiveDialog.dropTable.message', { table: destructiveAction.table.name })
        : t('components.destructiveDialog.truncateTable.message', { table: destructiveAction.table.name })"
      :detail="destructiveAction.type === 'drop'
        ? t('components.destructiveDialog.dropTable.detail')
        : t('components.destructiveDialog.truncateTable.detail')"
      :confirm-label="destructiveAction.type === 'drop'
        ? t('components.destructiveDialog.dropTable.confirm')
        : t('components.destructiveDialog.truncateTable.confirm')"
      :loading="isDestructiveActionExecuting"
      @confirm="handleDestructiveConfirm"
      @update:open="(v) => { if (!v) destructiveAction = null }"
    />

    <!-- Orphan Tab Warning Dialog -->
    <AlertDialog v-model:open="showOrphanTabDialog">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{{ t('pages.queries.orphanDialog.title') }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{ t('pages.queries.orphanDialog.message') }}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel @click="handleOrphanTabAcknowledge">
            {{ t('pages.queries.orphanDialog.acknowledge') }}
          </AlertDialogCancel>
          <AlertDialogAction @click="handleOrphanTabClose">
            {{ t('pages.queries.orphanDialog.close') }}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </AppLayout>
</template>
