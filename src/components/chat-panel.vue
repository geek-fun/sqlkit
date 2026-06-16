<script setup lang="ts">
import type { SessionProgress } from '@/store/dataStudioStore'
import type { ChatMessage } from '@/types/chat'
import { storeToRefs } from 'pinia'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import AgentMessageBubble from '@/components/agent-message-bubble.vue'
import ContextIndicator from '@/components/context-indicator.vue'
import ModelPicker from '@/components/model-picker.vue'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Spinner } from '@/components/ui/spinner'
import { toast } from '@/composables/useNotifications'
import { useAppStore } from '@/store'
import ToolConfirmationCard from '@/views/data-studio/components/tool-confirmation-card.vue'

const props = withDefaults(
  defineProps<{
    messages: ChatMessage[]
    isLoading: boolean
    error?: string
    emptyHint?: string
    inputPlaceholder?: string
    sessionId?: string | null
    contextSettings?: Record<string, unknown> | null
    progress?: SessionProgress | null
    stopReason?: string | null
    stopMessage?: string | null
    feature?: string
    compact?: boolean
    showModelPicker?: boolean
  }>(),
  {
    error: undefined,
    emptyHint: undefined,
    inputPlaceholder: undefined,
    sessionId: null,
    contextSettings: null,
    progress: null,
    stopReason: null,
    stopMessage: null,
    feature: 'dataStudio',
    compact: false,
    showModelPicker: true,
  },
)

const emit = defineEmits<{
  send: [message: string]
  stopLoop: []
  confirmToolCall: [msgId: string, event: { toolCallId: string, action: 'allow_once' | 'allow_always' | 'deny' | 'deny_always' | 'cancel' }]
  modelChange: [modelId: string]
  modelPickerOpen: []
}>()

const { t } = useI18n()
const appStore = useAppStore()
const { llmSettings } = storeToRefs(appStore)

const inputText = ref('')
const scrollAreaRef = ref<{ viewportElement: HTMLElement | null } | null>(null)
const contextIndicatorRef = ref<{ refresh: () => Promise<void> } | null>(null)

// ── Smart scroll ──────────────────────────────────────────────────────

const STICKY_THRESHOLD_PX = 32
const stickToBottom = ref(true)

const getViewport = (): HTMLElement | null => scrollAreaRef.value?.viewportElement ?? null

function isNearBottom(el: HTMLElement): boolean {
  const distance = el.scrollHeight - (el.scrollTop + el.clientHeight)
  return distance <= STICKY_THRESHOLD_PX
}

let scrollRafId = 0

function scrollToBottomForce() {
  if (!stickToBottom.value)
    return
  if (scrollRafId)
    cancelAnimationFrame(scrollRafId)
  scrollRafId = requestAnimationFrame(() => {
    scrollRafId = 0
    if (!stickToBottom.value)
      return
    const el = getViewport()
    if (!el)
      return
    el.scrollTop = el.scrollHeight
  })
}

function scrollToBottomBatched() {
  if (!stickToBottom.value || scrollRafId)
    return
  scrollRafId = requestAnimationFrame(() => {
    scrollRafId = 0
    const el = getViewport()
    if (!el)
      return
    el.scrollTop = el.scrollHeight
  })
}

function forceScrollToBottom() {
  stickToBottom.value = true
  if (scrollRafId)
    cancelAnimationFrame(scrollRafId)
  scrollRafId = 0
  const el = getViewport()
  if (el)
    el.scrollTop = el.scrollHeight
}

function handleViewportScroll() {
  const el = getViewport()
  if (!el)
    return
  stickToBottom.value = isNearBottom(el)
}

watch(
  () => props.messages.length,
  () => requestAnimationFrame(() => scrollToBottomForce()),
)

watch(
  () => {
    const msgs = props.messages
    if (msgs.length === 0)
      return ''
    const last = msgs[msgs.length - 1]
    return `${last.content?.length ?? 0}:${(last as any).thinking?.length ?? 0}:${(last as any).toolCalls?.length ?? 0}`
  },
  () => scrollToBottomBatched(),
)

