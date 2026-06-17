<script setup lang="ts">
import type { ServerConnection } from '@/store/connectionStore'
import { onClickOutside } from '@vueuse/core'
import { storeToRefs } from 'pinia'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import ChatPanel from '@/components/chat-panel.vue'
import AppLayout from '@/components/layout/AppLayout.vue'
import { disposeAgentRuntime, initAgentRuntime } from '@/composables/agentRuntime'
import { useDatabaseIcon } from '@/composables/useDatabaseIcon'
import { useDataStudioChatAgent } from '@/composables/useDataStudioChatAgent'
import { useAppStore } from '@/store'
import { DatabaseType, useConnectionStore } from '@/store/connectionStore'
import { useDataStudioStore } from '@/store/dataStudioStore'
import ModifySourceModal from '@/views/data-studio/components/modify-source-modal.vue'
import SessionHistoryPanel from '@/views/data-studio/components/session-history-panel.vue'

const { t } = useI18n()
const connectionStore = useConnectionStore()
const { connections } = storeToRefs(connectionStore)
const dataStudioStore = useDataStudioStore()
const { attachedSources, activeSession } = storeToRefs(dataStudioStore)

const { getDatabaseIcon } = useDatabaseIcon()

const AGENT_SUPPORTED_TYPES = new Set([
  DatabaseType.POSTGRESQL,
  DatabaseType.MYSQL,
  DatabaseType.SQLITE,
  DatabaseType.SQLSERVER,
])

const {
  isLoading,
  error,
  messages,
  sendMessage,
  handleConfirmation: rawHandleConfirmation,
  cancelSession,
  clearChat,
  activeSessionSources,
  lastSettings,
  initContextSettings,
  stopReason,
  stopMessage,
  progress,
} = useDataStudioChatAgent()

function handleConfirmation(msgId: string, event: { toolCallId: string, action: 'allow_once' | 'allow_always' | 'deny' | 'deny_always' | 'cancel' }) {
  rawHandleConfirmation(msgId, event.toolCallId, event.action)
}

const historyPanelOpen = ref(false)
const permissionMenuOpen = ref(false)
const showModifyModal = ref(false)
const selectedSourceIdx = ref<number>(-1)
const addSourceOpen = ref(false)
const addSourceQuery = ref('')
const addSourceSelectedId = ref('')
const addSourceMode = ref<'Ask' | 'Inherit'>('Inherit')
const addSourcePickerRef = ref<HTMLElement | null>(null)

function resetAddSourceState() {
  addSourceOpen.value = false
  addSourceQuery.value = ''
  addSourceSelectedId.value = ''
  addSourceMode.value = 'Inherit'
}

onClickOutside(addSourcePickerRef, resetAddSourceState)

const hasMessages = computed(() => messages.value.length > 0)

const emptyHint = computed(() =>
  activeSessionSources.value.length > 0
    ? t('dataStudio.agent.emptyState')
    : t('dataStudio.agent.noSource'),
)

const sessionPermissionsMode = computed(() => activeSession.value?.permissionsMode ?? 'Ask')

const availableAddConnections = computed(() => {
  const sessionConnIds = new Set(
    activeSessionSources.value.flatMap((s) => {
      const attached = attachedSources.value.find(a => a.sourceId === s.sourceId)
      return attached?.kind === 'database'
        ? [(attached as { connectionId: number }).connectionId]
        : []
    }),
  )
  return connections.value.filter(
    conn =>
      !sessionConnIds.has(Number(conn.id))
      && AGENT_SUPPORTED_TYPES.has(conn.type as DatabaseType),
  )
})

const filteredAddConnections = computed(() => {
  const q = addSourceQuery.value.toLowerCase().trim()
  return q
    ? availableAddConnections.value.filter(
        c => c.name.toLowerCase().includes(q) || c.type.toLowerCase().includes(q),
      )
    : availableAddConnections.value
})

function getConnectionMeta(conn: ServerConnection): string {
  const label = conn.type
  if (conn.type === DatabaseType.SQLITE)
    return `${label} • ${conn.host}`
  return `${label} • ${conn.host}:${conn.port}`
}

