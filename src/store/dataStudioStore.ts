import type { AgentMessage as AgentApiMessage, AttachedSourceRow, ConfirmationRuleRow } from '@/datasources/agentApi'
import { defineStore } from 'pinia'
import { ulid } from 'ulidx'
import { agentApi } from '@/datasources/agentApi'

// ─── Types ───────────────────────────────────────────────────────────────────

export type DataSourcePermissions = {
  read: boolean
  create: boolean
  update: boolean
  delete: boolean
}

export type PermissionsMode = 'Ask' | 'Auto'

export type SourcePermissionsMode = 'inherit' | 'custom'

export type DatabaseSource = {
  kind: 'database'
  sourceId: string
  connectionId: number
  name: string
  databaseType: 'POSTGRESQL' | 'MYSQL' | 'SQLSERVER' | 'SQLITE' | 'DUCKDB' | 'CLICKHOUSE' | 'ORACLE' | 'DB2' | 'H2' | 'SNOWFLAKE' | 'TRINO' | 'PRESTO' | 'COCKROACHDB' | 'REDSHIFT' | 'MARIADB' | 'TIDB' | 'OCEANBASE' | 'TDSQL'
  permissions: DataSourcePermissions
}

export type AttachedSource = DatabaseSource

export type SessionSource = {
  sourceId: string
  alias: string
  kind: string
  databaseType: string
  permissions: DataSourcePermissions
  permissionsMode: SourcePermissionsMode
  detached?: boolean
  detachedAt?: number
}

export type RiskLevel = 'safe' | 'elevated' | 'destructive'

export type AgentToolCallStatus = 'pending' | 'confirmed' | 'denied' | 'executing' | 'done' | 'error'

export type AgentToolCall = {
  id: string
  toolName: string
  args: string
  status: AgentToolCallStatus
  result?: string
  resultTruncated?: string
  durationMs?: number
  riskLevel: RiskLevel
  requiresConfirmation: boolean
}

export type AgentMessageRole = 'user' | 'assistant' | 'tool' | 'system'

export type AgentMessageStatus = 'pending' | 'streaming' | 'done' | 'error'

export type CompactionMarker = {
  summary: string
  preTokens: number
  postTokens: number
  trigger: string
}

export type CompactionMarkerInsertPayload = {
  trigger: string
  pre_tokens: number
  post_tokens: number
  removed_count: number
  fallback_keep_pairs?: number
}

export type AgentMessage = {
  id: string
  role: AgentMessageRole
  content: string
  thinking?: string
  thinkingDuration?: number
  status: AgentMessageStatus
  toolCalls?: AgentToolCall[]
  toolCallId?: string
  timestamp: number
  compaction?: CompactionMarker
  compactionInProgress?: boolean
  preparingInProgress?: boolean
}

export type AgentSessionStatus = 'idle' | 'running' | 'waiting_confirmation' | 'error' | 'stopped'

export type AgentSessionStopReason = 'iteration_cap' | 'wall_clock_budget' | 'token_budget' | 'llm_error'

export type AgentSession = {
  id: string
  title: string
  sources: SessionSource[]
  permissionsMode: PermissionsMode
  messages: AgentMessage[]
  status: AgentSessionStatus
  stopReason?: AgentSessionStopReason
  stopMessage?: string
  updated_at: number
  created_at: number
  model_id: string
}

export type ConfirmationRule = {
  id?: string
  sessionId: string
  toolName: string
  action: 'allow' | 'deny'
}

export type SessionProgressPhase = 'idle' | 'preparing' | 'iterating' | 'waiting_llm' | 'compacting'

export type SessionProgress = {
  phase: SessionProgressPhase
  iter?: number
  maxIter?: number
  updatedAt: number
}

// ─── Backend Type Mappers ─────────────────────────────────────────────────────

