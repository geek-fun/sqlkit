import type { ToolDefinition, ToolMetadata } from '@/datasources/agentApi'
import type {
  AgentSession,
  AgentSessionStopReason,
  AgentToolCall,
  AgentToolCallStatus,
  PermissionsMode,
  RiskLevel,
} from '@/store/dataStudioStore'
import { agentApi } from '@/datasources/agentApi'
import { useDataStudioStore } from '@/store/dataStudioStore'

// ─── Types ───────────────────────────────────────────────────────────────────

type SessionRuntime = {
  tools?: Array<ToolDefinition>
  metadata?: Record<string, ToolMetadata>
}

// ─── Module-level State ──────────────────────────────────────────────────────

const sessionRuntimes = new Map<string, SessionRuntime>()
let runtimeInitPromise: Promise<void> | null = null
let runtimeInitialized = false
let runtimeDisposed = false
let runtimeUnlisteners: Array<() => void> = []

// ─── Helpers ─────────────────────────────────────────────────────────────────

function hasStreamingMessage(session: AgentSession): string | undefined {
  const messages = session.messages
  for (let i = messages.length - 1; i >= 0; i--) {
    if (messages[i].status === 'streaming')
      return messages[i].id
  }
  return undefined
}

function hasPendingMessage(session: AgentSession): string | undefined {
  const messages = session.messages
  for (let i = messages.length - 1; i >= 0; i--) {
    if (messages[i].status === 'pending')
      return messages[i].id
  }
  return undefined
}

function findMessageByToolCallId(session: AgentSession, toolCallId: string): string | undefined {
  const messages = session.messages
  for (let i = messages.length - 1; i >= 0; i--) {
    if (messages[i].toolCalls?.some(tc => tc.id === toolCallId))
      return messages[i].id
  }
  return undefined
}

function getToolRiskLevel(runtime: SessionRuntime | undefined, toolName: string): RiskLevel {
  const risk = runtime?.metadata?.[toolName]?.riskLevel
  if (risk === 'destructive')
    return 'destructive'
  if (risk === 'elevated')
    return 'elevated'
  return 'safe'
}

function shouldRequireConfirmation(mode: PermissionsMode, risk: RiskLevel): boolean {
  return !(mode === 'Auto' && risk === 'safe')
}

function isDeniedByRule(rule: { action: string } | undefined): boolean {
  return rule?.action === 'deny'
}

function getPendingConfirmation(session: AgentSession): boolean {
  return session.status === 'waiting_confirmation'
}

// ─── Runtime Management ──────────────────────────────────────────────────────

export function getSessionRuntime(sessionId: string): SessionRuntime {
  let runtime = sessionRuntimes.get(sessionId)
  if (!runtime) {
    runtime = {}
    sessionRuntimes.set(sessionId, runtime)
  }
  return runtime
}

export function clearSessionRuntime(sessionId: string): void {
  sessionRuntimes.delete(sessionId)
}