function getConnectionIcon(type: string): string {
  return getDatabaseIcon(type as DatabaseType)
}

function selectAddConnection(conn: ServerConnection) {
  addSourceSelectedId.value = String(conn.id)
}

async function confirmAddSource() {
  if (!addSourceSelectedId.value)
    return
  const conn = connections.value.find(c => String(c.id) === addSourceSelectedId.value)
  if (!conn || conn.id === undefined)
    return

  try {
    const newSource = await dataStudioStore.addDatabaseSourceFromConnection({
      connectionId: Number(conn.id),
      name: conn.name,
      databaseType: conn.type as 'POSTGRESQL' | 'MYSQL' | 'SQLSERVER' | 'SQLITE',
      permissions: { read: true, create: false, update: false, delete: false },
    })

    if (!dataStudioStore.activeSession) {
      await dataStudioStore.getOrCreateSession()
    }
    dataStudioStore.attachSourceToActiveSession(newSource.sourceId)
    if (addSourceMode.value === 'Ask' && dataStudioStore.activeSession) {
      dataStudioStore.updateSessionSourceMode(
        newSource.sourceId,
        'custom',
      )
    }
    resetAddSourceState()
  }
  catch (err) {
    console.error('Failed to add data source:', err)
  }
}

function setAutoMode(auto: boolean) {
  permissionMenuOpen.value = false
  dataStudioStore.setSessionPermissionsMode(auto ? 'Auto' : 'Ask')
}

async function switchSession(sessionId: string) {
  dataStudioStore.setActiveSession(sessionId)
  await dataStudioStore.reloadSessionMessages(sessionId)
  await dataStudioStore.loadConfirmationRulesFromDb(sessionId)
  historyPanelOpen.value = false
}

async function deleteSession(sessionId: string) {
  await dataStudioStore.removeSession(sessionId)
}

function startNewSession() {
  dataStudioStore.setActiveSession('')
  historyPanelOpen.value = false
}

function openModifyModal(index: number) {
  selectedSourceIdx.value = index
  showModifyModal.value = true
}

async function onModelChange(modelId: string) {
  const sess = dataStudioStore.activeSession
  if (sess?.id) {
    dataStudioStore.setSessionModelId(sess.id, modelId)
  }
}

onMounted(async () => {
  await initAgentRuntime()
  await dataStudioStore.loadSessions()
  await connectionStore.fetchConnections()
  await dataStudioStore.loadAttachedSourcesFromDb()
  if (dataStudioStore.activeSessionId) {
    await dataStudioStore.loadConfirmationRulesFromDb(dataStudioStore.activeSessionId)
  }
  await initContextSettings()
})

onBeforeUnmount(() => {
  disposeAgentRuntime()
})

const appStore = useAppStore()
const { llmSettings } = storeToRefs(appStore)

function syncAllProviderModels() {
  llmSettings.value.providers
    .filter(p => p.enabled)
    .forEach(p => appStore.syncProviderModels(p.id).catch(() => {}))
}
</script>

