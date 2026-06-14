<script setup lang="ts">
import type { ServerConnection } from '@/store/connectionStore'
import { onClickOutside } from '@vueuse/core'
import { storeToRefs } from 'pinia'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import ChatPanel from '@/components/chat-panel.vue'
import AppLayout from '@/components/layout/AppLayout.vue'
import { disposeAgentRuntime, initAgentRuntime } from '@/composables/agentRuntime'
import { useDataStudioChatAgent } from '@/composables/useDataStudioChatAgent'
import { useAppStore } from '@/store/appStore'
import { DatabaseType, useConnectionStore } from '@/store/connectionStore'
import { useDataStudioStore } from '@/store/dataStudioStore'
import ModifySourceModal from '@/views/data-studio/components/modify-source-modal.vue'
import SessionHistoryPanel from '@/views/data-studio/components/session-history-panel.vue'

const { t } = useI18n()
const appStore = useAppStore()
const connectionStore = useConnectionStore()
const { connections } = storeToRefs(connectionStore)
const dataStudioStore = useDataStudioStore()
const { attachedSources, activeSession } = storeToRefs(dataStudioStore)

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

function selectAddConnection(conn: ServerConnection) {
  addSourceSelectedId.value = String(conn.id)
}

async function confirmAddSource() {
  if (!addSourceSelectedId.value)
    return
  const conn = connections.value.find(c => String(c.id) === addSourceSelectedId.value)
  if (!conn || conn.id === undefined)
    return

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

function setAutoMode(auto: boolean) {
  permissionMenuOpen.value = false
  dataStudioStore.setSessionPermissionsMode(auto ? 'Auto' : 'Ask')
}

async function switchSession(sessionId: string) {
  dataStudioStore.setActiveSession(sessionId)
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
  await appStore.setFeatureModelRoute('dataStudio', {
    selectedModelId: modelId,
    useRecommendedModel: false,
  })
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
</script>

<template>
  <AppLayout>
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
        <div class="px-5 py-3 border-b border-border flex shrink-0 items-center justify-between">
          <div class="flex gap-2 items-center">
            <button
              class="text-muted-foreground rounded inline-flex h-8 w-8 transition-colors items-center justify-center hover:text-foreground hover:bg-muted"
              :class="{ 'bg-muted text-foreground': historyPanelOpen }"
              :title="t('dataStudio.history.title')"
              @click="historyPanelOpen = !historyPanelOpen"
            >
              <span class="i-carbon-time h-5 w-5" />
            </button>
            <span class="text-xs tracking-wider font-bold px-3 py-1 border border-border rounded-full">
              {{ t('dataStudio.title') }}
            </span>
          </div>
          <div class="flex gap-2 items-center">
            <button
              v-if="isLoading"
              class="text-destructive rounded inline-flex h-8 w-8 transition-colors items-center justify-center hover:bg-destructive/10"
              :title="t('dataStudio.agent.stop')"
              @click="cancelSession"
            >
              <span class="i-carbon-stop-filled h-5 w-5" />
            </button>
            <button
              class="text-muted-foreground rounded inline-flex h-8 w-8 transition-colors items-center justify-center hover:text-foreground hover:bg-muted"
              :title="t('dataStudio.history.newSession')"
              @click="startNewSession"
            >
              <span class="i-carbon-add h-5 w-5" />
            </button>
            <button
              v-if="hasMessages"
              class="text-muted-foreground rounded inline-flex h-8 w-8 transition-colors items-center justify-center hover:text-foreground hover:bg-muted"
              :title="t('dataStudio.agent.clearChat')"
              @click="clearChat"
            >
              <span class="i-carbon-trash-can h-5 w-5" />
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
            feature="dataStudio"
            compact
            @send="sendMessage"
            @stop-loop="cancelSession"
            @confirm-tool-call="handleConfirmation"
            @model-change="onModelChange"
          >
            <template #toolbar-left>
              <!-- Source chips -->
              <button
                v-for="(source, idx) in activeSessionSources"
                :key="source.sourceId"
                class="text-[11px] text-muted-foreground font-medium px-2 border border-border rounded-full bg-muted/50 inline-flex gap-1 h-6 max-w-[120px] cursor-pointer transition-colors items-center hover:text-foreground hover:border-foreground/30"
                :title="t('dataStudio.modifySource.title')"
                @click="openModifyModal(idx)"
              >
                <span class="i-carbon-data-base shrink-0 h-3.5 w-3.5" />
                <span class="truncate">{{ source.alias }}</span>
                <span class="i-carbon-settings opacity-60 shrink-0 h-3 w-3" />
              </button>

              <!-- Add source dropdown -->
              <div ref="addSourcePickerRef" class="relative">
                <button
                  class="text-muted-foreground rounded inline-flex h-6 w-6 transition-colors items-center justify-center hover:bg-muted"
                  :title="t('dataStudio.addSource.title')"
                  @click.stop="addSourceOpen = !addSourceOpen"
                >
                  <span class="i-carbon-add-alt h-4 w-4" />
                </button>

                <Transition name="menu-rise">
                  <div
                    v-if="addSourceOpen"
                    class="mb-2 border border-border rounded-xl bg-popover w-72 shadow-lg bottom-full left-0 absolute z-50 overflow-hidden"
                    @click.stop
                  >
                    <div class="text-[11px] text-muted-foreground tracking-wide font-semibold px-3 pb-1.5 pt-2.5 uppercase">
                      {{ t('dataStudio.addSource.selectConnection') }}
                    </div>
                    <div class="px-3 pb-2">
                      <input
                        v-model="addSourceQuery"
                        class="text-sm px-2.5 py-1.5 outline-none border border-border rounded-md bg-background w-full focus:border-foreground/40"
                        :placeholder="t('dataStudio.addSource.searchPlaceholder')"
                      >
                    </div>
                    <div class="border-t border-border max-h-48 overflow-y-auto">
                      <button
                        v-for="conn in filteredAddConnections"
                        :key="String(conn.id)"
                        class="px-3 py-2 text-left border-b border-border/50 flex gap-2.5 w-full transition-colors items-center last:border-b-0 hover:bg-muted"
                        :class="{ 'bg-muted': addSourceSelectedId === String(conn.id) }"
                        @click="selectAddConnection(conn)"
                      >
                        <div class="border border-border rounded bg-muted flex shrink-0 h-7 w-7 items-center justify-center">
                          <span class="i-carbon-data-base text-muted-foreground h-4 w-4" />
                        </div>
                        <div class="flex-1 min-w-0">
                          <div class="text-sm text-foreground font-medium truncate">
                            {{ conn.name }}
                          </div>
                          <div class="text-[11px] text-muted-foreground truncate">
                            {{ getConnectionMeta(conn) }}
                          </div>
                        </div>
                        <span
                          v-if="addSourceSelectedId === String(conn.id)"
                          class="i-carbon-checkmark text-foreground shrink-0 h-3.5 w-3.5"
                        />
                      </button>
                      <div v-if="filteredAddConnections.length === 0" class="text-xs text-muted-foreground p-4 text-center">
                        {{ t('dataStudio.addSource.noConnections') }}
                      </div>
                    </div>
                    <div class="px-3 py-2 border-t border-border bg-muted/20 flex items-center justify-between">
                      <span class="text-[11px] text-muted-foreground font-semibold uppercase">
                        {{ t('dataStudio.addSource.connectionsFound', { count: filteredAddConnections.length }) }}
                      </span>
                      <button
                        class="text-xs text-primary-foreground font-medium px-2.5 py-1.5 rounded-md bg-primary inline-flex gap-1 transition-opacity items-center disabled:opacity-40 hover:opacity-90"
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

              <!-- Permission mode button -->
              <div v-if="activeSessionSources.length > 0" class="relative">
                <button
                  class="text-xs text-foreground px-2 border border-border rounded-full bg-muted/50 inline-flex gap-1 h-6 cursor-pointer transition-colors items-center hover:bg-muted"
                  :title="t('dataStudio.modifySource.accessPermissions')"
                  @click.stop="permissionMenuOpen = !permissionMenuOpen"
                >
                  <span
                    class="h-3.5 w-3.5"
                    :class="sessionPermissionsMode === 'Auto' ? 'i-carbon-unlocked' : 'i-carbon-locked'"
                  />
                  <span class="text-xs">
                    {{ sessionPermissionsMode === 'Auto' ? t('dataStudio.modifySource.inheritTitle') : t('dataStudio.modifySource.modeDefault') }}
                  </span>
                </button>
                <div
                  v-if="permissionMenuOpen"
                  class="mb-2 p-1.5 border border-border rounded-lg bg-popover min-w-[140px] shadow-lg bottom-full left-0 absolute z-50"
                  @click.stop
                >
                  <button
                    class="text-sm px-2.5 py-1.5 text-left rounded flex gap-2 w-full transition-colors items-center hover:bg-muted"
                    :class="{ 'bg-muted': sessionPermissionsMode === 'Ask' }"
                    @click="setAutoMode(false)"
                  >
                    <span class="i-carbon-locked text-muted-foreground h-4 w-4" />
                    <span>{{ t('dataStudio.modifySource.modeDefault') }}</span>
                  </button>
                  <button
                    class="text-sm px-2.5 py-1.5 text-left rounded flex gap-2 w-full transition-colors items-center hover:bg-muted"
                    :class="{ 'bg-muted': sessionPermissionsMode === 'Auto' }"
                    @click="setAutoMode(true)"
                  >
                    <span class="i-carbon-unlocked text-muted-foreground h-4 w-4" />
                    <span>{{ t('dataStudio.modifySource.inheritTitle') }}</span>
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
