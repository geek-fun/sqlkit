<script setup lang="ts">
import type { CursorPosition, Selection } from '@/common/sqlParser'
import type { TableInfo } from '@/store/databaseStore'
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { extractStatementAtCursor } from '@/common/sqlParser'
import { DatabaseBrowser, QueryResultPanel, QueryTabs } from '@/components/database-browser'
import AppLayout from '@/components/layout/AppLayout.vue'
import SQLEditor from '@/components/SQLEditor.vue'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { saveQueryFile } from '@/datasources'
import { ConnectionStatus, useConnectionStore, useDatabaseStore, useTabStore } from '@/store'

const { t } = useI18n()
const route = useRoute()
const connectionStore = useConnectionStore()
const databaseStore = useDatabaseStore()
const tabStore = useTabStore()

const showResultPanel = ref(false)
const sidebarWidth = ref(250)
const isResizingSidebar = ref(false)
const selectedDatabase = ref<string>('')

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

// Initialize on mount — restore previously selected connection & database from store
onMounted(async () => {
  // Route query param takes priority (e.g. deep-link from ConnectionsPage)
  const routeConnId = route.query.connectionId as string | undefined
  const connId = routeConnId || connectionStore.activeConnectionId

  if (connId) {
    selectedConnectionId.value = connId

    // If already connected, just restore UI state — no need to reconnect
    const isConnected = connectionStore.getConnectionStatus(connId) === ConnectionStatus.CONNECTED
    if (isConnected) {
      const currentDb = connectionStore.getCurrentDatabase(connId)
      if (currentDb) {
        selectedDatabase.value = currentDb
      }

      if (tabStore.tabs.length === 0) {
        tabStore.createTab(connId, currentDb || undefined)
      }

      await databaseStore.fetchDatabases(connId)
    }
    // If not connected yet (e.g. first visit via route param), the watch will connect
  }
})