<template>
  <AppLayout :hide-ai-button="true">
    <div class="bg-background flex h-full">
      <!-- History Panel (left sidebar) -->
      <div v-if="historyPanelOpen" class="border-r border-border shrink-0 w-72 overflow-y-auto">
        <SessionHistoryPanel
          @select="switchSession"
          @delete="deleteSession"
          @new-session="startNewSession"
          @close="historyPanelOpen = false"
        />
      </div>

      <!-- Main Conversation Area -->
      <div class="flex flex-1 flex-col min-w-0">
        <!-- Header -->
        <div class="px-4 py-2 border-b border-border flex shrink-0 items-center justify-between">
          <div class="flex gap-2 items-center">
            <button
              class="text-muted-foreground rounded inline-flex h-8 w-8 cursor-pointer transition-colors items-center justify-center hover:text-foreground hover:bg-muted"
              :class="{ 'bg-muted text-foreground': historyPanelOpen }"
              :title="t('dataStudio.history.title')"
              @click="historyPanelOpen = !historyPanelOpen"
            >
              <span class="i-carbon-time h-3.5 w-3.5" />
            </button>
            <span class="text-[11px] tracking-wider font-bold px-3 py-0.5 border border-border rounded-full">
              {{ t('dataStudio.title') }}
            </span>
          </div>
          <div class="flex gap-2 items-center">
            <button
              v-if="isLoading"
              class="text-destructive rounded inline-flex h-8 w-8 cursor-pointer transition-colors items-center justify-center hover:bg-destructive/10"
              :title="t('dataStudio.agent.stop')"
              @click="cancelSession"
            >
              <span class="i-carbon-stop-filled h-3.5 w-3.5" />
            </button>
            <button
              class="text-muted-foreground rounded inline-flex h-8 w-8 cursor-pointer transition-colors items-center justify-center hover:text-foreground hover:bg-muted"
              :title="t('dataStudio.history.newSession')"
              @click="startNewSession"
            >
              <span class="i-carbon-add h-3.5 w-3.5" />
            </button>
            <button
              v-if="hasMessages"
              class="text-muted-foreground rounded inline-flex h-8 w-8 cursor-pointer transition-colors items-center justify-center hover:text-foreground hover:bg-muted"
              :title="t('dataStudio.agent.clearChat')"
              @click="clearChat"
            >
              <span class="i-carbon-trash-can h-3.5 w-3.5" />
            </button>
          </div>
        </div>

        <!-- Chat Panel -->
        <div class="flex-1 min-h-0">
          <ChatPanel
            :messages="messages"
            :is-loading="isLoading"
            :error="error"
            :empty-hint="emptyHint"
            :input-placeholder="t('dataStudio.inputPlaceholder')"
            :session-id="activeSession?.id ?? null"
            :context-settings="lastSettings"
            :stop-reason="stopReason"
            :stop-message="stopMessage"
            :progress="progress"
            feature="dataStudio"
            compact
            @send="sendMessage"
            @stop-loop="cancelSession"
            @confirm-tool-call="handleConfirmation"
            @model-change="onModelChange"
            @model-picker-open="syncAllProviderModels"
          >
            <template #toolbar-left>
              <!-- Connected source chips -->
              <button
                v-for="(source, idx) in activeSessionSources"
                :key="source.sourceId"
                class="source-chip"
                :title="t('dataStudio.modifySource.title')"
                @click="openModifyModal(idx)"
              >
                <img
                  :src="getConnectionIcon(source.databaseType)"
                  class="shrink-0 h-3.5 w-3.5 object-contain"
                  :alt="source.databaseType"
                >
                <span class="source-chip-name">{{ source.alias }}</span>
                <span class="source-chip-edit i-carbon-settings h-3.5 w-3.5" />
              </button>

              <!-- Add source dropdown -->
              <div ref="addSourcePickerRef" class="add-source-picker">
                <button
                  class="icon-button-sm"
                  :aria-expanded="addSourceOpen"
                  :title="t('dataStudio.addSource.title')"
                  @click.stop="addSourceOpen = !addSourceOpen"
                >
                  <span class="i-carbon-add-alt h-3.5 w-3.5" />
                </button>

                <Transition name="menu-rise">
                  <div v-if="addSourceOpen" class="add-source-menu" @click.stop>
                    <div class="add-source-menu-title">
                      {{ t('dataStudio.addSource.selectConnection') }}
                    </div>

                    <div class="add-source-search-wrap">
                      <span class="add-source-search-icon i-carbon-search h-3.5 w-3.5" />
                      <input
                        v-model="addSourceQuery"
                        class="add-source-search"
                        :placeholder="t('dataStudio.addSource.searchPlaceholder')"
                        autocomplete="off"
                      >
                    </div>

                    <div class="add-source-list">
                      <button
                        v-for="conn in filteredAddConnections"
                        :key="String(conn.id)"
                        class="add-source-item"
                        :class="{ 'add-source-item--selected': addSourceSelectedId === String(conn.id) }"
                        @click="selectAddConnection(conn)"
                      >
                        <div class="add-source-item-icon">
                          <img
                            :src="getConnectionIcon(conn.type)"
                            class="h-4 w-4 object-contain"
                            :alt="conn.type"
                          >
                        </div>
                        <div class="add-source-item-info">
                          <span class="add-source-item-name">{{ conn.name }}</span>
                          <span class="add-source-item-meta">{{ getConnectionMeta(conn) }}</span>
                        </div>
                        <span
                          v-if="addSourceSelectedId === String(conn.id)"
                          class="i-carbon-checkmark text-foreground ml-auto shrink-0 h-3.5 w-3.5"
                        />
                      </button>
                      <div v-if="filteredAddConnections.length === 0" class="add-source-empty">
                        {{ t('dataStudio.addSource.noConnections') }}
                      </div>
                    </div>

                    <div v-if="addSourceSelectedId" class="add-source-permissions">
                      <div class="add-source-permissions-header">
                        <span class="i-carbon-security h-3.5 w-3.5" />
                        <span class="text-xs font-semibold">
                          {{ t('dataStudio.modifySource.accessPermissions') }}
                        </span>
                      </div>
                      <div class="add-source-mode-row">
                        <button
                          class="mode-btn" :class="[addSourceMode === 'Ask' && 'mode-btn--active']"
                          @click="addSourceMode = 'Ask'"
                        >
                          <span class="i-carbon-locked h-3.5 w-3.5" />
                          <span>{{ t('dataStudio.modifySource.modeDefault') }}</span>
                        </button>
                        <button
                          class="mode-btn" :class="[addSourceMode === 'Inherit' && 'mode-btn--active']"
                          @click="addSourceMode = 'Inherit'"
                        >
                          <span class="i-carbon-link h-3.5 w-3.5" />
                          <span>{{ t('dataStudio.modifySource.inheritTitle') }}</span>
                        </button>
                      </div>
                    </div>

                    <div class="add-source-footer">
                      <button
                        class="add-source-connect-btn"
                        :disabled="!addSourceSelectedId"
                        @click="confirmAddSource"
                      >
                        <span class="i-carbon-data-connected h-3.5 w-3.5" />
                        {{ t('dataStudio.addSource.connectSource') }}
                      </button>
                    </div>
                  </div>
                </Transition>
              </div>

              <!-- Permission mode picker -->
              <div class="permission-picker">
                <button
                  class="permission-trigger"
                  :disabled="activeSessionSources.length === 0"
                  :aria-expanded="permissionMenuOpen"
                  :title="t('dataStudio.modifySource.accessPermissions')"
                  @click.stop="activeSessionSources.length > 0 && (permissionMenuOpen = !permissionMenuOpen)"
                >
                  <span
                    class="permission-trigger-icon h-4 w-4"
                    :class="sessionPermissionsMode === 'Auto' ? 'i-carbon-unlocked' : 'i-carbon-locked'"
                  />
                  <span class="permission-trigger-label">
                    {{ sessionPermissionsMode === 'Auto' ? t('dataStudio.modifySource.modeFull') : t('dataStudio.modifySource.modeDefault') }}
                  </span>
                  <span class="permission-trigger-chevron i-carbon-chevron-down h-3.5 w-3.5" />
                </button>
                <div v-if="permissionMenuOpen" class="permission-menu">
                  <div class="permission-menu-title">
                    {{ t('dataStudio.modifySource.accessPermissions') }}
                  </div>
                  <button
                    class="permission-menu-item"
                    :class="{ 'permission-menu-item--active': sessionPermissionsMode === 'Ask' }"
                    @click="setAutoMode(false)"
                  >
                    <span class="permission-menu-icon i-carbon-locked h-3.5 w-3.5" />
                    <span class="permission-menu-label">
                      {{ t('dataStudio.modifySource.modeDefault') }}
                    </span>
                    <span
                      v-if="sessionPermissionsMode === 'Ask'"
                      class="permission-check i-carbon-checkmark h-3.5 w-3.5"
                    />
                  </button>
                  <button
                    class="permission-menu-item"
                    :class="{ 'permission-menu-item--active': sessionPermissionsMode === 'Auto' }"
                    @click="setAutoMode(true)"
                  >
                    <span class="permission-menu-icon i-carbon-unlocked h-3.5 w-3.5" />
                    <span class="permission-menu-label">
                      {{ t('dataStudio.modifySource.modeFull') }}
                    </span>
                    <span
                      v-if="sessionPermissionsMode === 'Auto'"
                      class="permission-check i-carbon-checkmark h-3.5 w-3.5"
                    />
                  </button>
                </div>
              </div>
            </template>
            <template #empty>
              <div class="i-carbon-ibm-watsonx-assistant text-muted-foreground/20 h-12 w-12" />
            </template>
          </ChatPanel>
        </div>
      </div>

      <!-- Modals -->
      <ModifySourceModal v-model:open="showModifyModal" :source-idx="selectedSourceIdx" />
    </div>
  </AppLayout>
