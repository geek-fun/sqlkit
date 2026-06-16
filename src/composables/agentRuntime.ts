import type { ToolDefinition, ToolMetadata } from '@/datasources/agentApi'
import type { AgentSession, AgentSessionStopReason, AgentToolCall } from '@/store/dataStudioStore'
import { ulid } from 'ulidx'
import {
  agentApi,

} from '@/datasources/agentApi'
import {

  useDataStudioStore,
} from '@/store/dataStudioStore'

type SessionRuntime = {
  tools?: Array<ToolDefinition>
  metadata?: Record<string, ToolMetadata>
}

const sessionRuntimes = new Map<string, SessionRuntime>()
let runtimeInitPromise: Promise<void> | null = null
let runtimeInitialized = false
let runtimeDisposed = false
let runtimeUnlisteners: Array<() => void> = []

function getSessionRuntime(sessionId: string): SessionRuntime {
  if (!sessionRuntimes.has(sessionId))
    sessionRuntimes.set(sessionId, {})
  return sessionRuntimes.get(sessionId)!
}

function clearSessionRuntime(sessionId: string) {
  sessionRuntimes.delete(sessionId)
}

function getSessionById(sessions: Array<AgentSession>, sessionId: string): AgentSession | undefined {
  return sessions.find(s => s.id === sessionId)
}

function shouldRequireConfirmation(session: AgentSession, toolName: string, riskLevel: AgentToolCall['riskLevel']): boolean {
  const rule = useDataStudioStore().findConfirmationRule(session.id, toolName)
  if (rule?.action === 'allow' || rule?.action === 'deny')
    return false
  if (riskLevel === 'safe')
    return false
  if (riskLevel === 'destructive')
    return true
  return session.permissionsMode !== 'Auto'
}

function isDeniedByRule(sessionId: string, toolName: string): boolean {
  return useDataStudioStore().findConfirmationRule(sessionId, toolName)?.action === 'deny'
}

