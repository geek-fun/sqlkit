<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import ChatPanel from '@/components/chat-panel.vue'
import { useSidebarChatAgent } from '@/composables/useSidebarChatAgent'
import { useDataStudioStore } from '@/store/dataStudioStore'
import SessionHistoryPanel from '@/views/data-studio/components/session-history-panel.vue'

const emit = defineEmits<{
  close: []
}>()

const { t } = useI18n()
const dataStudioStore = useDataStudioStore()

const MIN_WIDTH = 320
const MAX_WIDTH = 600
const DEFAULT_WIDTH = 420

const {
  isLoading,
  error,
  messages,
  sendMessage,
  handleConfirmation: rawHandleConfirmation,
  clearChat,
  dismissError,
  activeSession,
  lastSettings,
  initContextSettings,
  cancelSession,
  stopReason,
  stopMessage,
  progress,
} = useSidebarChatAgent()

function handleConfirmation(msgId: string, event: { toolCallId: string, action: 'allow_once' | 'allow_always' | 'deny' | 'deny_always' | 'cancel' }) {
  rawHandleConfirmation(msgId, event.toolCallId, event.action)
}

const currentWidth = ref(DEFAULT_WIDTH)
const isResizing = ref(false)
const historyPanelOpen = ref(false)

function startResize(e: MouseEvent) {
  isResizing.value = true
  document.addEventListener('mousemove', onResize)
  document.addEventListener('mouseup', stopResize)
  e.preventDefault()
}

function onResize(e: MouseEvent) {
  if (!isResizing.value)
    return
  const newWidth = window.innerWidth - e.clientX - 48
  currentWidth.value = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, newWidth))
}

function stopResize() {
  isResizing.value = false
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
}

function switchSession(sessionId: string) {
  dataStudioStore.sidebarSessionId = sessionId
  historyPanelOpen.value = false
}

async function deleteSession(sessionId: string) {
  await dataStudioStore.removeSession(sessionId)
}

function handleNewSession() {
  dataStudioStore.sidebarSessionId = undefined
  historyPanelOpen.value = false
}

function handleClearChat() {
  clearChat()
}

function handleClose() {
  emit('close')
}

function onModelChange(modelId: string) {
  const sess = dataStudioStore.activeSidebarSession
  if (sess?.id) {
    dataStudioStore.setSessionModelId(sess.id, modelId)
  }
}

onMounted(async () => {
  await dataStudioStore.loadSessions()
  await initContextSettings()
})

onUnmounted(() => {
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
})
</script>

<template>
  <div class="ai-assistant-resizable" :style="{ width: `${currentWidth}px` }">
    <div class="resize-handle" @mousedown="startResize" />

    <!-- Session history slide-over -->
    <transition name="history-slide">
      <div v-if="historyPanelOpen" class="ai-assistant-history-overlay">
        <SessionHistoryPanel
          @select="switchSession"
          @delete="deleteSession"
          @new-session="handleNewSession"
          @close="historyPanelOpen = false"
        />
      </div>
    </transition>

    <ChatPanel
      :messages="messages"
      :is-loading="isLoading"
      :error="error"
      :empty-hint="t('dataStudio.agent.emptyState')"
      :input-placeholder="t('dataStudio.inputPlaceholder')"
      :session-id="activeSession?.id ?? null"
      :context-settings="lastSettings"
      :stop-reason="stopReason"
      :stop-message="stopMessage"
      :progress="progress"
      feature="sidebarAssistant"
      compact
      @send="sendMessage"
      @stop-loop="cancelSession"
      @confirm-tool-call="handleConfirmation"
      @model-change="onModelChange"
      @dismiss-error="dismissError"
    >
      <template #header>
        <div class="header-row">
          <span class="header-title">
            {{ $t('aside.aiAssistant') }}
          </span>
          <div class="header-actions">
            <button
              class="header-icon-btn"
              :title="t('dataStudio.history.newSession')"
              @click="handleNewSession"
            >
              <span class="i-carbon-add h-3.5 w-3.5" />
            </button>
            <button
              class="header-icon-btn"
              :class="{ 'header-icon-btn--active': historyPanelOpen }"
              :title="t('dataStudio.history.title')"
              @click="historyPanelOpen = !historyPanelOpen"
            >
              <span class="i-carbon-time h-3.5 w-3.5" />
            </button>
            <button
              class="header-icon-btn"
              :title="t('dataStudio.agent.clearChat')"
              @click="handleClearChat"
            >
              <span class="i-carbon-trash-can h-3.5 w-3.5" />
            </button>
            <button
              class="header-icon-btn header-icon-btn--close"
              :title="t('common.buttons.close')"
              @click="handleClose"
            >
              <span class="i-carbon-close h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      </template>
    </ChatPanel>
  </div>
</template>

<style scoped>
.ai-assistant-resizable {
  height: 100%;
  display: flex;
  flex-direction: column;
  border-left: 1px solid hsl(var(--border));
  position: relative;
  background: hsl(var(--background));
}

.resize-handle {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 4px;
  cursor: col-resize;
  z-index: 10;
  background: transparent;
  transition: background 0.15s;
}

.resize-handle:hover {
  background: hsl(var(--primary) / 0.3);
}

.ai-assistant-history-overlay {
  position: absolute;
  inset: 0;
  z-index: 20;
  background: hsl(var(--background));
  display: flex;
  flex-direction: column;
}

.history-slide-enter-active,
.history-slide-leave-active {
  transition:
    transform 0.2s cubic-bezier(0.16, 1, 0.3, 1),
    opacity 0.2s ease;
}

.history-slide-enter-from,
.history-slide-leave-to {
  transform: translateX(100%);
  opacity: 0;
}

.header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 12px 12px 16px;
  border-bottom: 1px solid hsl(var(--border));
}

.header-title {
  font-size: 16px;
  font-weight: 700;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.header-icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: all 0.15s;
}

.header-icon-btn:hover {
  background: hsl(var(--muted));
  color: hsl(var(--foreground));
}

.header-icon-btn--active {
  background: hsl(var(--muted));
  color: hsl(var(--foreground));
}

.header-icon-btn--close {
  margin-left: 4px;
}
</style>