// ── Model picker ──────────────────────────────────────────────────────

const iterationIndexMap = computed<Record<string, number>>(() => {
  let count = 0
  return props.messages.reduce<Record<string, number>>((acc, msg) => {
    if (msg.role === 'assistant' && (msg as any).toolCalls?.length) {
      acc[msg.id] = count++
    }
    return acc
  }, {})
})

const modelGroups = computed(() =>
  llmSettings.value.providers
    .filter(provider => provider.enabled && (provider.models ?? []).length > 0)
    .map(provider => ({
      id: provider.id,
      label: provider.name,
      models: (provider.models ?? []).map(modelId => ({
        id: `${provider.id}::${modelId}`,
        label: modelId,
        providerConfigId: provider.id,
      })),
    })),
)

const featureRoute = computed(() =>
  props.feature === 'sidebarAssistant'
    ? llmSettings.value.models.sidebarAssistant
    : llmSettings.value.models.dataStudio,
)

const selectedModelId = computed(() => featureRoute.value.selectedModelId ?? undefined)
const recentModelIds = computed(() => (selectedModelId.value ? [selectedModelId.value] : []))

// ── Input ─────────────────────────────────────────────────────────────

function sendMessage() {
  const text = inputText.value.trim()
  if (!text || props.isLoading)
    return
  emit('send', text)
  inputText.value = ''
  forceScrollToBottom()
}

function handleContinue() {
  if (props.isLoading)
    return
  emit('send', 'continue')
}

const canSend = computed(() => inputText.value.trim().length > 0 && !props.isLoading)

// ── Model change ──────────────────────────────────────────────────────

async function onModelChange(modelId: string) {
  emit('modelChange', modelId)
  await appStore.setFeatureModelRoute(props.feature as 'sidebarAssistant' | 'dataStudio', {
    selectedModelId: modelId,
    useRecommendedModel: false,
  })
  const ok = await appStore.verifyModelAvailability(modelId)
  if (!ok)
    toast.warning(t('dataStudio.modelUnavailable') || 'Selected model is not available')
}

function onModelPickerOpen() {
  emit('modelPickerOpen')
  llmSettings.value.providers
    .filter(p => p.enabled)
    .forEach(p => appStore.syncProviderModels(p.id).catch(() => {}))
}

// ── Lifecycle ─────────────────────────────────────────────────────────

onMounted(async () => {
  await nextTick()
  const el = getViewport()
  el?.addEventListener('scroll', handleViewportScroll, { passive: true })
  forceScrollToBottom()
})

onBeforeUnmount(() => {
  const el = getViewport()
  el?.removeEventListener('scroll', handleViewportScroll)
  if (scrollRafId)
    cancelAnimationFrame(scrollRafId)
})