</template>

<style scoped>
.data-studio-container {
  background: hsl(var(--background));
}

.data-studio-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.data-studio-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  border-bottom: 1px solid hsl(var(--border));
  flex-shrink: 0;
}

.data-studio-conversation {
  flex: 1;
  min-height: 0;
}

.data-studio-history {
  width: 288px;
  flex-shrink: 0;
  border-right: 1px solid hsl(var(--border));
  overflow-y: auto;
}

.icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: all 0.15s;
}

.icon-button:hover {
  background: hsl(var(--muted));
  color: hsl(var(--foreground));
}

.icon-button--active {
  background: hsl(var(--muted));
  color: hsl(var(--foreground));
}

.icon-button-sm {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: all 0.15s;
}

.icon-button-sm:hover {
  background: hsl(var(--muted));
  color: hsl(var(--foreground));
}

.source-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 24px;
  max-width: 112px;
  padding: 0 8px;
  border: 1px solid hsl(var(--border));
  border-radius: 9999px;
  background: hsl(var(--muted) / 0.5);
  color: hsl(var(--muted-foreground));
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.source-chip:hover {
  border-color: hsl(var(--foreground) / 0.3);
  color: hsl(var(--foreground));
}

.source-chip-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.source-chip-edit {
  opacity: 0.5;
  flex-shrink: 0;
  transition: opacity 0.15s;
}

