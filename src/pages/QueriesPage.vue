<script setup lang="ts">
import type { StatementToExecute } from '@/composables/useSqlStatements'
import type { DatabaseType } from '@/store/connectionStore'
import type { TableInfo } from '@/store/databaseStore'
import type { ApiResponse } from '@/types/api'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { DataTableView, DbTypeIcon, QueryResultPanel, QueryTabs } from '@/components/database-browser'
import ListingTab from '@/components/database-browser/ListingTab.vue'
import ErDiagramView from '@/components/er-diagram/ErDiagramView.vue'
import AppLayout from '@/components/layout/AppLayout.vue'
import { ConnectionSelector, CreateDatabaseDialog, CreateObjectDialog, CreateTableDialog, DatabaseSelectorRow, DropDatabaseDialog, SavedQueriesPanel, SchemaTree, SidebarSplitView } from '@/components/sidebar'
import SQLEditor from '@/components/SQLEditor.vue'
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { DestructiveConfirmDialog } from '@/components/ui/destructive-confirm-dialog'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { toast } from '@/composables/useNotifications'
import { usePlatform } from '@/composables/usePlatform'
import { useSqlFormatter } from '@/composables/useSqlFormatter'
import { browseApi, loadQueryFile, saveQueryFile, saveQueryFileAs, saveQueryMetadata } from '@/datasources'
import { ConnectionStatus, useAppStore, useConnectionStore, useDatabaseStore, useTabStore } from '@/store'
import { isApiSuccess } from '@/types/api'

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
const schemaTreeRef = ref<InstanceType<typeof SchemaTree>>()
const savedQueriesRef = ref<InstanceType<typeof SavedQueriesPanel>>()
const savedQueriesCollapsed = ref(true)

const { formatSql, resolveDialect } = useSqlFormatter()

// ── Destructive action dialog state ──
const destructiveDialogOpen = ref(false)
const destructiveAction = ref<{ type: 'drop' | 'truncate', table: TableInfo, database: string, schema?: string } | null>(null)
const isDestructiveActionExecuting = ref(false)
const showOrphanTabDialog = ref(false)
const orphanTabToHandle = ref<string | null>(null)

// ── Database action dialog state ──
const createDatabaseDialogOpen = ref(false)
const createSchemaDialogOpen = ref(false)
const dropDatabaseDialogOpen = ref(false)
const createTableDialogOpen = ref(false)
const isDatabaseActionExecuting = ref(false)

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
    await databaseStore.fetchDatabases(newConnId)
    selectedDatabase.value = connectionStore.getCurrentDatabase(newConnId) || ''
    return
  }

  try {
    await connectionStore.connect(newConnId)
    connectionStore.setActiveConnection(newConnId)
    await databaseStore.fetchDatabases(newConnId)
    selectedDatabase.value = connectionStore.getCurrentDatabase(newConnId) || ''
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

const getConnectionId = () => selectedConnectionId.value || connectionStore.activeConnectionId

const isTableViewConnectionValid = computed(() => {
  if (!activeTab.value?.tableView)
    return true
  if (activeTab.value.orphanFromConnectionId)
    return false

  const connId = activeTab.value.connectionId
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
    toast.warning(t('pages.queries.notifications.selectConnectionFirst'))
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
  if (!connId) {
    toast.warning(t('pages.queries.notifications.selectConnectionFirst'))
    return
  }
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
    toast.error(t('pages.queries.notifications.noActiveConnection'))
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
    toast.success(t('pages.queries.notifications.tableActionSuccess', { action: actionLabel }))
    destructiveDialogOpen.value = false
    destructiveAction.value = null
    schemaTreeRef.value?.refresh()
  }
  catch (err) {
    toast.error(t('pages.queries.notifications.tableActionFailed', { action: action.type, error: err instanceof Error ? err.message : String(err) }))
  }
  finally {
    isDestructiveActionExecuting.value = false
  }
}

function handleSelectTable(table: TableInfo, database: string, schema?: string) {
  const connId = getActiveConnectionId()
  if (!connId)
    return
  // Execute USE database to set the backend connection's database context.
  // The Rust get_table_data MySQL branch ignores query.database, requiring
  // the connection to already have a database selected.
  const escapedDb = database.replace(/`/g, '``')
  invoke('execute_query', { connectionId: connId, sql: `USE \`${escapedDb}\`` }).catch(() => {
    // Best-effort: some databases don't support USE (e.g. SQLite)
  })
  tabStore.openTableViewTab(connId, database, table.name, schema)
}

