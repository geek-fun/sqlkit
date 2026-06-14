<script setup lang="ts">
import type { SessionProgress } from '@/store/dataStudioStore'
import type { ChatMessage, ChatMessageStatus } from '@/types/chat'
import { nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import ToolConfirmationCard from '@/views/data-studio/components/tool-confirmation-card.vue'

const props = defineProps<{
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
  feature: string
  compact?: boolean
}>()

const emit = defineEmits<{
  'send': [message: string]
  'stopLoop': []
  'confirmToolCall': [msgId: string, event: { toolCallId: string, action: 'allow_once' | 'allow_always' | 'deny' | 'deny_always' | 'cancel' }]
  'model-change': [modelId: string]
  'model-picker-open': []
}>()

const { t } = useI18n()
const inputText = ref('')
const messagesContainer = ref<HTMLElement | null>(null)
const inputRef = ref<HTMLTextAreaElement | null>(null)

function sendMessage() {
  const text = inputText.value.trim()
  if (!text || props.isLoading)
    return
  emit('send', text)
  inputText.value = ''
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    sendMessage()
  }
}

watch(() => props.messages.length, async () => {
  await nextTick()
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
  }
})

function statusLabel(status: ChatMessageStatus): string {
  switch (status) {
    case 'pending': return '...'
    case 'streaming': return '...'
    case 'error': return `[${t('common.error')}]`
    default: return ''
  }
}

function messageClass(role: string): string {
  if (role === 'user')
    return 'bg-primary/10 ml-8'
  if (role === 'system')
    return 'bg-muted/30 text-xs text-muted-foreground italic text-center'
  return 'bg-muted/30 mr-8'
}

function alignClass(role: string): string {
  if (role === 'user')
    return 'justify-end'
  return 'justify-start'
}

