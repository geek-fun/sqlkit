import type { ComputedRef, Ref } from 'vue'
import type { UseChatAgentConfig } from './useChatAgent'
import type { AgentMessage, AgentSession, AgentToolCall, AgentToolCallStatus, AttachedSource, ConfirmationRule, SessionSource } from '@/store/dataStudioStore'
import type {
  ChatMessage,
  ChatMessageRole,
  ChatMessageStatus,
  ChatSession,
  ChatSessionStatus,
  ConnectionEntry,
} from '@/types/chat'
import { storeToRefs } from 'pinia'
import { computed } from 'vue'
import { useConnectionStore } from '@/store/connectionStore'
import {

  useDataStudioStore,
} from '@/store/dataStudioStore'
import { useChatAgent } from './useChatAgent'

function adaptDataStudioMessage(msg: AgentMessage): ChatMessage {
  return {
    id: msg.id,
    role: msg.role as ChatMessageRole,
    content: msg.content,
    status: msg.status as ChatMessageStatus,
    timestamp: msg.timestamp,
    thinking: msg.thinking,
    thinkingDuration: msg.thinkingDuration,
    toolCalls: msg.toolCalls,
    toolCallId: msg.toolCallId,
    compaction: msg.compaction,
    compactionInProgress: msg.compactionInProgress,
    preparingInProgress: msg.preparingInProgress,
  }
}

function adaptDataStudioSession(session: AgentSession): ChatSession {
  return {
    id: session.id,
    messages: session.messages.map(adaptDataStudioMessage),
    status: session.status as ChatSessionStatus,
    sources: session.sources,
    permissionsMode: session.permissionsMode,
    maxIterations: 200,
    stopReason: session.stopReason,
    stopMessage: session.stopMessage,
  }
}

function getNonDetachedSources(sources: SessionSource[]): SessionSource[] {
  return sources.filter(source => !source.detached)
}

function resolveDatabaseSource(attachedSources: AttachedSource[], sessionSource: SessionSource): AttachedSource | undefined {
  return attachedSources.find(source => source.sourceId === sessionSource.sourceId)
}

export function useDataStudioChatAgent() {
  const dataStudioStore = useDataStudioStore()
  const connectionStore = useConnectionStore()
  const {
    attachedSources,
    confirmationRules,
    sessions: rawSessions,
  } = storeToRefs(dataStudioStore)

  const sessions = computed(() => rawSessions.value.map(adaptDataStudioSession))
  const activeSession = computed(() => {
    const found = rawSessions.value.find(s => s.id === dataStudioStore.activeSessionId)
    return found ? adaptDataStudioSession(found) : undefined
  })

  const activeSessionSources = computed(() => {
    const session = rawSessions.value.find(e => e.id === dataStudioStore.activeSessionId)
    return session ? getNonDetachedSources(session.sources) : []
  })

  const contextProvider = () => {
    const session = rawSessions.value.find(e => e.id === dataStudioStore.activeSessionId)
    const activeSources = session ? getNonDetachedSources(session.sources) : []

    const connections = activeSources.reduce<Record<string, ConnectionEntry>>(
      (acc, sessionSource) => {
        const attachedSource = resolveDatabaseSource(attachedSources.value, sessionSource)
        if (!attachedSource || attachedSource.kind !== 'database')
          return acc

        const connection = connectionStore.connections.find(
          c => Number(c.id) === Number(attachedSource.connectionId),
        )
        if (!connection || connection.id == null)
          return acc

        acc[sessionSource.alias] = {
          connectionId: String(connection.id),
          dbType: attachedSource.databaseType,
          permissions: sessionSource.permissions,
        }
        return acc
      },
      {},
    )

    return { connections }
  }

  const config: UseChatAgentConfig = {
    feature: 'dataStudio',
    sessionStore: {
      sessions: sessions as unknown as Ref<Array<ChatSession>>,
      activeSessionId: computed(() => dataStudioStore.activeSessionId ?? null),
      activeSession: activeSession as ComputedRef<ChatSession | undefined>,
      addMessage: (_sessionId: string, message: ChatMessage) => {
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
          compaction: message.compaction,
        })
      },
      updateStreamingContent: (_sessionId: string, messageId: string, chunk: string) =>
        dataStudioStore.updateStreamingContent(messageId, chunk),
      updateStreamingThinking: (_sessionId: string, messageId: string, chunk: string) =>
        dataStudioStore.updateStreamingThinking(messageId, chunk),
      setMessageStatus: (_sessionId: string, messageId: string, status: ChatMessageStatus) =>
        dataStudioStore.setMessageStatus(messageId, status as 'pending' | 'streaming' | 'done' | 'error'),
      setMessageToolCalls: (_sessionId: string, messageId: string, toolCalls: Array<AgentToolCall>) =>
        dataStudioStore.setMessageToolCalls(messageId, toolCalls),
      removeOrphanedStreamingMessages: (_sessionId: string, _finalizedMessageId: string) =>
        dataStudioStore.removeOrphanedStreamingMessages(),
      updateToolCallStatus: (messageId: string, toolCallId: string, status: AgentToolCallStatus, _result?: string, _durationMs?: number, _sessionId?: string) =>
        dataStudioStore.updateToolCallStatus(messageId, toolCallId, status),
      setSessionStatus: (sessionId: string, status: ChatSessionStatus) =>
        dataStudioStore.setSessionStatus(sessionId, status as 'idle' | 'running' | 'waiting_confirmation' | 'error' | 'stopped'),
      setSessionStopped: (sessionId, reason, message) =>
        dataStudioStore.setSessionStopped(sessionId, reason as any, message),
      clearSessionStop: (sessionId: string) => dataStudioStore.clearSessionStop(sessionId),
      setSessionSchema: (_sessionId: string, _schema: string) => undefined,
      clearSession: (sessionId: string) => dataStudioStore.clearSession(sessionId),
      getOrCreateSession: async () => {
        const session = await dataStudioStore.getOrCreateSession()
        return session.id
      },
      reloadSessionMessages: (sessionId: string) => dataStudioStore.reloadSessionMessages(sessionId),
    },
    contextProvider,
    confirmationRules: confirmationRules as Ref<ConfirmationRule[]>,
    addConfirmationRule: rule => dataStudioStore.addConfirmationRule(rule),
    findConfirmationRule: (sessionId: string, toolName: string) =>
      dataStudioStore.findConfirmationRule(sessionId, toolName),
    autoMode: computed(() => {
      const session = rawSessions.value.find(e => e.id === dataStudioStore.activeSessionId)
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

  const stopReason = computed(() => activeSession.value?.stopReason ?? null)
  const stopMessage = computed(() => activeSession.value?.stopMessage ?? null)
  const progress = computed(() =>
    dataStudioStore.activeSessionId
      ? dataStudioStore.getSessionProgress(dataStudioStore.activeSessionId)
      : null,
  )

  return {
    isLoading: agent.isLoading,
    error: agent.error,
    activeSession: agent.activeSession,
    activeSessionSources,
    messages,
    lastSettings: agent.lastSettings,
    initContextSettings: agent.initContextSettings,
    sendMessage,
    handleConfirmation: agent.handleConfirmation,
    cancelSession: agent.cancelSession,
    clearChat: agent.clearChat,
    dismissError: agent.dismissError,
    attachedSources,
    confirmationRules,
    stopReason,
    stopMessage,
    progress,
  }
}