function handleOpenListingTab(type: 'VIEW' | 'PROCEDURE' | 'FUNCTION', database: string, schema?: string) {
  const connId = getActiveConnectionId()
  if (!connId)
    return
  tabStore.openListingTab(connId, database, type, schema)
}

async function handleOpenDdlTab(name: string, type: string, database: string, schema?: string) {
  const connId = getActiveConnectionId()
  if (!connId)
    return

  if (type === 'VIEW') {
    tabStore.openTableViewTab(connId, database, name, schema)
    return
  }

  // For PROCEDURE / FUNCTION: fetch DDL and open in a query tab
  const tab = tabStore.createTab(connId, database, schema)
  const objectType = type === 'PROCEDURE' ? 'PROCEDURE' : 'FUNCTION'
  tabStore.updateTabName(tab.id, `${name}.sql`)
  try {
    const ddl = await browseApi.getObjectDdl(connId, database, schema ?? null, name, objectType)
    const header = `-- ${objectType}: ${schema ? `${schema}.` : ''}${name}\n\n`
    tabStore.updateTabContent(tab.id, header + ddl)
  }
  catch (err) {
    tabStore.updateTabContent(tab.id, `-- Failed to load DDL for ${name}:\n-- ${String(err)}`)
  }
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

async function saveMetadataEntry(filePath: string) {
  const now = Math.floor(Date.now() / 1000)
  const connId = getConnectionId()
  const conn = connId ? connectionStore.getConnectionById(connId) : undefined
  try {
    await saveQueryMetadata(filePath, {
      connectionId: connId ?? null,
      connectionName: conn?.name ?? null,
      createdAt: now,
      modifiedAt: now,
    })
  }
  catch (error) {
    console.error('Failed to save metadata:', error)
  }
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
      await saveMetadataEntry(result.file_path)
      toast.success(t('pages.queries.notifications.querySaved'), { description: result.file_path })
      savedQueriesRef.value?.refresh()
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
      await saveMetadataEntry(result.file_path)
      toast.success(t('pages.queries.notifications.querySaved'), { description: result.file_path })
      savedQueriesRef.value?.refresh()
    }
    else {
      toast.error(t('pages.queries.notifications.loadFailed'), { description: result.message })
    }
  }
  catch (error) {
    toast.error(t('pages.queries.notifications.loadFailed'), { description: error instanceof Error ? error.message : String(error) })
  }
}

function handleDatabaseRefresh() {
  schemaTreeRef.value?.refresh()
}

function handleDatabaseAction(kind: string) {
  switch (kind) {
    case 'newDatabase':
      createDatabaseDialogOpen.value = true
      break
    case 'newSchema':
      createSchemaDialogOpen.value = true
      break
    case 'dropDatabase':
      dropDatabaseDialogOpen.value = true
      break
    case 'newTable':
      createTableDialogOpen.value = true
      break
    case 'newView':
    case 'newFunction':
    case 'newProcedure':
      openTemplateTab(kind)
      break
    case 'backupDatabase':
    case 'exportDatabase':
      router.push('/transfer')
      break
    case 'showErDiagram': {
      const connId = getActiveConnectionId()
      const db = selectedDatabase.value
      // Database-level ER diagram: omit schema to show all tables across all schemas
      if (connId && db) {
        tabStore.openErDiagramTab(connId, db, undefined)
      }
      break
    }
    default:
      toast.info(t('sidebar.notImplemented'))
  }
}

// ── Database CRUD handlers ──