.source-chip:hover .source-chip-edit {
  opacity: 0.8;
}

.source-chip .source-chip-edit:hover {
  opacity: 1;
}

/* ── Add source ── */

.add-source-picker {
  position: relative;
}

.add-source-menu {
  position: absolute;
  bottom: calc(100% + 4px);
  left: 0;
  z-index: 50;
  width: 288px;
  border: 1px solid hsl(var(--border));
  border-radius: 12px;
  background: hsl(var(--popover));
  box-shadow: 0 8px 24px hsl(0 0% 0% / 0.12);
  overflow: hidden;
}

.add-source-menu-title {
  padding: 10px 12px 6px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: hsl(var(--muted-foreground));
}

.add-source-search-wrap {
  position: relative;
  margin: 0 12px 8px;
}

.add-source-search-icon {
  position: absolute;
  left: 8px;
  top: 50%;
  transform: translateY(-50%);
  color: hsl(var(--muted-foreground) / 0.6);
  pointer-events: none;
}

.add-source-search {
  width: 100%;
  padding: 7px 10px 7px 28px;
  border: 1px solid hsl(var(--border));
  border-radius: 6px;
  background: hsl(var(--background));
  color: hsl(var(--foreground));
  font-size: 13px;
  outline: none;
}

.add-source-search:focus {
  border-color: hsl(var(--foreground) / 0.4);
}

.add-source-search::placeholder {
  color: hsl(var(--muted-foreground) / 0.5);
}

.add-source-list {
  max-height: 192px;
  overflow-y: auto;
}

.add-source-item {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  text-align: left;
  border-bottom: 1px solid hsl(var(--border) / 0.5);
  cursor: pointer;
  transition: background-color 0.12s;
}

.add-source-item:last-child {
  border-bottom: none;
}

.add-source-item:hover {
  background: hsl(var(--muted) / 0.5);
}

.add-source-item--selected {
  background: hsl(var(--muted) / 0.5);
}