async function initAgentRuntime(): Promise<void> {
  if (runtimeInitialized)
    return
  runtimeDisposed = false

  const unlisteners: Array<() => void> = []

  unlisteners.push(
    await agentApi.onAgentLoopDelta(({ session_id, content }) => {
      const store = useDataStudioStore()
      store.removePreparingPlaceholder()
      const session = getSessionById(store.sessions, session_id)
      if (!session)
        return

      const streamingMsg = [...session.messages]
        .reverse()
        .find(message => message.role === 'assistant' && message.status === 'streaming')

      if (streamingMsg) {
        store.updateStreamingContent(streamingMsg.id, content, session_id)
        return
      }

      const lastAssistant = [...session.messages]
        .reverse()
        .find(message => message.role === 'assistant')
      const hasUnresolvedTools = lastAssistant?.toolCalls?.some(
        toolCall => toolCall.status === 'executing' || toolCall.status === 'pending',
      )

      if (!hasUnresolvedTools) {
        store.addMessage({
          id: ulid(),
          role: 'assistant',
          content,
          status: 'streaming',
          timestamp: Date.now(),
        }, session_id)
      }
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopThinkingDelta(({ session_id, content }) => {
      const store = useDataStudioStore()
      store.removePreparingPlaceholder()
      const session = getSessionById(store.sessions, session_id)
      if (!session)
        return

      const streamingMsg = [...session.messages]
        .reverse()
        .find(message => message.role === 'assistant' && message.status === 'streaming')

      if (streamingMsg) {
        store.updateStreamingThinking(streamingMsg.id, content, session_id)
        return
      }

      store.addMessage({
        id: ulid(),
        role: 'assistant',
        content: '',
        thinking: content,
        status: 'streaming',
        timestamp: Date.now(),
      }, session_id)
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopToolCall(({ session_id, tool_call_id, tool_name, arguments: args }) => {
      const store = useDataStudioStore()
      store.removePreparingPlaceholder()
      const session = getSessionById(store.sessions, session_id)
      if (!session)
        return

      const runtime = getSessionRuntime(session_id)
      const riskLevel = (runtime.metadata?.[tool_name]?.riskLevel as AgentToolCall['riskLevel']) ?? 'elevated'
      const needsConfirmation = shouldRequireConfirmation(session, tool_name, riskLevel)
      const denied = isDeniedByRule(session_id, tool_name)

      const toolCall: AgentToolCall = {
        id: tool_call_id,
        toolName: tool_name,
        args: JSON.stringify(args ?? {}),
        status: denied ? 'denied' : needsConfirmation ? 'pending' : 'executing',
        riskLevel,
        requiresConfirmation: needsConfirmation,
      }

      const streamingAssistant = [...session.messages]
        .reverse()
        .find(message => message.role === 'assistant' && message.status === 'streaming')
      const lastAssistant
        = streamingAssistant
          ?? [...session.messages].reverse().find(message => message.role === 'assistant')

      if (lastAssistant) {
        store.setMessageToolCalls(lastAssistant.id, [
          ...(lastAssistant.toolCalls ?? []),
          toolCall,
        ], session_id)
      }
      else {
        store.addMessage({
          id: ulid(),
          role: 'assistant',
          content: '',
          status: 'streaming',
          timestamp: Date.now(),
          toolCalls: [toolCall],
        }, session_id)
      }

      if (denied) {
        agentApi.confirmToolCall(tool_call_id, false).catch(() => undefined)
        return
      }

      if (!needsConfirmation) {
        agentApi.confirmToolCall(tool_call_id, true).catch(() => undefined)
      }
      else {
        store.setSessionStatus(session_id, 'waiting_confirmation')
      }
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopToolResult(({ session_id, tool_call_id, envelope, error }) => {
      const store = useDataStudioStore()
      const session = getSessionById(store.sessions, session_id)
      if (!session)
        return

      const assistantMsg = [...session.messages]
        .reverse()
        .find(
          message =>
            message.role === 'assistant'
            && message.toolCalls?.some(toolCall => toolCall.id === tool_call_id),
        )

      if (assistantMsg) {
        store.updateToolCallStatus(
          assistantMsg.id,
          tool_call_id,
          error ? 'error' : 'done',
          envelope.summary,
          envelope.metadata?.duration_ms,
          session_id,
        )

        if (envelope.full_result) {
          store.toolResultFullBodies = {
            ...store.toolResultFullBodies,
            [tool_call_id]: envelope.full_result,
          }
        }
      }
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopStepDone(({ session_id }) => {
      const store = useDataStudioStore()
      const session = getSessionById(store.sessions, session_id)
      if (!session)
        return

      const streamingMsg = [...session.messages]
        .reverse()
        .find(message => message.role === 'assistant' && message.status === 'streaming')

      if (streamingMsg) {
        store.setMessageStatus(streamingMsg.id, 'done', session_id)
        store.removeOrphanedStreamingMessages(session_id)
      }
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopDone(({ session_id }) => {
      const store = useDataStudioStore()
      const session = getSessionById(store.sessions, session_id)
      if (session) {
        session.messages
          .filter(message => message.role === 'assistant' && message.status === 'streaming')
          .forEach(message => store.setMessageStatus(message.id, 'done', session_id))
      }
      store.setSessionStatus(session_id, 'idle')
      store.removeOrphanedStreamingMessages(session_id)
      store.clearSessionError(session_id)
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopStopped(({ session_id, reason, message }) => {
      const store = useDataStudioStore()
      const normalized = (['iteration_cap', 'wall_clock_budget', 'token_budget', 'llm_error'].includes(reason)
        ? reason
        : 'iteration_cap') as AgentSessionStopReason
      store.setSessionStopped(session_id, normalized, message)
      store.removeOrphanedStreamingMessages(session_id)
      store.clearSessionProgress(session_id)
      store.clearSessionError(session_id)
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopError(({ session_id, error }) => {
      const store = useDataStudioStore()
      const session = getSessionById(store.sessions, session_id)

      store.setSessionStatus(session_id, 'error')
      store.setSessionError(session_id, error)
      store.clearSessionProgress(session_id)

      if (!session)
        return

      const streamingMsgs = session.messages
        .filter(message => message.role === 'assistant' && message.status === 'streaming')

      for (const msg of streamingMsgs)
        store.setMessageStatus(msg.id, 'error', session_id)
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopSummaryInjected((payload) => {
      const store = useDataStudioStore()
      store.replaceCompactionInProgressWithMarker({
        trigger: payload.trigger,
        pre_tokens: payload.pre_tokens,
        post_tokens: payload.post_tokens,
        removed_count: payload.removed_count,
        fallback_keep_pairs: payload.fallback_keep_pairs,
      })
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopIteration(({ session_id, iter_count, max_iterations }) => {
      const store = useDataStudioStore()
      store.setSessionProgress(session_id, {
        phase: 'iterating',
        iter: iter_count,
        maxIter: max_iterations,
      })
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopWaitingLlm(({ session_id, iter_count }) => {
      const store = useDataStudioStore()
      store.removePreparingPlaceholder()
      store.setSessionProgress(session_id, {
        phase: 'waiting_llm',
        iter: iter_count,
      })
      const session = store.sessions.find(s => s.id === session_id)
      if (
        session
        && !session.messages.some(m => m.role === 'assistant' && m.status === 'streaming')
      ) {
        store.addMessage({
          id: ulid(),
          role: 'assistant',
          content: '',
          status: 'streaming',
          timestamp: Date.now(),
        }, session_id)
      }
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopCompacting(({ session_id, phase }) => {
      const store = useDataStudioStore()
      if (phase === 'start') {
        store.removePreparingPlaceholder()
        store.setSessionProgress(session_id, { phase: 'compacting' })
        store.addMessage({
          id: `compacting-${session_id}`,
          role: 'system',
          content: '',
          timestamp: Date.now(),
          status: 'done',
          compactionInProgress: true,
        }, session_id)
      }
      else {
        const existing = store.getSessionProgress(session_id)
        store.setSessionProgress(session_id, {
          phase: 'iterating',
          iter: existing?.iter,
          maxIter: existing?.maxIter,
        })
      }
    }),
  )

  unlisteners.push(
    await agentApi.onAgentLoopWarning(({ session_id, warning }) => {
      console.warn(`[AgentRuntime] Session ${session_id}: ${warning}`)
    }),
  )

  runtimeInitPromise = Promise.resolve().then(() => {
    if (runtimeDisposed) {
      unlisteners.forEach(unlisten => unlisten())
      return
    }
    runtimeUnlisteners = unlisteners
    runtimeInitialized = true
  })

  await runtimeInitPromise
}

function disposeAgentRuntime() {
  if (!runtimeInitialized || runtimeDisposed)
    return
  runtimeDisposed = true
  for (const unlisten of runtimeUnlisteners)
    unlisten()
  runtimeUnlisteners = []
  runtimeInitPromise = null
  sessionRuntimes.clear()
  runtimeInitialized = false
}

export { clearSessionRuntime, disposeAgentRuntime, getSessionRuntime, initAgentRuntime }