// ── Status helpers ────────────────────────────────────────────────────
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Header slot -->
    <div v-if="$slots.header" class="shrink-0">
      <slot name="header" />
    </div>

    <!-- Messages area -->
    <div class="flex-1 min-h-0">
      <ScrollArea ref="scrollAreaRef" class="h-full">
        <!-- Empty state -->
        <div v-if="messages.length === 0 && !isLoading" class="px-4 text-center flex flex-col h-full items-center justify-center">
          <div class="i-carbon-ibm-watsonx-assistant text-muted-foreground/20 mb-4 h-16 w-16" />
          <p class="text-sm text-muted-foreground max-w-xs">
            <slot name="empty">
              {{ emptyHint || t('dataStudio.agent.emptyState') }}
            </slot>
          </p>
        </div>

        <!-- Message bubbles -->
        <AgentMessageBubble
          v-for="msg in messages"
          :key="msg.id"
          :message="msg"
          :iteration-index="iterationIndexMap[msg.id]"
        />

        <!-- Tool confirmation cards -->
        <div v-for="msg in messages" :key="`conf-${msg.id}`">
          <ToolConfirmationCard
            v-for="tc in ((msg as any).toolCalls?.filter((tc: any) => tc.requiresConfirmation && tc.status === 'pending') ?? [])"
            :key="tc.id"
            :tool-call="tc"
            @confirm="emit('confirmToolCall', msg.id, $event)"
          />
        </div>

        <!-- Error banner -->
        <div v-if="error" class="text-sm text-destructive mx-4 mb-2 p-3 border border-destructive/20 rounded-lg bg-destructive/10 flex gap-2 items-center">
          <span class="i-carbon-warning shrink-0 h-4 w-4" />
          <span>{{ error }}</span>
        </div>

        <!-- Progress indicator -->
        <div v-if="isLoading && progress" class="text-xs text-muted-foreground py-1 flex gap-2 items-center justify-center">
          <span class="i-carbon-loading h-3.5 w-3.5 animate-spin" />
          <span v-if="progress.phase === 'iterating'">
            {{ t('dataStudio.agent.iteration') }} {{ progress.iter }}/{{ progress.maxIter }}
          </span>
          <span v-else-if="progress.phase === 'waiting_llm'">
            {{ t('dataStudio.agent.waitingModel') }}
          </span>
          <span v-else-if="progress.phase === 'compacting'">
            {{ t('dataStudio.agent.compacting') }}
          </span>
          <span v-else-if="progress.phase === 'preparing'">
            {{ t('dataStudio.agent.preparing') }}
          </span>
        </div>
      </ScrollArea>
    </div>

    <!-- Stop banner -->
    <div v-if="stopReason && stopMessage" class="mx-4 mb-2 p-3 border rounded-xl flex gap-2 items-center justify-between" style="border-color: hsl(38 92% 50% / 0.35); background: hsl(38 92% 50% / 0.08);">
      <div class="flex gap-2 items-start">
        <span class="i-carbon-pause-filled shrink-0 h-4 w-4" style="color: hsl(38 92% 50%);" />
        <div class="flex flex-col gap-0.5">
          <span class="text-xs font-medium">{{ stopMessage }}</span>
          <span class="text-[11px] text-muted-foreground">{{ t('dataStudio.agent.continueHint') }}</span>
        </div>
      </div>
      <div class="flex shrink-0 gap-1.5">
        <button
          class="text-xs font-semibold px-2.5 py-1 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          style="border: 1px solid transparent; background: transparent; color: hsl(var(--muted-foreground));"
          :disabled="isLoading"
          @click="emit('stopLoop')"
        >
          {{ t('dataStudio.agent.stop') }}
        </button>
        <button
          class="text-xs font-semibold px-2.5 py-1 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          style="border: 1px solid hsl(38 92% 50% / 0.5); background: hsl(38 92% 50% / 0.15);"
          :disabled="isLoading"
          @click="handleContinue"
        >
          {{ t('dataStudio.agent.continue') }}
        </button>
      </div>
    </div>

    <!-- Input area -->
    <div class="chat-input-area">
      <div v-if="stopReason && stopMessage" class="loop-stopped-banner" role="status">
        <div class="loop-stopped-banner__body">
          <span class="loop-stopped-banner__icon i-carbon-pause-filled" />
          <div class="loop-stopped-banner__texts">
            <span class="loop-stopped-banner__message">{{ stopMessage }}</span>
            <span class="loop-stopped-banner__hint">{{ t('dataStudio.agent.continueHint') }}</span>
          </div>
        </div>
        <div class="loop-stopped-banner__actions">
          <button
            class="loop-stopped-banner__action loop-stopped-banner__action--secondary"
            type="button"
            :disabled="isLoading"
            @click="emit('stopLoop')"
          >
            {{ t('dataStudio.agent.stop') }}
          </button>
          <button
            class="loop-stopped-banner__action loop-stopped-banner__action--primary"
            type="button"
            :disabled="isLoading"
            @click="handleContinue"
          >
            {{ t('dataStudio.agent.continue') }}
          </button>
        </div>
      </div>

      <div class="chat-input-wrapper">
        <textarea
          v-model="inputText"
          class="chat-textarea"
          rows="3"
          :placeholder="inputPlaceholder || t('dataStudio.inputPlaceholder')"
          autocomplete="off"
          autocorrect="off"
          autocapitalize="off"
          spellcheck="false"
          data-form-type="other"
          :disabled="isLoading"
          @keydown.enter.exact.prevent="sendMessage"
        />

        <div class="chat-toolbar">
          <div class="toolbar-left">
            <slot name="toolbar-left" />
          </div>

          <div class="toolbar-center">
            <ContextIndicator
              v-if="sessionId"
              ref="contextIndicatorRef"
              :session-id="sessionId"
              :settings="contextSettings ?? null"
            />
            <ModelPicker
              v-if="showModelPicker"
              :groups="modelGroups"
              :model-value="selectedModelId"
              :recent-model-ids="recentModelIds"
              :compact="compact"
              @open="onModelPickerOpen"
              @update:model-value="onModelChange"
            />
          </div>

          <button
            class="send-button"
            :class="{ 'send-button--stop': isLoading }"
            :disabled="!canSend && !isLoading"
            @click="isLoading ? emit('stopLoop') : sendMessage()"
          >
            <Spinner v-if="isLoading" size="sm" />
            <span v-else class="i-carbon-arrow-up h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-input-area {
  padding: 8px;
  position: relative;
}