function fromAttachedSourceRow(row: AttachedSourceRow): AttachedSource {
  return {
    kind: 'database',
    sourceId: row.id,
    connectionId: row.connection_id ? Number(row.connection_id) : 0,
    name: row.alias || row.name,
    databaseType: (row.database_type as DatabaseSource['databaseType']) ?? 'POSTGRESQL',
    permissions: { read: true, create: false, update: false, delete: false },
  }
}

function toAttachedSourceFields(source: AttachedSource): Parameters<typeof agentApi.saveAttachedSource> {
  const fields: Parameters<typeof agentApi.saveAttachedSource> = [
    source.sourceId,
    'database',
    source.name,
    source.name,
    source.databaseType,
    undefined,
    undefined,
    source.connectionId,
  ]
  return fields
}

async function saveAttachedSourceWithFields(source: AttachedSource) {
  await agentApi.saveAttachedSource(...toAttachedSourceFields(source))
}

function fromBackendMessage(row: AgentApiMessage): AgentMessage {
  const base = {
    id: row.id,
    role: row.role as AgentMessageRole,
    status: 'done' as AgentMessageStatus,
    timestamp: new Date(row.created_at).getTime(),
  }

  if (row.role === 'system') {
    try {
      const parsed = JSON.parse(row.content) as {
        _compact_boundary?: boolean
        summary?: string
        pre_tokens?: number
        post_tokens?: number
        trigger?: string
      }
      if (parsed?._compact_boundary) {
        return {
          ...base,
          content: '',
          compaction: {
            summary: parsed.summary ?? '',
            preTokens: parsed.pre_tokens ?? 0,
            postTokens: parsed.post_tokens ?? 0,
            trigger: (parsed.trigger === 'manual' ? 'manual' : 'auto') as CompactionMarker['trigger'],
          },
        }
      }
    }
    catch { /* fall through */ }
    return { ...base, content: row.content }
  }

  if (row.role === 'tool') {
    try {
      const parsed = JSON.parse(row.content) as { content?: string }
      return { ...base, content: parsed.content ?? row.content }
    }
    catch { /* fall through */ }
    return { ...base, content: row.content }
  }

  if (row.role === 'user') {
    return { ...base, content: row.content }
  }

  // assistant — content is JSON with { content, thinking, tool_calls }
  try {
    const parsed = JSON.parse(row.content) as {
      content?: string | null
      thinking?: string | null
      tool_calls?: Array<{ id: string; function: { name: string; arguments: string } }> | null
    }
    if (
      typeof parsed === 'object' &&
      parsed !== null &&
      ('content' in parsed || 'thinking' in parsed || 'tool_calls' in parsed)
    ) {
      const toolCalls: AgentToolCall[] = (parsed.tool_calls ?? []).map(tc => ({
        id: tc.id,
        toolName: tc.function?.name ?? '',
        args: tc.function?.arguments ?? '{}',
        status: 'done' as AgentToolCallStatus,
        riskLevel: 'safe' as RiskLevel,
        requiresConfirmation: false,
      }))
      return {
        ...base,
        content: parsed.content ?? '',
        thinking: parsed.thinking ?? undefined,
        toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
      }
    }
  }
  catch {
    const isLlmError = row.role === 'assistant' && row.content.startsWith('LLM HTTP ')
    return { ...base, status: isLlmError ? 'error' : 'done', content: row.content }
  }

  const isLlmError = row.role === 'assistant' && row.content.startsWith('LLM HTTP ')
  return { ...base, status: isLlmError ? 'error' : 'done', content: row.content }
}

const dedupAdjacentCompactions = (messages: AgentMessage[]): AgentMessage[] => {
  let found = false
  return messages.reduceRight((acc, m) => {
    if (!m.compaction || !found) {
      if (m.compaction) found = true
      acc.unshift(m)
    }
    return acc
  }, [] as AgentMessage[])
}

function fromConfirmationRuleRow(row: ConfirmationRuleRow): ConfirmationRule {
  return {
    id: row.id,
    sessionId: row.session_id,
    toolName: row.tool_name,
    action: row.action as 'allow' | 'deny',
  }
}