// Watch for connection changes — connect only when not already connected
watch(selectedConnectionId, async (newConnId, oldConnId) => {
  if (!newConnId || newConnId === oldConnId) {
    return
  }

  const alreadyConnected = connectionStore.getConnectionStatus(newConnId) === ConnectionStatus.CONNECTED

  if (alreadyConnected) {
    // Already connected — just restore UI state and make it active
    connectionStore.setActiveConnection(newConnId)
    const currentDb = connectionStore.getCurrentDatabase(newConnId)
    if (currentDb) {
      selectedDatabase.value = currentDb
    }
    if (tabStore.tabs.length === 0) {
      tabStore.createTab(newConnId, currentDb || undefined)
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

    if (tabStore.tabs.length === 0) {
      tabStore.createTab(newConnId, connectionStore.getCurrentDatabase(newConnId) || undefined)
    }
  }
  catch (error) {
    console.error('Failed to connect:', error)
  }
})

// Persist the selected database back to the store so it survives navigation
watch(selectedDatabase, (db) => {
  const connId = selectedConnectionId.value || connectionStore.activeConnectionId
  if (connId && db) {
    connectionStore.setCurrentDatabase(connId, db)
  }
})

interface ExecuteQueryDetails { query: string, cursorPosition?: CursorPosition, selection?: Selection }

async function executeQuery(details?: ExecuteQueryDetails) {
  if (!activeTab.value) {
    return
  }

  const tabContent = activeTab.value.content || ''
  if (!tabContent.trim()) {
    return
  }

  const sqlToExecute = details && typeof details === 'object' && 'query' in details
    ? extractStatementAtCursor(details.query || '', details.cursorPosition, details.selection)
    : tabContent

  if (!sqlToExecute?.trim()) {
    return
  }

  showResultPanel.value = true
  await tabStore.executeQuery(activeTab.value.id, sqlToExecute)
}

async function handleExplainQuery() {
  // TODO: Implement explain query
}

const getConnectionId = () => selectedConnectionId.value || connectionStore.activeConnectionId

function handleNewTab() {
  const connId = getConnectionId()
  if (connId) {
    const connection = connectionStore.getConnectionById(connId)
    tabStore.createTab(connId, connection?.database || undefined)
  }
}

function handleTabSelect(tabId: string) {
  tabStore.setActiveTab(tabId)
}

function handleTabClose(tabId: string) {
  tabStore.closeTab(tabId)
}

function handleTabCloseForce(tabId: string) {
  tabStore.closeTab(tabId)
}

function handleCreateScript(table: TableInfo, database: string, schema?: string) {
  const schemaPrefix = schema ? `"${schema}".` : ''
  const script = `-- CREATE TABLE script for ${table.name}
-- TODO: Generate actual CREATE TABLE from server
CREATE TABLE ${schemaPrefix}"${table.name}" (
  -- columns will be generated here
);`

  const connId = getConnectionId()
  if (connId) {
    const tab = tabStore.createTab(connId, database)
    tabStore.updateTabContent(tab.id, script)
    tabStore.updateTabName(tab.id, `CREATE_${table.name}.sql`)
  }
}

function handleSelectTopN(table: TableInfo, database: string, schema?: string, n = 100) {
  const schemaPrefix = schema ? `"${schema}".` : ''
  const query = `SELECT * FROM ${schemaPrefix}"${table.name}" LIMIT ${n};`

  const connId = getConnectionId()
  if (connId) {
    const tab = tabStore.createTab(connId, database)
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

  const connId = getConnectionId()
  if (connId) {
    const tab = tabStore.createTab(connId, database)
    tabStore.updateTabContent(tab.id, query)
    tabStore.updateTabName(tab.id, `STRUCTURE_${table.name}`)
  }
}

function handleExportData(_table: TableInfo, _database: string, _schema?: string) {
  // TODO: Implement export data functionality
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

async function handleSaveQuery() {
  if (!activeTab.value || !activeTab.value.content.trim()) {
    return
  }

  try {
    const result = await saveQueryFile(
      activeTab.value.content,
      activeTab.value.filePath,
      activeTab.value.filePath ? undefined : `${activeTab.value.name}.sql`,
    )

    if (result.success && result.file_path) {
      tabStore.markTabSaved(activeTab.value.id, result.file_path)
    }
  }
  catch (error) {
    console.error('Failed to save query:', error)
  }
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
              <SelectTrigger class="text-xs h-8">
                <SelectValue :placeholder="t('pages.queries.selectConnection')" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="conn in availableConnections"
                  :key="conn.id"
                  :value="conn.id!"
                >
                  <div class="flex gap-2 items-center">
                    <span class="font-medium">{{ conn.name }}</span>
                    <span class="text-xs text-muted-foreground">({{ conn.host }})</span>
                  </div>
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <!-- Database Browser -->
          <DatabaseBrowser
            v-model:selected-database="selectedDatabase"
            :connection-id="selectedConnectionId"
            class="flex-1"
            @create-script="handleCreateScript"
            @select-top-n="handleSelectTopN"
            @view-structure="handleViewStructure"
            @export-data="handleExportData"
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
            :tabs="tabStore.tabs"
            :active-tab-id="tabStore.activeTabId"
            @select="handleTabSelect"
            @close="handleTabClose"
            @close-force="handleTabCloseForce"
            @new="handleNewTab"
          />

          <!-- Toolbar -->
          <div class="px-2 py-1 border-b bg-muted/30 flex gap-2 items-center">
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger as-child>
                  <Button
                    variant="ghost"
                    size="sm"
                    class="gap-1 h-7"
                    :disabled="!activeTab"
                    @click="executeQuery"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <polygon points="6 3 20 12 6 21 6 3" />
                    </svg>
                    {{ t('pages.queries.editor.execute') }}
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <p>{{ t('pages.queries.shortcuts.execute') }}</p>
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
                    :disabled="!activeTab"
                    @click="handleExplainQuery"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M12 20h9" />
                      <path d="M16.376 3.622a1 1 0 0 1 3.002 3.002L7.368 18.635a2 2 0 0 1-.855.506l-2.872.838a.5.5 0 0 1-.62-.62l.838-2.872a2 2 0 0 1 .506-.854z" />
                    </svg>
                    {{ t('pages.queries.editor.explain') }}
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <p>{{ t('pages.queries.shortcuts.explain') }}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>

            <div class="flex-1" />

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
              @execute="(details) => executeQuery(details)"
              @save="handleSaveQuery"
            />
            <div v-else class="text-muted-foreground flex h-full items-center justify-center">
              <div class="text-center">
                <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" class="mx-auto mb-4 opacity-50">
                  <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
                  <polyline points="14 2 14 8 20 8" />
                </svg>
                <p>{{ t('pages.queries.noTab') }}</p>
                <Button variant="outline" size="sm" class="mt-2" @click="handleNewTab">
                  {{ t('pages.queries.newTab') }}
                </Button>
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
            @close="closeResultPanel"
          />
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
  </AppLayout>
</template>
