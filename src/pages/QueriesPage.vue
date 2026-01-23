<script setup lang="ts">
import type { TableInfo } from '@/store/databaseStore'
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
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
import { useConnectionStore, useDatabaseStore, useTabStore } from '@/store'

const { t } = useI18n()
const route = useRoute()
const connectionStore = useConnectionStore()
const databaseStore = useDatabaseStore()
const tabStore = useTabStore()

const editorRef = ref<InstanceType<typeof SQLEditor> | null>(null)
const showResultPanel = ref(false)
const selectedDatabase = ref<string>('')
const sidebarWidth = ref(250)
const isResizingSidebar = ref(false)

// Get connection from route or store
const connectionId = computed(() => {
  return (route.query.connectionId as string) || connectionStore.activeConnectionId
})

const activeConnection = computed(() => {
  if (connectionId.value) {
    return connectionStore.getConnectionById(connectionId.value)
  }
  return null
})

const databases = computed(() => {
  if (!connectionId.value)
    return []
  return databaseStore.metadata[connectionId.value]?.databases || []
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

// Initialize on mount
onMounted(async () => {
  // Set active connection from route if provided
  if (route.query.connectionId) {
    connectionStore.setActiveConnection(route.query.connectionId as string)
  }

  // Create initial tab if none exists
  if (connectionId.value && tabStore.tabs.length === 0) {
    tabStore.createTab(connectionId.value, selectedDatabase.value || undefined)
  }

  // Fetch databases for active connection
  if (connectionId.value) {
    await databaseStore.fetchDatabases(connectionId.value)

    // Set default database if connection has one
    if (activeConnection.value?.database) {
      selectedDatabase.value = activeConnection.value.database
    }
  }
})

// Watch for database changes
watch(selectedDatabase, (newDb) => {
  if (newDb) {
    databaseStore.selectDatabase(newDb)
  }
})

// Execute query
async function executeQuery() {
  if (!activeTab.value || !activeTab.value.content.trim()) {
    return
  }

  showResultPanel.value = true
  await tabStore.executeQuery(activeTab.value.id)
}

// Handle keyboard shortcuts
function handleKeyDown(event: KeyboardEvent) {
  // Ctrl+Enter - Execute query
  if (event.ctrlKey && event.key === 'Enter') {
    event.preventDefault()
    executeQuery()
  }

  // F6 - Explain query
  if (event.key === 'F6') {
    event.preventDefault()
    handleExplainQuery()
  }

  // Ctrl+S - Save (mark as saved)
  if (event.ctrlKey && event.key === 's') {
    event.preventDefault()
    if (activeTab.value) {
      tabStore.markTabSaved(activeTab.value.id)
    }
  }

  // Ctrl+Tab - Switch tabs
  if (event.ctrlKey && event.key === 'Tab') {
    event.preventDefault()
    switchToNextTab()
  }
}

function switchToNextTab() {
  if (tabStore.tabs.length <= 1)
    return

  const currentIndex = tabStore.tabs.findIndex(t => t.id === tabStore.activeTabId)
  const nextIndex = (currentIndex + 1) % tabStore.tabs.length
  tabStore.setActiveTab(tabStore.tabs[nextIndex].id)
}

async function handleExplainQuery() {
  // TODO: Implement explain query
}

// Tab management
function handleNewTab() {
  if (connectionId.value) {
    tabStore.createTab(connectionId.value, selectedDatabase.value || undefined)
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

// Database browser handlers
function handleCreateScript(table: TableInfo, database: string, schema?: string) {
  const schemaPrefix = schema ? `"${schema}".` : ''
  const script = `-- CREATE TABLE script for ${table.name}
-- TODO: Generate actual CREATE TABLE from server
CREATE TABLE ${schemaPrefix}"${table.name}" (
  -- columns will be generated here
);`

  if (connectionId.value) {
    const tab = tabStore.createTab(connectionId.value, database)
    tabStore.updateTabContent(tab.id, script)
    tabStore.updateTabName(tab.id, `CREATE_${table.name}`)
  }
}

function handleSelectTopN(table: TableInfo, database: string, schema?: string, n = 100) {
  const schemaPrefix = schema ? `"${schema}".` : ''
  const query = `SELECT * FROM ${schemaPrefix}"${table.name}" LIMIT ${n};`

  if (connectionId.value) {
    const tab = tabStore.createTab(connectionId.value, database)
    tabStore.updateTabContent(tab.id, query)
    tabStore.updateTabName(tab.id, `SELECT_${table.name}`)

    // Auto-execute the query
    tabStore.executeQuery(tab.id)
    showResultPanel.value = true
  }
}

function handleViewStructure(table: TableInfo, database: string, schema?: string) {
  // Create a query to show table structure
  const query = `-- Table structure for ${table.name}
-- Database: ${database}${schema ? `\n-- Schema: ${schema}` : ''}

-- View columns
SELECT column_name, data_type, is_nullable, column_default
FROM information_schema.columns
WHERE table_name = '${table.name}'${schema ? ` AND table_schema = '${schema}'` : ''};`

  if (connectionId.value) {
    const tab = tabStore.createTab(connectionId.value, database)
    tabStore.updateTabContent(tab.id, query)
    tabStore.updateTabName(tab.id, `STRUCTURE_${table.name}`)
  }
}

function handleExportData(_table: TableInfo, _database: string, _schema?: string) {
  // TODO: Implement export data functionality
}

// Sidebar resize
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
    <div class="flex flex-col h-full" @keydown="handleKeyDown">
      <!-- Main content area with sidebar -->
      <div class="flex flex-1 overflow-hidden">
        <!-- Database Browser Sidebar -->
        <div
          class="border-r bg-background flex flex-col"
          :style="{ width: `${sidebarWidth}px` }"
        >
          <!-- Database selector -->
          <div class="p-2 border-b">
            <Select v-model="selectedDatabase">
              <SelectTrigger class="text-xs h-8">
                <SelectValue :placeholder="t('pages.queries.selectDatabase')" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="db in databases"
                  :key="db"
                  :value="db"
                >
                  {{ db }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <!-- Database Browser -->
          <DatabaseBrowser
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
              <span v-if="selectedDatabase">/ {{ selectedDatabase }}</span>
            </div>
          </div>

          <!-- Editor -->
          <div class="flex-1 overflow-hidden">
            <SQLEditor
              v-if="activeTab"
              ref="editorRef"
              v-model="editorContent"
              height="100%"
              dialect="sql"
              @execute="executeQuery"
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
          <span v-if="activeConnection">
            {{ t('pages.queries.status.connection') }}: {{ activeConnection.name }}
          </span>
          <span v-if="selectedDatabase">
            {{ t('pages.queries.status.database') }}: {{ selectedDatabase }}
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