async function handleCreateDatabase(name: string, options: { charset?: string, collation?: string, encoding?: string, locale?: string }) {
  const connId = getActiveConnectionId()
  if (!connId)
    return
  isDatabaseActionExecuting.value = true
  try {
    const dbType = activeConnection.value?.type

    // Build database-specific CREATE DATABASE SQL
    const MYSQL_SET = new Set(['MYSQL', 'MARIADB', 'TIDB', 'OCEANBASE', 'TDSQL', 'POLARDB', 'DORIS', 'SELECTDB', 'STARROCKS', 'DATABEND', 'GOLDENDB', 'MANTICORESEARCH', 'SINGLESTOREMEMSQL', 'CLOUDSQLMYSQL'])
    const PG_SET = new Set(['POSTGRESQL', 'COCKROACHDB', 'REDSHIFT', 'YUGABYTEDB', 'TIMESCALEDB', 'KINGBASEES', 'GAUSSDB', 'HIGHGO', 'UXDB', 'OPENGAUSS', 'GBASE8C', 'QUESTDB', 'VASTBASE', 'YASHANDB', 'GREENPLUM', 'ENTERPRISEDB', 'CRATEDB', 'MATERIALIZE', 'ALLOYDB', 'CLOUDSQLPG', 'FUJITSUPG'])
    const MSSQL_SET = new Set(['SQLSERVER', 'HANA'])

    let sql: string
    if (dbType && MYSQL_SET.has(dbType)) {
      sql = `CREATE DATABASE \`${name}\``
      if (options.charset)
        sql += ` CHARACTER SET ${options.charset}`
      if (options.collation)
        sql += ` COLLATE ${options.collation}`
    }
    else if (dbType && PG_SET.has(dbType)) {
      sql = `CREATE DATABASE "${name}"`
      if (options.encoding)
        sql += ` ENCODING '${options.encoding}'`
      if (options.locale)
        sql += ` LC_COLLATE '${options.locale}'`
    }
    else if (dbType && MSSQL_SET.has(dbType)) {
      sql = `CREATE DATABASE [${name}]`
    }
    else {
      sql = `CREATE DATABASE ${name}`
    }

    const result: ApiResponse<unknown> = await invoke('execute_query', {
      connectionId: connId,
      sql,
    })
    if (!isApiSuccess(result))
      throw new Error(result.error?.message || 'Unknown error')
    toast.success(t('sidebar.databases.actions.createObject.created', { type: t('sidebar.databases.actions.createObject.types.database'), name }))
    createDatabaseDialogOpen.value = false
    // Refresh database list without switching to Mode A
    await databaseStore.fetchDatabases(connId)
  }
  catch (err) {
    toast.error(t('sidebar.databases.actions.createObject.error', { error: String(err) }))
  }
  finally {
    isDatabaseActionExecuting.value = false
  }
}

async function handleCreateSchema(name: string) {
  const connId = getActiveConnectionId()
  if (!connId)
    return
  isDatabaseActionExecuting.value = true
  try {
    const result: ApiResponse<unknown> = await invoke('execute_query', {
      connectionId: connId,
      sql: `CREATE SCHEMA ${name}`,
    })
    if (!isApiSuccess(result))
      throw new Error(result.error?.message || 'Unknown error')
    toast.success(t('sidebar.databases.actions.createObject.created', { type: t('sidebar.databases.actions.createObject.types.schema'), name }))
    createSchemaDialogOpen.value = false
    schemaTreeRef.value?.refresh()
  }
  catch (err) {
    toast.error(t('sidebar.databases.actions.createObject.error', { error: String(err) }))
  }
  finally {
    isDatabaseActionExecuting.value = false
  }
}