function adjustTextareaHeight(e: Event) {
  const el = e.target as HTMLTextAreaElement
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, 120)}px`
}
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Header slot -->
    <div v-if="$slots.header" class="shrink-0">
      <slot name="header" />
    </div>

    <!-- Messages area -->
    <div ref="messagesContainer" class="px-4 py-4 flex-1 overflow-y-auto space-y-3">
      <!-- Empty state -->
      <div v-if="messages.length === 0 && !isLoading" class="text-center flex flex-col h-full items-center justify-center">
        <div class="i-carbon-ibm-watsonx-assistant text-muted-foreground/20 mb-4 h-16 w-16" />
        <p class="text-sm text-muted-foreground max-w-xs">
          <slot name="empty">
            {{ emptyHint || t('dataStudio.agent.emptyState') }}
          </slot>
        </p>
      </div>

      <!-- Message bubbles -->
      <div v-for="msg in messages" :key="msg.id" :class="alignClass(msg.role)" class="flex">
        <div class="px-3 py-2 rounded-lg max-w-[80%]" :class="[messageClass(msg.role)]">
          <!-- Preparing indicator -->
          <div v-if="msg.preparingInProgress" class="text-sm text-muted-foreground flex gap-2 items-center">
            <span class="i-carbon-loading h-4 w-4 animate-spin" />
            <span>{{ t('dataStudio.agent.preparing') }}</span>
          </div>

          <!-- Compaction marker -->
          <div v-else-if="msg.compaction" class="text-xs text-muted-foreground/60">
            <div class="flex gap-1.5 items-center">
              <span class="i-carbon-compress h-3 w-3" />
              <span>{{ msg.compaction.summary }}</span>
            </div>
          </div>

          <!-- Normal content -->
          <div v-else class="text-sm whitespace-pre-wrap break-words">
            {{ msg.content }}
            <span v-if="msg.status === 'streaming' || msg.status === 'pending'" class="ml-0.5 bg-current h-4 w-2 inline-block animate-pulse" />
            <span v-if="msg.status === 'error'" class="text-destructive">{{ statusLabel(msg.status) }}</span>
          </div>

          <!-- Thinking content -->
          <div v-if="msg.thinking && msg.role === 'assistant'" class="mt-1 pt-1 border-t border-border/30">
            <details class="text-xs text-muted-foreground/60">
              <summary class="cursor-pointer transition-colors hover:text-foreground">
                {{ t('dataStudio.agent.thinking') }}{{ msg.thinkingDuration ? ` (${msg.thinkingDuration}s)` : '' }}
              </summary>
              <div class="mt-1 whitespace-pre-wrap">
                {{ msg.thinking }}
              </div>
            </details>
          </div>

          <!-- Tool calls -->
          <div v-if="msg.toolCalls && msg.toolCalls.length > 0 && msg.role === 'assistant'" class="mt-2 space-y-1">
            <div v-for="tc in msg.toolCalls" :key="tc.id" class="text-xs">
              <div class="flex gap-1.5 items-center" :class="tc.status === 'error' ? 'text-destructive' : 'text-muted-foreground'">
                <span
                  :class="tc.status === 'done' ? 'i-carbon-checkmark' : tc.status === 'error' ? 'i-carbon-warning' : 'i-carbon-tool-box'"
                  class="shrink-0 h-3.5 w-3.5"
                />
                <span class="font-mono">{{ tc.toolName }}</span>
                <span v-if="tc.durationMs" class="opacity-60">({{ tc.durationMs }}ms)</span>
              </div>
              <div v-if="tc.result" class="text-muted-foreground/60 mt-0.5 max-w-full truncate">
                {{ tc.result }}
                <button
                  v-if="tc.resultTruncated"
                  class="ml-1 underline hover:text-foreground"
                  :title="t('dataStudio.agent.viewFullResult')"
                >
                  {{ t('dataStudio.agent.viewMore') }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Tool confirmation cards (shown separately) -->
      <div v-for="msg in messages" :key="`conf-${msg.id}`">
        <ToolConfirmationCard
          v-for="tc in (msg.toolCalls?.filter(tc => tc.requiresConfirmation && tc.status === 'pending') ?? [])"
          :key="tc.id"
          :tool-call="tc"
          @confirm="emit('confirmToolCall', msg.id, $event)"
        />
      </div>

      <!-- Error banner -->
      <div v-if="error" class="text-sm text-destructive p-3 border border-destructive/20 rounded-lg bg-destructive/10 flex gap-2 items-center">
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
    </div>

    <!-- Stop banner -->
    <div v-if="stopReason && stopMessage" class="text-xs text-amber-700 mx-4 mb-2 p-2 border border-amber-200 rounded-lg bg-amber-50 flex gap-2 items-center">
      <span class="i-carbon-pause-filled shrink-0 h-3.5 w-3.5" />
      <span>{{ stopMessage }}</span>
    </div>

    <!-- Input area -->
    <div class="px-4 py-3 border-t border-border shrink-0">
      <div class="flex gap-2 items-center">
        <!-- Toolbar left slot -->
        <div v-if="$slots['toolbar-left']" class="flex gap-1 items-center">
          <slot name="toolbar-left" />
        </div>

        <div class="flex-1 relative">
          <textarea
            ref="inputRef"
            v-model="inputText"
            :placeholder="inputPlaceholder || t('dataStudio.inputPlaceholder')"
            class="text-sm px-3 py-2 outline-none border border-border rounded-lg bg-background max-h-[120px] min-h-[36px] w-full resize-none transition-colors focus:border-foreground/40"
            rows="1"
            :disabled="isLoading"
            @keydown="handleKeydown"
            @input="adjustTextareaHeight"
          />
        </div>

        <div class="flex gap-1 items-center">
          <!-- Stop button -->
          <button
            v-if="isLoading"
            class="text-destructive-foreground rounded-lg bg-destructive inline-flex h-8 w-8 transition-opacity items-center justify-center hover:opacity-90"
            :title="t('dataStudio.agent.stop')"
            @click="emit('stopLoop')"
          >
            <span class="i-carbon-stop-filled h-4 w-4" />
          </button>

          <!-- Send button -->
          <button
            v-else
            class="text-primary-foreground rounded-lg bg-primary inline-flex h-8 w-8 transition-opacity items-center justify-center disabled:opacity-40 hover:opacity-90"
            :disabled="!inputText.trim()"
            @click="sendMessage"
          >
            <span class="i-carbon-send h-4 w-4" />
          </button>
        </div>

        <!-- Toolbar right slot -->
        <div v-if="$slots['toolbar-right']" class="flex gap-1 items-center">
          <slot name="toolbar-right" />
        </div>
      </div>
    </div>
  </div>
</template>
