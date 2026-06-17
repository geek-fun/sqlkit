import type { ComputedRef, Ref } from 'vue'
import type { UseChatAgentConfig } from './useChatAgent'
import type { AgentSession, AgentToolCall, AgentToolCallStatus, ConfirmationRule } from '@/store/dataStudioStore'
import type {
  ChatContextConfig,
  ChatMessage,
  ChatMessageRole,
  ChatMessageStatus,
  ChatSession,
  ChatSessionStatus,
} from '@/types/chat'
import { storeToRefs } from 'pinia'
import { computed } from 'vue'
import { useAppStore } from '@/store/appStore'
import { useDataStudioStore } from '@/store/dataStudioStore'
import { useTabStore } from '@/store/tabStore'
import { clearSessionRuntime } from './agentRuntime'
import { useChatAgent } from './useChatAgent'

function adaptSidebarSession(session: AgentSession): ChatSession {
  const appStore = useAppStore()
  return {
    id: session.id,
    messages: session.messages.map(msg => ({
      id: msg.id,
      role: msg.role as ChatMessageRole,
      content: msg.content,
      status: msg.status as ChatMessageStatus,
      timestamp: msg.timestamp,
      thinking: msg.thinking,
      thinkingDuration: msg.thinkingDuration,
      toolCalls: msg.toolCalls,
      toolCallId: msg.toolCallId,
      compaction: msg.compaction as ChatMessage['compaction'],
      compactionInProgress: msg.compactionInProgress,
      preparingInProgress: msg.preparingInProgress,
    })),
    status: session.status as ChatSessionStatus,
    sources: session.sources,
    permissionsMode: session.permissionsMode as 'Ask' | 'Auto',
    maxIterations: appStore.llmSettings.chat.maxIterations,
    stopReason: session.stopReason,
    stopMessage: session.stopMessage,
  }
}

export function useSidebarChatAgent() {
  const dataStudioStore = useDataStudioStore()
  const {
    confirmationRules,
    sessions: rawSessions,
  } = storeToRefs(dataStudioStore)

  const sessions = computed(() => rawSessions.value.map(adaptSidebarSession))
  const activeSession = computed(() => {
    const found = rawSessions.value.find(s => s.id === dataStudioStore.sidebarSessionId)
    return found ? adaptSidebarSession(found) : undefined
  })

  const contextProvider = () => {
    const tabStore = useTabStore()
    const activeTab = tabStore.activeTab
    const context: ChatContextConfig = {}

    if (activeTab?.connectionId) {
      context.connections = {
        [activeTab.name || 'active']: {
          connectionId: activeTab.connectionId,
          dbType: 'SQL',
          permissions: { read: true, create: false, update: false, delete: false },
        },
      }
    }

    if (activeTab?.content) {
      context.activePanel = { editorContent: activeTab.content }
    }

    return context
  }

  const config: UseChatAgentConfig = {
    feature: 'sidebarAssistant',
    sessionStore: {
      sessions: sessions as unknown as Ref<Array<ChatSession>>,
      activeSessionId: computed(() => dataStudioStore.sidebarSessionId ?? null),
      activeSession: activeSession as ComputedRef<ChatSession | undefined>,
      addMessage: (sessionId: string, message: ChatMessage) => {
        dataStudioStore.addMessage({
          id: message.id,
          role: message.role as 'user' | 'assistant' | 'tool' | 'system',
          content: message.content,
          status: message.status as 'pending' | 'streaming' | 'done' | 'error',
          timestamp: message.timestamp,
          thinking: message.thinking,
          thinkingDuration: message.thinkingDuration,
          toolCalls: message.toolCalls,
          toolCallId: message.toolCallId,
          compaction: message.compaction as never,
        }, sessionId)
      },
      updateStreamingContent: (sessionId: string, messageId: string, chunk: string) =>
        dataStudioStore.updateStreamingContent(messageId, chunk, sessionId),
      updateStreamingThinking: (sessionId: string, messageId: string, chunk: string) =>
        dataStudioStore.updateStreamingThinking(messageId, chunk, sessionId),
      setMessageStatus: (sessionId: string, messageId: string, status: ChatMessageStatus) =>
        dataStudioStore.setMessageStatus(messageId, status as 'pending' | 'streaming' | 'done' | 'error', sessionId),
      setMessageToolCalls: (sessionId: string, messageId: string, toolCalls: Array<AgentToolCall>) =>
        dataStudioStore.setMessageToolCalls(messageId, toolCalls, sessionId),
      removeOrphanedStreamingMessages: (sessionId: string, _finalizedMessageId: string) =>
        dataStudioStore.removeOrphanedStreamingMessages(sessionId),
      updateToolCallStatus: (messageId: string, toolCallId: string, status: AgentToolCallStatus, result?: string, durationMs?: number, sessionId?: string) =>
        dataStudioStore.updateToolCallStatus(messageId, toolCallId, status, result, durationMs, sessionId),
      setSessionStatus: (sessionId: string, status: ChatSessionStatus) =>
        dataStudioStore.setSessionStatus(sessionId, status as 'idle' | 'running' | 'waiting_confirmation' | 'error' | 'stopped'),
      setSessionStopped: (sessionId, reason, message) =>
        dataStudioStore.setSessionStopped(sessionId, reason as never, message),
      clearSessionStop: (sessionId: string) => dataStudioStore.clearSessionStop(sessionId),
      clearSession: (sessionId: string) => dataStudioStore.clearSession(sessionId),
      getOrCreateSession: () => {
        return dataStudioStore.getOrCreateSidebarSession()
      },
      reloadSessionMessages: (sessionId: string) => dataStudioStore.reloadSessionMessages(sessionId),
    },
    contextProvider,
    confirmationRules: confirmationRules as Ref<ConfirmationRule[]>,
    addConfirmationRule: rule => dataStudioStore.addConfirmationRule(rule),
    findConfirmationRule: (sessionId: string, toolName: string) =>
      dataStudioStore.findConfirmationRule(sessionId, toolName),
    autoMode: computed(() => {
      const session = rawSessions.value.find(e => e.id === dataStudioStore.sidebarSessionId)
      return session?.permissionsMode === 'Auto'
    }) as Ref<boolean>,
  }

  const agent = useChatAgent(config)

  const messages = computed(() => agent.activeSession.value?.messages ?? [])

  const sendMessage = async (content: string) => {
    await agent.sendMessage({
      content,
      context: contextProvider(),
    })
  }

  const startNewSession = () => {
    const oldSessionId = dataStudioStore.sidebarSessionId
    if (oldSessionId) {
      clearSessionRuntime(oldSessionId)
    }
    dataStudioStore.sidebarSessionId = undefined
  }

  return {
    isLoading: agent.isLoading,
    error: agent.error,
    activeSession: agent.activeSession,
    messages,
    lastSettings: agent.lastSettings,
    initContextSettings: agent.initContextSettings,
    sendMessage,
    startNewSession,
    handleConfirmation: agent.handleConfirmation,
    cancelSession: agent.cancelSession,
    clearChat: agent.clearChat,
    dismissError: agent.dismissError,
    stopReason: computed(() => activeSession.value?.stopReason ?? null),
    stopMessage: computed(() => activeSession.value?.stopMessage ?? null),
    progress: computed(() =>
      dataStudioStore.sidebarSessionId
        ? dataStudioStore.getSessionProgress(dataStudioStore.sidebarSessionId)
        : null,
    ),
  }
}