async function handleDropDatabase() {
  const connId = getActiveConnectionId()
  const dbName = selectedDatabase.value
  if (!connId || !dbName)
    return
  isDatabaseActionExecuting.value = true
  try {
    const dbType = activeConnection.value?.type
    const MYSQL_SET = new Set(['MYSQL', 'MARIADB', 'TIDB', 'OCEANBASE', 'TDSQL', 'POLARDB', 'DORIS', 'SELECTDB', 'STARROCKS', 'DATABEND', 'GOLDENDB', 'MANTICORESEARCH', 'SINGLESTOREMEMSQL', 'CLOUDSQLMYSQL'])
    const PG_SET = new Set(['POSTGRESQL', 'COCKROACHDB', 'REDSHIFT', 'YUGABYTEDB', 'TIMESCALEDB', 'KINGBASEES', 'GAUSSDB', 'HIGHGO', 'UXDB', 'OPENGAUSS', 'GBASE8C', 'QUESTDB', 'VASTBASE', 'YASHANDB', 'GREENPLUM', 'ENTERPRISEDB', 'CRATEDB', 'MATERIALIZE', 'ALLOYDB', 'CLOUDSQLPG', 'FUJITSUPG'])
    const MSSQL_SET = new Set(['SQLSERVER', 'HANA'])

    let sql: string
    if (dbType && MYSQL_SET.has(dbType))
      sql = `DROP DATABASE \`${dbName}\``
    else if (dbType && PG_SET.has(dbType))
      sql = `DROP DATABASE "${dbName}"`
    else if (dbType && MSSQL_SET.has(dbType))
      sql = `DROP DATABASE [${dbName}]`
    else
      sql = `DROP DATABASE ${dbName}`

    const result: ApiResponse<unknown> = await invoke('execute_query', {
      connectionId: connId,
      sql,
    })
    if (!isApiSuccess(result))
      throw new Error(result.error?.message || 'Unknown error')
    toast.success(t('sidebar.databases.actions.dropDatabase.success', { name: dbName }))
    dropDatabaseDialogOpen.value = false
    selectedDatabase.value = ''
    // Refresh database list after dropping
    await databaseStore.fetchDatabases(connId)
  }
  catch (err) {
    toast.error(t('sidebar.databases.actions.dropDatabase.error', { error: String(err) }))
  }
  finally {
    isDatabaseActionExecuting.value = false
  }
}

function handleCreateTable(tableName: string, sql: string) {
  const connId = getActiveConnectionId()
  if (!connId)
    return

  const tab = tabStore.createTab(connId, selectedDatabase.value, selectedSchema.value)
  tabStore.updateTabName(tab.id, `${tableName}.sql`)
  tabStore.updateTabContent(tab.id, sql)
  createTableDialogOpen.value = false
  toast.success(t('sidebar.databases.actions.createTable.opened'))
}

function openTemplateTab(kind: string) {
  const connId = getActiveConnectionId()
  if (!connId)
    return

  const typeLabel = kind.replace('new', '').toUpperCase()
  const tab = tabStore.createTab(connId, selectedDatabase.value, selectedSchema.value)
  tabStore.updateTabName(tab.id, `new_${kind.replace('new', '').toLowerCase()}.sql`)
  tabStore.updateTabContent(tab.id, getTemplateSQL(kind))
  toast.info(t('sidebar.databases.actions.templateOpened', { type: typeLabel }))
}