.add-source-item-icon {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid hsl(var(--border));
  border-radius: 6px;
  background: hsl(var(--muted));
  flex-shrink: 0;
}

.add-source-item-icon .i-carbon-data-base {
  color: hsl(var(--muted-foreground));
}

.add-source-item-info {
  flex: 1;
  min-width: 0;
}

.add-source-item-name {
  display: block;
  font-size: 13px;
  font-weight: 500;
  color: hsl(var(--foreground));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.add-source-item-meta {
  display: block;
  font-size: 11px;
  color: hsl(var(--muted-foreground));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.add-source-empty {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  padding: 16px;
  text-align: center;
}

.add-source-permissions {
  padding: 8px 12px;
  border-top: 1px solid hsl(var(--border));
}

.add-source-permissions-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
  color: hsl(var(--muted-foreground));
}

.add-source-mode-row {
  display: flex;
  gap: 8px;
}

.mode-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 6px 10px;
  border: 1px solid hsl(var(--border));
  border-radius: 8px;
  background: transparent;
  color: hsl(var(--muted-foreground));
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.12s;
}

.mode-btn:hover {
  background: hsl(var(--muted) / 0.4);
}

.mode-btn--active {
  border-color: hsl(var(--primary) / 0.5);
  background: hsl(var(--primary) / 0.08);
  color: hsl(var(--foreground));
}

.add-source-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 8px 12px;
  border-top: 1px solid hsl(var(--border));
  background: hsl(var(--muted) / 0.2);
}

.add-source-connect-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 5px 10px;
  border: none;
  border-radius: 6px;
  background: hsl(var(--primary));
  color: hsl(var(--primary-foreground));
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s;
}

.add-source-connect-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.add-source-connect-btn:not(:disabled):hover {
  opacity: 0.9;
}

/* ── Permission picker ── */

.permission-picker {
  position: relative;
}

.permission-trigger {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  padding: 0 8px;
  border: 1px solid hsl(var(--border));
  border-radius: 9999px;
  background: hsl(var(--muted) / 0.5);
  color: hsl(var(--foreground));
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.permission-trigger:hover {
  background: hsl(var(--muted));
  color: hsl(var(--foreground));
}

.permission-trigger:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.permission-trigger:disabled:hover {
  background: hsl(var(--muted) / 0.5);
  color: hsl(var(--muted-foreground));
}

.permission-trigger-icon {
  flex-shrink: 0;
}

.permission-trigger-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 60px;
}

.permission-trigger-chevron {
  flex-shrink: 0;
  transition: transform 0.18s ease;
}

.permission-trigger[aria-expanded='true'] .permission-trigger-chevron {
  transform: rotate(180deg);
}

.permission-menu {
  position: absolute;
  bottom: calc(100% + 4px);
  left: 0;
  z-index: 50;
  min-width: 160px;
  padding: 6px;
  border: 1px solid hsl(var(--border));
  border-radius: 10px;
  background: hsl(var(--popover));
  box-shadow: 0 8px 24px hsl(0 0% 0% / 0.12);
}

.permission-menu-title {
  padding: 4px 8px 6px;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: hsl(var(--muted-foreground) / 0.6);
}

.permission-menu-item {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: hsl(var(--foreground));
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  transition: background-color 0.12s;
}

.permission-menu-item:hover {
  background: hsl(var(--muted) / 0.5);
}

.permission-menu-icon {
  color: hsl(var(--muted-foreground));
  flex-shrink: 0;
}

.permission-menu-item--active .permission-menu-icon {
  color: hsl(var(--primary));
}

.permission-menu-label {
  flex: 1;
}

.permission-check {
  color: hsl(var(--primary));
  flex-shrink: 0;
}

.menu-rise-enter-active {
  animation: menu-rise 0.18s cubic-bezier(0.16, 1, 0.3, 1);
}
.menu-rise-leave-active {
  animation: menu-rise 0.14s cubic-bezier(0.16, 1, 0.3, 1) reverse;
}
@keyframes menu-rise {
  from { opacity: 0; transform: scale(0.93) translateY(6px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}
</style>