// ─── State ───────────────────────────────────────────────────────────────────

type DataStudioStoreState = {
  attachedSources: AttachedSource[]
  sessions: AgentSession[]
  activeSessionId: string | undefined
  sidebarSessionId: string | undefined
  confirmationRules: ConfirmationRule[]
  toolResultFullBodies: Record<string, string>
  sessionErrors: Record<string, string>
  sessionProgress: Record<string, SessionProgress>
}

// ─── Store ───────────────────────────────────────────────────────────────────

export const useDataStudioStore = defineStore('dataStudio', {
  state: (): DataStudioStoreState => ({
    attachedSources: [],
    sessions: [],
    activeSessionId: undefined,
    sidebarSessionId: undefined,
    confirmationRules: [],
    toolResultFullBodies: {},
    sessionErrors: {},
    sessionProgress: {},
  }),
  persist: {
    pick: ['activeSessionId', 'sidebarSessionId'],
  },

  getters: {
    activeSession: (state): AgentSession | undefined =>
      state.sessions.find(s => s.id === state.activeSessionId),

    activeSidebarSession: (state): AgentSession | undefined =>
      state.sessions.find(s => s.id === state.sidebarSessionId),

    getSessionProgress: state => (sessionId: string): SessionProgress | undefined =>
      state.sessionProgress[sessionId],
  },

  actions: {
    // ── Source Management ──────────────────────────────────────────────────

    async addAttachedSource(source: AttachedSource) {
      await saveAttachedSourceWithFields(source)
      this.attachedSources = [...this.attachedSources, source]
    },

    async updateAttachedSource(source: AttachedSource) {
      await saveAttachedSourceWithFields(source)
      this.attachedSources = this.attachedSources.map(s =>
        s.sourceId === source.sourceId ? source : s,
      )
    },

    async removeAttachedSource(sourceId: string) {
      await agentApi.deleteAttachedSource(sourceId)
      this.attachedSources = this.attachedSources.filter(s => s.sourceId !== sourceId)
    },

    getAttachedSourceById(sourceId: string): AttachedSource | undefined {
      return this.attachedSources.find(s => s.sourceId === sourceId)
    },

    async loadAttachedSourcesFromDb() {
      try {
        const rows = await agentApi.loadAttachedSources()
        this.attachedSources = rows.map(fromAttachedSourceRow)
      }
      catch (error) {
        console.error('Failed to load attached sources:', error)
        this.attachedSources = []
      }
    },

    async addDatabaseSourceFromConnection(params: {
      connectionId: number
      name: string
      databaseType: DatabaseSource['databaseType']
      permissions: DataSourcePermissions
    }): Promise<DatabaseSource> {
      const existing = this.attachedSources.find(
        s => s.kind === 'database' && (s as DatabaseSource).connectionId === params.connectionId,
      ) as DatabaseSource | undefined
      if (existing) {
        const updated = { ...existing, permissions: params.permissions }
        this.attachedSources = this.attachedSources.map(s =>
          s.sourceId === existing.sourceId ? updated : s,
        )
        return updated
      }

      const source: DatabaseSource = {
        kind: 'database',
        sourceId: ulid(),
        ...params,
      }
      await this.addAttachedSource(source)
      return source
    },

    attachSourceToActiveSession(sourceId: string) {
      const session = this.activeSession
      if (!session)
        return

      const source = this.attachedSources.find(s => s.sourceId === sourceId)
      if (!source)
        return

      const alreadyAttached = session.sources.some(s => s.sourceId === sourceId)
      if (alreadyAttached)
        return

      const sessionSource: SessionSource = {
        sourceId: source.sourceId,
        alias: source.name,
        kind: source.kind,
        databaseType: source.databaseType,
        permissions: { ...source.permissions },
        permissionsMode: 'inherit',
      }

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? { ...s, sources: [...s.sources, sessionSource], updated_at: Date.now() }
          : s,
      )
    },

    detachSourceFromSession(sourceId: string) {
      const session = this.activeSession
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              sources: s.sources.map(src =>
                src.sourceId === sourceId
                  ? { ...src, detached: true, detachedAt: Date.now() }
                  : src,
              ),
              updated_at: Date.now(),
            }
          : s,
      )
    },

    setSessionPermissionsMode(mode: PermissionsMode) {
      const session = this.activeSession
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? { ...s, permissionsMode: mode, updated_at: Date.now() }
          : s,
      )
    },

    updateSessionSourcePermissions(sourceId: string, permissions: DataSourcePermissions) {
      const session = this.activeSession
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              sources: s.sources.map(src =>
                src.sourceId === sourceId
                  ? { ...src, permissions: { ...permissions } }
                  : src,
              ),
              updated_at: Date.now(),
            }
          : s,
      )
    },

    updateSessionSourceMode(sourceId: string, permissionsMode: SourcePermissionsMode) {
      const session = this.activeSession
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              sources: s.sources.map(src =>
                src.sourceId === sourceId
                  ? { ...src, permissionsMode }
                  : src,
              ),
              updated_at: Date.now(),
            }
          : s,
      )
    },

    // ── Session Management ─────────────────────────────────────────────────

    async createSession(params?: { title?: string, model_id?: string }): Promise<AgentSession> {
      // Use backend-generated ID to stay in sync with the server
      const row = await agentApi.createAgentSession(
        params?.title ?? 'New Chat',
        undefined,
        'Ask',
        params?.model_id ?? null,
      )

      const session: AgentSession = {
        id: row.id, // Use the backend's UUID
        title: row.title,
        sources: [],
        permissionsMode: 'Ask',
        messages: [],
        status: 'idle',
        updated_at: Number(row.updated_at),
        created_at: Number(row.created_at),
        model_id: row.model_id ?? '',
      }

      this.sessions = [...this.sessions, session]
      this.activeSessionId = session.id
      return session
    },

    setActiveSession(sessionId: string) {
      this.activeSessionId = sessionId
    },

    async getOrCreateSession(sessionId?: string): Promise<AgentSession> {
      if (sessionId) {
        const existing = this.sessions.find(s => s.id === sessionId)
        if (existing) {
          this.activeSessionId = sessionId
          return existing
        }
      }
      return await this.createSession()
    },

    async getOrCreateSidebarSession(): Promise<string> {
      if (this.sidebarSessionId && this.sessions.some(s => s.id === this.sidebarSessionId))
        return this.sidebarSessionId

      const row = await agentApi.createAgentSession('AI Assistant', undefined, 'Ask', null)
      const session: AgentSession = {
        id: row.id,
        title: row.title,
        sources: [],
        permissionsMode: 'Ask',
        messages: [],
        status: 'idle',
        updated_at: Number(row.updated_at),
        created_at: Number(row.created_at),
        model_id: row.model_id ?? '',
      }
      this.sessions = [session, ...this.sessions]
      this.sidebarSessionId = session.id
      return session.id
    },

    // ── Messages ───────────────────────────────────────────────────────────

    /** Find session by ID, or fall back to activeSession. */
    _resolveSession(sessionId?: string): AgentSession | undefined {
      if (sessionId)
        return this.sessions.find(s => s.id === sessionId)
      return this.activeSession
    },

    addMessage(message: Omit<AgentMessage, 'id' | 'timestamp'> & { id?: string, timestamp?: number }, sessionId?: string): AgentMessage {
      const session = this._resolveSession(sessionId)
      if (!session)
        throw new Error('No active session')

      const msg: AgentMessage = {
        id: ulid(),
        timestamp: Date.now(),
        ...message,
      }

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? { ...s, messages: [...s.messages, msg], updated_at: Date.now() }
          : s,
      )
      return msg
    },

    updateStreamingContent(messageId: string, chunk: string, sessionId?: string) {
      const session = this._resolveSession(sessionId)
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              messages: s.messages.map(m =>
                m.id === messageId ? { ...m, content: m.content + chunk } : m,
              ),
            }
          : s,
      )
    },

    updateStreamingThinking(messageId: string, chunk: string, sessionId?: string) {
      const session = this._resolveSession(sessionId)
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              messages: s.messages.map(m =>
                m.id === messageId ? { ...m, thinking: (m.thinking ?? '') + chunk } : m,
              ),
            }
          : s,
      )
    },

    setMessageStatus(messageId: string, status: AgentMessageStatus, sessionId?: string) {
      const session = this._resolveSession(sessionId)
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              messages: s.messages.map((m) => {
                if (m.id !== messageId)
                  return m
                const thinkingDuration
                  = status === 'done' && m.thinking && m.status === 'streaming'
                    ? Math.round((Date.now() - m.timestamp) / 1000)
                    : m.thinkingDuration
                return { ...m, status, thinkingDuration }
              }),
            }
          : s,
      )
    },

    setMessageToolCalls(messageId: string, toolCalls: AgentToolCall[], sessionId?: string) {
      const session = this._resolveSession(sessionId)
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              messages: s.messages.map(m =>
                m.id === messageId ? { ...m, toolCalls } : m,
              ),
            }
          : s,
      )
    },

    updateToolCallStatus(messageId: string, toolCallId: string, status: AgentToolCallStatus, result?: string, durationMs?: number, sessionId?: string) {
      const session = this._resolveSession(sessionId)
      if (!session)
        return

      const TOOL_RESULT_PREVIEW_CHARS = 200
      let storedResult = result
      let resultTruncated = false
      let resultFullLength: number | undefined
      if (result !== undefined && result.length > TOOL_RESULT_PREVIEW_CHARS) {
        this.toolResultFullBodies = { ...this.toolResultFullBodies, [toolCallId]: result }
        storedResult = result.slice(0, TOOL_RESULT_PREVIEW_CHARS)
        resultTruncated = true
        resultFullLength = result.length
      }

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              messages: s.messages.map(m =>
                m.id === messageId
                  ? {
                      ...m,
                      toolCalls: m.toolCalls?.map(tc =>
                        tc.id === toolCallId
                          ? {
                              ...tc,
                              status,
                              ...(storedResult !== undefined ? { result: storedResult } : {}),
                              ...(resultTruncated ? { resultTruncated: `Showing first ${TOOL_RESULT_PREVIEW_CHARS} of ${resultFullLength} characters` } : {}),
                              ...(durationMs !== undefined ? { durationMs } : {}),
                            }
                          : tc,
                      ),
                    }
                  : m,
              ),
            }
          : s,
      )
    },

    removeOrphanedStreamingMessages(sessionId?: string) {
      const session = this._resolveSession(sessionId)
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              messages: s.messages.filter(
                m => !(m.status === 'streaming' && !m.content && !m.toolCalls?.length),
              ),
            }
          : s,
      )
    },

    // ── Compaction ─────────────────────────────────────────────────────────

    insertCompactionMarker(marker: CompactionMarker) {
      const session = this.activeSession
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              messages: [
                ...s.messages,
                {
                  id: ulid(),
                  role: 'system' as const,
                  content: '',
                  status: 'done' as const,
                  timestamp: Date.now(),
                  compaction: marker,
                },
              ],
              updated_at: Date.now(),
            }
          : s,
      )
    },

    replaceCompactionInProgressWithMarker(payload: CompactionMarkerInsertPayload) {
      const session = this.activeSession
      if (!session)
        return

      const marker: CompactionMarker = {
        summary: `Compacted ${payload.removed_count} messages`,
        preTokens: payload.pre_tokens,
        postTokens: payload.post_tokens,
        trigger: payload.trigger,
      }

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              messages: [
                ...s.messages.filter(m => !m.compactionInProgress),
                {
                  id: ulid(),
                  role: 'system' as const,
                  content: '',
                  status: 'done' as const,
                  timestamp: Date.now(),
                  compaction: marker,
                },
              ],
              updated_at: Date.now(),
            }
          : s,
      )
    },

    // ── Placeholders ───────────────────────────────────────────────────────

    insertPreparingPlaceholder() {
      const session = this.activeSession
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              messages: [
                ...s.messages,
                {
                  id: ulid(),
                  role: 'assistant' as const,
                  content: '',
                  status: 'pending' as const,
                  timestamp: Date.now(),
                  preparingInProgress: true,
                },
              ],
              updated_at: Date.now(),
            }
          : s,
      )
    },

    removePreparingPlaceholder() {
      const session = this.activeSession
      if (!session)
        return

      this.sessions = this.sessions.map(s =>
        s.id === session.id
          ? {
              ...s,
              messages: s.messages.filter(m => !m.preparingInProgress),
              updated_at: Date.now(),
            }
          : s,
      )
    },

    // ── Progress ───────────────────────────────────────────────────────────

    setSessionProgress(sessionId: string, progress: Omit<SessionProgress, 'updatedAt'>) {
      this.sessionProgress = {
        ...this.sessionProgress,
        [sessionId]: { ...progress, updatedAt: Date.now() },
      }
    },

    clearSessionProgress(sessionId: string) {
      const { [sessionId]: _removed, ...rest } = this.sessionProgress
      this.sessionProgress = rest
    },

    // ── Status ─────────────────────────────────────────────────────────────

    setSessionStatus(sessionId: string, status: AgentSessionStatus) {
      this.sessions = this.sessions.map(s =>
        s.id === sessionId ? { ...s, status, updated_at: Date.now() } : s,
      )
      if (status !== 'error') {
        const { [sessionId]: _removed, ...rest } = this.sessionErrors
        this.sessionErrors = rest
      }
      if (status === 'idle' || status === 'stopped' || status === 'error') {
        this.clearSessionProgress(sessionId)
        this.removePreparingPlaceholder()
      }
    },

    setSessionError(sessionId: string, error: string) {
      this.sessionErrors = { ...this.sessionErrors, [sessionId]: error }
    },

    clearSessionError(sessionId: string) {
      const { [sessionId]: _removed, ...rest } = this.sessionErrors
      this.sessionErrors = rest
    },

    getSessionError(sessionId: string): string | undefined {
      return this.sessionErrors[sessionId]
    },

    setSessionStopped(sessionId: string, reason: AgentSessionStopReason, message?: string) {
      this.sessions = this.sessions.map(s =>
        s.id === sessionId
          ? { ...s, status: 'stopped', stopReason: reason, stopMessage: message, updated_at: Date.now() }
          : s,
      )
    },

    clearSessionStop(sessionId: string) {
      this.sessions = this.sessions.map(s =>
        s.id === sessionId
          ? { ...s, stopReason: undefined, stopMessage: undefined, updated_at: Date.now() }
          : s,
      )
    },

    async setSessionModelId(sessionId: string, modelId: string) {
      this.sessions = this.sessions.map(s =>
        s.id === sessionId ? { ...s, model_id: modelId, updated_at: Date.now() } : s,
      )
      await agentApi.updateSessionMeta(sessionId, undefined, undefined, modelId).catch(e =>
        console.warn('[persist] updateSessionMeta failed:', e),
      )
    },

    // ── Data Loading ───────────────────────────────────────────────────────

    async reloadSessionMessages(sessionId: string) {
      try {
        const rows = await agentApi.loadSessionMessages(sessionId)
        const messages = dedupAdjacentCompactions(rows.map(fromBackendMessage))
        this.sessions = this.sessions.map(s =>
          s.id === sessionId ? { ...s, messages, updated_at: Date.now() } : s,
        )
      }
      catch (error) {
        console.error(`Failed to reload messages for session ${sessionId}:`, error)
      }
    },

    async loadConfirmationRulesFromDb(sessionId: string) {
      try {
        const rows = await agentApi.loadConfirmationRules(sessionId)
        this.confirmationRules = rows.map(fromConfirmationRuleRow)
      }
      catch (error) {
        console.error('Failed to load confirmation rules:', error)
        this.confirmationRules = []
      }
    },

    findConfirmationRule(sessionId: string, toolName: string): ConfirmationRule | undefined {
      return this.confirmationRules.find(
        r => r.sessionId === sessionId && r.toolName === toolName,
      )
    },

    async addConfirmationRule(rule: ConfirmationRule) {
      const row = await agentApi.saveConfirmationRule(
        rule.sessionId,
        rule.toolName,
        rule.action,
      )
      this.confirmationRules = [...this.confirmationRules, fromConfirmationRuleRow(row)]
    },

    async removeConfirmationRule(ruleId: string) {
      await agentApi.deleteConfirmationRule(ruleId)
      this.confirmationRules = this.confirmationRules.filter(r => r.id !== ruleId)
    },

    // ── Session Lifecycle ──────────────────────────────────────────────────

    async clearSession(sessionId: string) {
      await agentApi.clearAgentSessionMessages(sessionId)
      const session = this.sessions.find(s => s.id === sessionId)
      const toolCallIds = (session?.messages ?? []).flatMap(m =>
        (m.toolCalls ?? []).map(tc => tc.id),
      )
      this.sessions = this.sessions.map(s =>
        s.id === sessionId
          ? { ...s, messages: [], status: 'idle', stopReason: undefined, stopMessage: undefined, updated_at: Date.now() }
          : s,
      )
      this.clearSessionError(sessionId)
      if (toolCallIds.length > 0) {
        const next = { ...this.toolResultFullBodies }
        toolCallIds.forEach(id => delete next[id])
        this.toolResultFullBodies = next
      }
    },

    async loadSessions() {
      try {
        const backendSessions = await agentApi.loadAgentSessions()
        const loaded = await Promise.all(
          backendSessions.map(async (s) => {
            const backendMessages = await agentApi.loadSessionMessages(s.id).catch(
              () => [] as AgentApiMessage[],
            )
            const messages: AgentMessage[] = dedupAdjacentCompactions(
              backendMessages.map(fromBackendMessage),
            )
            const sources: SessionSource[] = (() => {
              try { return s.sources ? JSON.parse(s.sources) : [] }
              catch { return [] }
            })()
            return {
              id: s.id,
              title: s.title,
              sources,
              permissionsMode: (s.permissions_mode as PermissionsMode) ?? 'Ask',
              messages,
              status: (s.status as AgentSessionStatus) ?? 'idle',
              updated_at: new Date(s.updated_at).getTime(),
              created_at: new Date(s.created_at).getTime(),
              model_id: s.model_id ?? '',
            } satisfies AgentSession
          }),
        )
        this.sessions = loaded.filter(s => s.id !== '')
        if (backendSessions.length > 0) {
          const stillValid =
            this.activeSessionId && this.sessions.some(s => s.id === this.activeSessionId)
          if (!stillValid) {
            this.activeSessionId = this.sessions[0]?.id ?? undefined
          }
        }
      }
      catch (error) {
        console.error('Failed to load sessions:', error)
        this.sessions = []
      }
    },

    async removeSession(sessionId: string) {
      await agentApi.deleteAgentSession(sessionId)
      this.sessions = this.sessions.filter(s => s.id !== sessionId)
      if (this.activeSessionId === sessionId) {
        this.activeSessionId = this.sessions[0]?.id ?? undefined
      }
      if (this.sidebarSessionId === sessionId) {
        this.sidebarSessionId = undefined
      }
    },
  },
})