function getTemplateSQL(kind: string): string {
  const qualified = selectedSchema.value ? `${selectedSchema.value}.` : ''

  switch (kind) {
    case 'newView':
      return [
        `CREATE VIEW ${qualified}your_view_name AS`,
        'SELECT',
        '  *',
        `FROM ${qualified}your_table`,
        'WHERE',
        '  your_condition;',
        '',
        `-- Note: Replace placeholders above before executing.`,
      ].join('\n')

    case 'newFunction':
      return [
        `CREATE OR REPLACE FUNCTION ${qualified}your_function_name()`,
        'RETURNS void',
        'LANGUAGE plpgsql',
        'AS $$',
        'BEGIN',
        '  -- TODO: Implement function body',
        '  -- E.g.: INSERT INTO audit_log (action) VALUES (\'your_action\');',
        'END;',
        '$$;',
        '',
        `-- Note: Replace placeholders above before executing.`,
      ].join('\n')

    case 'newProcedure':
      return [
        `CREATE OR REPLACE PROCEDURE ${qualified}your_procedure_name()`,
        'LANGUAGE plpgsql',
        'AS $$',
        'BEGIN',
        '  -- TODO: Implement procedure body',
        '  -- E.g.: UPDATE your_table SET updated_at = NOW() WHERE id = 1;',
        'END;',
        '$$;',
        '',
        `-- Note: Replace placeholders above before executing.`,
      ].join('\n')

    default:
      return '-- Select SQL to execute'
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
        <!-- Sidebar -->
        <div
          class="border-r bg-background flex flex-col"
          :style="{ width: `${sidebarWidth}px` }"
        >
          <!-- Connection selector with status indicator -->
          <ConnectionSelector v-model="selectedConnectionId" />

          <!-- Database selector row with refresh + action menu -->
          <DatabaseSelectorRow
            v-model="selectedDatabase"
            :connection-id="getActiveConnectionId()"
            :loading="databaseStore.loading"
            @refresh="handleDatabaseRefresh"
            @action="handleDatabaseAction"
          />

          <!-- Tree + Saved Queries split -->
          <SidebarSplitView
            class="flex-1"
            :bottom-open="!savedQueriesCollapsed"
          >
            <template #top>
              <SchemaTree
                ref="schemaTreeRef"
                v-model:selected-database="selectedDatabase"
                v-model:selected-schema="selectedSchema"
                :connection-id="getActiveConnectionId()"
                @select-table="handleSelectTable"
                @create-script="handleCreateScript"
                @select-top-n="handleSelectTopN"
                @view-structure="handleViewStructure"
                @export-data="handleExportData"
                @show-er-diagram="handleShowErDiagram"
                @drop-table="handleDropTable"
                @truncate-table="handleTruncateTable"
                @open-listing-tab="handleOpenListingTab"
                @open-ddl-tab="handleOpenDdlTab"
              />
            </template>
            <template #bottom>
              <SavedQueriesPanel
                ref="savedQueriesRef"
                v-model:collapsed="savedQueriesCollapsed"
                @open="handleOpenSavedQuery"
                @new-query="handleNewTab"
              />
            </template>
          </SidebarSplitView>
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

          <!-- Data Table View — uses tab's own connectionId so it works regardless of sidebar selection -->
          <DataTableView
            v-if="activeTab?.tableView && !activeTab.orphanFromConnectionId && isTableViewConnectionValid"
            :key="`${activeTab.connectionId}-${activeTab.id}-${activeTab.tableView.database}-${activeTab.tableView.tableName}`"
            :connection-id="activeTab.connectionId || ''"
            :database="activeTab.tableView.database"
            :schema="activeTab.tableView.schema"
            :table-name="activeTab.tableView.tableName"
            class="flex-1"
          />

          <!-- Invalid table-view state (no active connection) -->
          <div
            v-else-if="activeTab?.tableView && !isTableViewConnectionValid"
            class="flex flex-1 items-center justify-center"
          >
            <div class="text-muted-foreground text-center">
              <span class="i-carbon-warning mx-auto mb-2 h-8 w-8 block" />
              <p class="text-sm">
                {{ t('pages.queries.status.connectionLost') }}
              </p>
              <p class="text-xs mt-1">
                {{ t('pages.queries.status.reconnecting') }}
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
            <!-- Toolbar (DBX-style: compact h-9, colored icons, no constant bg) -->
            <div class="h-9 shrink-0 border-b border-border bg-background/80 px-2 flex items-center gap-0.5 text-xs text-muted-foreground">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      variant="ghost"
                      size="icon"
                      class="h-7 w-7 text-emerald-600 hover:bg-emerald-500/10 dark:text-emerald-400 dark:hover:bg-emerald-500/20 disabled:opacity-40"
                      :disabled="!activeTab || activeTab.orphanFromConnectionId"
                      @click="executeQuery"
                    >
                      <span class="i-lucide-play h-3.5 w-3.5 shrink-0" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.queries.editor.execute') }} ({{ t('pages.queries.shortcuts.execute', { key: modifierKey }) }})</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>

              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      variant="ghost"
                      size="icon"
                      class="h-7 w-7 text-violet-600 hover:bg-violet-500/10 dark:text-violet-400 dark:hover:bg-violet-500/20 disabled:opacity-40"
                      :class="{ '!text-emerald-600 !bg-emerald-500/10 !dark:bg-emerald-500/20': explainAnalyzeMode }"
                      :disabled="!activeTab || activeTab.orphanFromConnectionId || activeTab.isExplaining"
                      @click="handleExplainQuery(explainAnalyzeMode)"
                    >
                      <span class="i-lucide-line-chart h-3.5 w-3.5 shrink-0" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.queries.editor.explain') }} ({{ t('pages.queries.shortcuts.explain', { key: modifierKey }) }})</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      variant="ghost"
                      size="icon"
                      class="h-7 w-7 text-muted-foreground hover:text-violet-600 hover:bg-violet-500/10 dark:hover:text-violet-400 dark:hover:bg-violet-500/20 disabled:opacity-40"
                      :class="explainAnalyzeMode ? '!text-violet-600 !bg-violet-100 dark:!text-violet-300 dark:!bg-violet-900/30' : ''"
                      :disabled="!activeTab || activeTab.orphanFromConnectionId || activeTab.isExplaining"
                      @click="toggleExplainMode"
                    >
                      <span class="i-lucide-activity h-3.5 w-3.5 shrink-0" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.queries.explain.analyzeToggle', { mode: explainAnalyzeMode ? 'ON' : 'OFF' }) }}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>

              <div class="w-px h-4 bg-border mx-0.5" />

              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      variant="ghost"
                      size="icon"
                      class="h-7 w-7 text-amber-600 hover:bg-amber-500/10 dark:text-amber-400 dark:hover:bg-amber-500/20 disabled:opacity-40"
                      :disabled="!activeTab || activeTab.orphanFromConnectionId"
                      @click="handleToolbarFormat"
                    >
                      <span class="i-lucide-align-left h-3.5 w-3.5 shrink-0" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.queries.editor.format') }} ({{ t('pages.queries.shortcuts.format') }})</p>
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
                      size="icon"
                      class="h-7 w-7 text-sky-600 hover:bg-sky-500/10 dark:text-sky-400 dark:hover:bg-sky-500/20 disabled:opacity-40"
                      :disabled="!activeTab || !activeTab.content.trim()"
                      @click="handleDownloadQuery"
                    >
                      <span class="i-lucide-download h-3.5 w-3.5 shrink-0" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.queries.shortcuts.saveAs', { key: modifierKey }) }}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>

              <!-- Status info -->
              <div v-if="activeTab" class="text-xs text-muted-foreground flex gap-2 items-center">
                <span v-if="activeConnection" class="font-medium">{{ activeConnection.name }}</span>
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
                  <span class="i-carbon-document text-muted-foreground/50 mx-auto mb-4 h-12 w-12 block" />
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
            {{ t('pages.queries.status.rows') }}: {{ activeTab.results.rows.length }}
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

    <!-- Create Database dialog -->
    <CreateDatabaseDialog
      :open="createDatabaseDialogOpen"
      :database-type="activeConnection?.type"
      @update:open="(v: boolean) => createDatabaseDialogOpen = v"
      @confirm="handleCreateDatabase"
    />

    <!-- Create Schema dialog -->
    <CreateObjectDialog
      :open="createSchemaDialogOpen"
      object-type="schema"
      @update:open="(v: boolean) => createSchemaDialogOpen = v"
      @confirm="handleCreateSchema"
    />

    <!-- Drop Database dialog -->
    <DropDatabaseDialog
      :open="dropDatabaseDialogOpen"
      :database-name="selectedDatabase"
      @update:open="(v: boolean) => dropDatabaseDialogOpen = v"
      @confirm="handleDropDatabase"
    />

    <!-- Create Table dialog -->
    <CreateTableDialog
      :open="createTableDialogOpen"
      :database="selectedDatabase"
      :schema="selectedSchema || null"
      :database-type="activeConnection?.type"
      @update:open="(v: boolean) => createTableDialogOpen = v"
      @create="handleCreateTable"
    />
  </AppLayout>
</template>