.chat-input-wrapper {
  display: flex;
  flex-direction: column;
  border: 1px solid hsl(var(--border));
  border-radius: 12px;
  background: hsl(var(--background));
}

.chat-textarea {
  width: 100%;
  min-height: 60px;
  padding: 8px 10px;
  border: none;
  outline: none;
  background: transparent;
  color: hsl(var(--foreground));
  font-size: 13px;
  resize: none;
  line-height: 1.5;
}

.chat-textarea:focus {
  outline: none;
}

.chat-textarea:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.chat-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  height: 36px;
  border-top: 1px solid hsl(var(--border));
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 4px;
}

.toolbar-center {
  flex: 1;
  display: flex;
  justify-content: flex-end;
  gap: 4px;
}

.send-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: hsl(var(--foreground));
  color: hsl(var(--background));
  border: none;
  cursor: pointer;
  transition: opacity 0.2s;
  flex-shrink: 0;
}

.send-button:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.send-button--stop {
  background: hsl(var(--destructive));
  color: hsl(var(--destructive-foreground));
}

.loop-stopped-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 8px;
  padding: 8px 12px;
  border-radius: 10px;
  border: 1px solid hsl(38 92% 50% / 0.35);
  background: hsl(38 92% 50% / 0.08);
  color: hsl(var(--foreground));
}

.loop-stopped-banner__body {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  flex: 1 1 auto;
}

.loop-stopped-banner__texts {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.loop-stopped-banner__icon {
  flex: 0 0 auto;
  width: 16px;
  height: 16px;
  margin-top: 1px;
  color: hsl(38 92% 50%);
}

.loop-stopped-banner__message {
  font-size: 13px;
  font-weight: 500;
  line-height: 1.4;
}

.loop-stopped-banner__hint {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  line-height: 1.4;
}

.loop-stopped-banner__actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}

.loop-stopped-banner__action {
  padding: 4px 10px;
  font-size: 12px;
  font-weight: 600;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.loop-stopped-banner__action--primary {
  border: 1px solid hsl(38 92% 50% / 0.5);
  background: hsl(38 92% 50% / 0.15);
  color: hsl(var(--foreground));
}

.loop-stopped-banner__action--primary:hover:not(:disabled) {
  background: hsl(38 92% 50% / 0.25);
}

.loop-stopped-banner__action--secondary {
  border: 1px solid transparent;
  background: transparent;
  color: hsl(var(--muted-foreground));
}

.loop-stopped-banner__action--secondary:hover:not(:disabled) {
  background: hsl(var(--secondary));
  color: hsl(var(--foreground));
}

.loop-stopped-banner__action:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