export async function initAgentRuntime(): Promise<void> {
  if (runtimeInitialized)
    return

  // Reset disposed flag to allow re-initialization after disposeAgentRuntime()
  runtimeDisposed = false

  const init = async () => {
    const unlisteners: Array<() => void> = []

    // ── Streaming content delta ─────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopDelta((payload) => {
        const store = useDataStudioStore()
        const session = store.activeSession
        if (!session)
          return

        const messageId = hasStreamingMessage(session) ?? hasPendingMessage(session)
        if (!messageId)
          return

        const message = session.messages.find(m => m.id === messageId)
        if (message?.status === 'pending')
          store.setMessageStatus(messageId, 'streaming')

        store.updateStreamingContent(messageId, payload.content)
        store.removePreparingPlaceholder()
      }),
    )

    // ── Streaming thinking delta ────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopThinkingDelta((payload) => {
        const store = useDataStudioStore()
        const session = store.activeSession
        if (!session)
          return

        const messageId = hasStreamingMessage(session) ?? hasPendingMessage(session)
        if (!messageId)
          return

        const message = session.messages.find(m => m.id === messageId)
        if (message?.status === 'pending')
          store.setMessageStatus(messageId, 'streaming')

        store.updateStreamingThinking(messageId, payload.content)
      }),
    )

    // ── Tool call ───────────────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopToolCall((payload) => {
        const store = useDataStudioStore()
        const session = store.activeSession
        if (!session || getPendingConfirmation(session))
          return

        const runtime = getSessionRuntime(payload.session_id)
        const riskLevel = getToolRiskLevel(runtime, payload.tool_name)
        const rule = store.findConfirmationRule(payload.session_id, payload.tool_name)

        const toolCall: AgentToolCall = {
          id: payload.tool_call_id,
          toolName: payload.tool_name,
          args: JSON.stringify(payload.arguments),
          status: 'pending',
          riskLevel,
          requiresConfirmation: false,
        }

        const messageId = hasStreamingMessage(session) ?? hasPendingMessage(session)
        if (messageId) {
          const existing = session.messages.find(m => m.id === messageId)?.toolCalls ?? []
          store.setMessageToolCalls(messageId, [...existing, toolCall])
        }

        if (isDeniedByRule(rule)) {
          agentApi.confirmToolCall(payload.tool_call_id, false)
          if (messageId)
            store.updateToolCallStatus(messageId, payload.tool_call_id, 'denied')
          return
        }

        if (!shouldRequireConfirmation(session.permissionsMode, riskLevel)) {
          agentApi.confirmToolCall(payload.tool_call_id, true)
          if (messageId)
            store.updateToolCallStatus(messageId, payload.tool_call_id, 'confirmed')
          return
        }

        // Confirmation IS required — mark the tool call accordingly so the UI shows the card
        const updatedToolCall: AgentToolCall = { ...toolCall, requiresConfirmation: true }
        if (messageId) {
          const existing = session.messages.find(m => m.id === messageId)?.toolCalls ?? []
          const filtered = existing.filter(tc => tc.id !== payload.tool_call_id)
          store.setMessageToolCalls(messageId, [...filtered, updatedToolCall])
          store.updateToolCallStatus(messageId, payload.tool_call_id, 'pending')
        }
        store.setSessionStatus(payload.session_id, 'waiting_confirmation')
      }),
    )

    // ── Tool result ─────────────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopToolResult((payload) => {
        const store = useDataStudioStore()
        const session = store.activeSession
        if (!session)
          return

        const messageId = findMessageByToolCallId(session, payload.tool_call_id)
        if (!messageId)
          return

        const status: AgentToolCallStatus = payload.error ? 'error' : 'done'
        store.updateToolCallStatus(messageId, payload.tool_call_id, status)

        if (payload.envelope.full_result) {
          store.toolResultFullBodies = {
            ...store.toolResultFullBodies,
            [payload.tool_call_id]: payload.envelope.full_result,
          }
        }
      }),
    )

    // ── Step done ───────────────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopStepDone((_payload) => {
        const store = useDataStudioStore()
        const session = store.activeSession
        if (!session)
          return

        const messageId = hasStreamingMessage(session)
        if (messageId)
          store.setMessageStatus(messageId, 'done')

        store.removeOrphanedStreamingMessages()
      }),
    )

    // ── Session done ────────────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopDone((payload) => {
        const store = useDataStudioStore()
        store.setSessionStatus(payload.session_id, 'idle')
        store.removeOrphanedStreamingMessages()
        store.clearSessionProgress(payload.session_id)
      }),
    )

    // ── Session stopped ─────────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopStopped((payload) => {
        const store = useDataStudioStore()
        store.setSessionStopped(payload.session_id, payload.reason as AgentSessionStopReason, payload.message)
        store.removeOrphanedStreamingMessages()
        store.clearSessionProgress(payload.session_id)
      }),
    )

    // ── Error ───────────────────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopError((payload) => {
        const store = useDataStudioStore()
        store.setSessionStatus(payload.session_id, 'error')
        store.setSessionError(payload.session_id, payload.error)
        store.removeOrphanedStreamingMessages()
        store.clearSessionProgress(payload.session_id)

        const session = store.activeSession
        if (session) {
          const messageId = hasStreamingMessage(session) ?? hasPendingMessage(session)
          if (messageId)
            store.setMessageStatus(messageId, 'error')
        }
      }),
    )

    // ── Compaction marker ───────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopSummaryInjected((payload) => {
        const store = useDataStudioStore()
        store.insertCompactionMarker({
          summary: `Compacted ${payload.removed_count} messages`,
          preTokens: payload.pre_tokens,
          postTokens: payload.post_tokens,
          trigger: payload.trigger,
        })
      }),
    )

    // ── Iteration ───────────────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopIteration((payload) => {
        const store = useDataStudioStore()
        store.setSessionProgress(payload.session_id, {
          phase: 'iterating',
          iter: payload.iter_count,
          maxIter: payload.max_iterations,
        })
      }),
    )

    // ── Waiting for LLM ─────────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopWaitingLlm((payload) => {
        const store = useDataStudioStore()
        store.setSessionProgress(payload.session_id, {
          phase: 'waiting_llm',
          iter: payload.iter_count,
        })
      }),
    )

    // ── Compacting ──────────────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopCompacting((payload) => {
        const store = useDataStudioStore()
        if (payload.phase === 'start') {
          store.setSessionProgress(payload.session_id, { phase: 'compacting' })
        }
        else {
          store.clearSessionProgress(payload.session_id)
        }
      }),
    )

    // ── Warning ─────────────────────────────────────────────────────────────

    unlisteners.push(
      await agentApi.onAgentLoopWarning((payload) => {
        console.warn(`[AgentRuntime] Session ${payload.session_id}: ${payload.warning}`)
      }),
    )

    runtimeUnlisteners = unlisteners
  }

  runtimeInitPromise = init()
  await runtimeInitPromise
  runtimeInitialized = true
}

export function disposeAgentRuntime(): void {
  if (!runtimeInitialized || runtimeDisposed)
    return

  for (const unlisten of runtimeUnlisteners) {
    unlisten()
  }

  runtimeUnlisteners = []
  sessionRuntimes.clear()
  runtimeInitPromise = null
  runtimeInitialized = false
  runtimeDisposed = true
}
