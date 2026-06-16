import type { ComputedRef, Ref } from 'vue'
import type { ToolDefinition, ToolMetadata } from '@/datasources/agentApi'
import type { AgentToolCall, AgentToolCallStatus, ConfirmationRule } from '@/store/dataStudioStore'
import type { ChatContextConfig, ChatMessage, ChatMessageStatus, ChatPermissions, ChatSession, ChatSessionStatus, SendMessageOptions } from '@/types/chat'
import { ulid } from 'ulidx'
import { computed, ref } from 'vue'
import { agentApi } from '@/datasources/agentApi'
import { useDataStudioStore } from '@/store/dataStudioStore'

// ─── Types ───────────────────────────────────────────────────────────────────

export type SessionRuntime = {
  assistantMessageId?: string
  tools: ToolDefinition[]
  toolMetadata: Record<string, ToolMetadata>
}

export type UseChatAgentConfig = {
  feature: 'sidebarAssistant' | 'dataStudio'
  sessionStore: {
    sessions: Ref<Array<ChatSession>>
    activeSessionId: Ref<string | null>
    activeSession: ComputedRef<ChatSession | undefined>
    addMessage: (sessionId: string, message: ChatMessage) => void
    updateStreamingContent: (sessionId: string, messageId: string, chunk: string) => void
    updateStreamingThinking: (sessionId: string, messageId: string, chunk: string) => void
    setMessageStatus: (sessionId: string, messageId: string, status: ChatMessageStatus) => void
    setMessageToolCalls: (sessionId: string, messageId: string, toolCalls: Array<AgentToolCall>) => void
    removeOrphanedStreamingMessages: (sessionId: string, finalizedMessageId: string) => void
    updateToolCallStatus: (messageId: string, toolCallId: string, status: AgentToolCallStatus, result?: string, durationMs?: number, sessionId?: string) => void
    setSessionStatus: (sessionId: string, status: ChatSessionStatus) => void
    setSessionStopped?: (sessionId: string, reason: string, message?: string) => void
    clearSessionStop?: (sessionId: string) => void
    setSessionSchema?: (sessionId: string, schema: string) => void
    clearSession: (sessionId: string) => void
    getOrCreateSession: (sessionId?: string) => string | Promise<string>
    reloadSessionMessages: (sessionId: string) => void
  }
  contextProvider?: () => ChatContextConfig
  confirmationRules?: Ref<Array<ConfirmationRule>>
  addConfirmationRule?: (rule: ConfirmationRule) => void
  findConfirmationRule?: (sessionId: string, toolName: string) => ConfirmationRule | undefined
  autoMode?: Ref<boolean>
}

// ─── Module-level Runtime Cache ──────────────────────────────────────────────

const sessionRuntimes = ref<Record<string, SessionRuntime>>({})

function getSessionRuntime(sessionId: string): SessionRuntime | undefined {
  return sessionRuntimes.value[sessionId]
}

function clearSessionRuntime(sessionId: string) {
  const { [sessionId]: _removed, ...rest } = sessionRuntimes.value
  sessionRuntimes.value = rest
}

// ─── Database-specific Knowledge Blocks ──────────────────────────────────────

type DatabaseKnowledgeMap = Record<string, string[]>

const DATABASE_KNOWLEDGE: DatabaseKnowledgeMap = {
  POSTGRESQL: [
    'PostgreSQL knowledge:',
    '- Uses standard SQL with extensions (JSON/JSONB, arrays, full-text search, PostGIS)',
    '- Supports schemas (public by default), transactions, CTEs, window functions, recursive queries',
    '- String concatenation: || operator (not +)',
    '- ILIKE for case-insensitive matching, SIMILAR TO for regex patterns',
    '- LIMIT/OFFSET for pagination (also supports FETCH FIRST/FETCH NEXT)',
    '- RETURNING clause for INSERT/UPDATE/DELETE to return affected rows',
    '- SERIAL/BIGSERIAL for auto-increment columns; GENERATED AS IDENTITY (preferred)',
    '- ON CONFLICT DO UPDATE/DO NOTHING for upsert behavior',
    '- EXPLAIN ANALYZE for query execution plans',
    '- Supports partial, unique, and expression-based indexes',
    '- DISTINCT ON for deduplication per column',
  ],
  MYSQL: [
    'MySQL knowledge:',
    '- Uses standard SQL with some variations',
    '- Backtick quoting for identifiers: `column_name` (also double quotes in sql_mode)',
    '- LIMIT/OFFSET for pagination',
    '- CONCAT() function for string concatenation (not ||, except when sql_mode=PIPES_AS_CONCAT)',
    '- AUTO_INCREMENT for auto-incrementing columns',
    '- SHOW statements for metadata (SHOW TABLES, SHOW COLUMNS, SHOW CREATE TABLE)',
    '- DESCRIBE table_name for column info',
    '- ENGINE=InnoDB for transactional tables (default)',
    '- REPLACE INTO for upsert (DELETE + INSERT); INSERT ... ON DUPLICATE KEY UPDATE',
    '- LAST_INSERT_ID() to get last auto-increment value',
    '- Variables: @user_variable for session variables',
    '- Common Table Expressions (WITH) supported from MySQL 8.0',
    '- Window functions supported from MySQL 8.0',
  ],
  SQLSERVER: [
    'SQL Server knowledge:',
    '- Uses T-SQL dialect',
    '- Square bracket quoting for identifiers: [column_name]',
    '- TOP n instead of LIMIT (SELECT TOP 10 * FROM table)',
    '- OFFSET/FETCH NEXT for pagination (SQL Server 2012+, requires ORDER BY)',
    '- GETDATE()/GETUTCDATE()/SYSDATETIME() for current datetime',
    '- String concatenation: + operator (NULL + string = NULL, use COALESCE)',
    '- IDENTITY(seed, increment) for auto-incrementing columns',
    '- Supports schemas (dbo by default)',
    '- @@IDENTITY / SCOPE_IDENTITY() for last identity value',
    '- MERGE for upsert operations (INSERT, UPDATE, DELETE in one statement)',
    '- TRY...CATCH for error handling in T-SQL blocks',
    '- WITH (NOLOCK) hint for dirty reads (use with caution)',
    '- CAST and CONVERT for type conversions',
    '- EXEC/EXECUTE for dynamic SQL execution',
    '- Common Table Expressions (WITH) and recursive CTEs supported',
    '- Window functions: ROW_NUMBER(), RANK(), DENSE_RANK(), NTILE()',
    '- STRING_AGG() for string aggregation (SQL Server 2017+)',
  ],
  SQLITE: [
    'SQLite knowledge:',
    '- Lightweight embedded SQL database',
    '- Limited ALTER TABLE support (can only ADD COLUMN or RENAME COLUMN/TO)',
    '- No RIGHT JOIN or FULL OUTER JOIN (use LEFT JOIN with swapped tables)',
    '- Uses dynamic typing (affinity-based type system)',
    '- LIMIT/OFFSET for pagination',
    '- No stored procedures or user-defined functions (except via extensions)',
    '- AUTOINCREMENT keyword for auto-incrementing rowid (INTEGER PRIMARY KEY auto-increments without it)',
    '- PRAGMA statements for configuration (PRAGMA table_info, PRAGMA index_list)',
    '- .dump or .schema for schema export (CLI)',
    '- Supports CTEs (WITH), window functions (from 3.25.0)',
    '- JSON functions supported (from 3.38.0, json_extract, json_set, etc.)',
    '- Strict tables with STRICT keyword (from 3.37.0)',
    '- INSERT OR REPLACE / ON CONFLICT for upsert',
    '- No native LIKE with indices (no ESF) - use full-text search (FTS5) extension',
    '- Concurrency: WAL mode for better concurrent reads',
  ],
}

function buildSQLKnowledgeBlock(dbTypes: string[]): string {
  const seen = new Set<string>()
  const blocks: string[] = []

  for (const dbType of dbTypes) {
    const upper = dbType.toUpperCase()
    if (seen.has(upper))
      continue
    seen.add(upper)

    const knowledge = DATABASE_KNOWLEDGE[upper]
    if (knowledge)
      blocks.push('', ...knowledge)
  }

  return blocks.join('\n')
}

// ─── System Prompt Builder ───────────────────────────────────────────────────

function buildSystemPrompt({ schema, sources, permissionsMode }: {
  schema?: string
  sources: Array<{ connectionId: string, databaseType?: string, permissions?: ChatPermissions }>
  permissionsMode?: string
}): string {
  const parts: string[] = []

  // Core identity
  parts.push(
    'You are a Data Studio AI agent in SQLKit, a cross-platform SQL database GUI client.',
    'Your role is to help users explore, query, and manage their databases using natural language.',
    'You can execute SQL queries, list database objects, describe schemas, and analyze data.',
    '',
  )

  // Source summary
  if (sources.length > 0) {
    parts.push('## Attached Data Sources')
    parts.push('')

    for (const source of sources) {
      const dbType = source.databaseType ?? 'SQL'
      const permParts: string[] = []

      if (source.permissions) {
        if (source.permissions.read)
          permParts.push('read')
        if (source.permissions.create)
          permParts.push('create')
        if (source.permissions.update)
          permParts.push('update')
        if (source.permissions.delete)
          permParts.push('delete')
      }

      const permStr = permParts.length > 0 ? ` (permissions: ${permParts.join(', ')})` : ''
      parts.push(`- ${source.connectionId}: ${dbType}${permStr}`)
    }

    parts.push('')

    // Database-specific knowledge blocks
    const dbTypes = sources.map(s => s.databaseType ?? '').filter(Boolean)
    const knowledgeBlock = buildSQLKnowledgeBlock(dbTypes)
    if (knowledgeBlock) {
      parts.push('## Database-Specific Knowledge')
      parts.push(knowledgeBlock)
      parts.push('')
    }
  }

  // Schema context
  if (schema) {
    parts.push('## Database Schema')
    parts.push('')
    parts.push(schema)
    parts.push('')
  }

  // Mode instructions
  const isAuto = permissionsMode === 'AUTO'
  if (isAuto) {
    parts.push('## Mode: AUTO')
    parts.push(
      'You are in AUTO mode. You may execute queries automatically.',
      'For destructive operations (DROP, TRUNCATE, ALTER, DELETE without WHERE), you must still request confirmation before proceeding.',
    )
  }
  else {
    parts.push('## Mode: ASK')
    parts.push(
      'You are in ASK mode (default). Before executing any query, explain what you plan to do and ask for confirmation.',
      'The user must explicitly approve each operation before you execute it.',
    )
  }
  parts.push('')

  // Session-wide rules
  parts.push('## Rules')
  parts.push(
    '- Never fabricate data, query results, or schema information',
    '- Never use XML tags in your responses (no <thinking>, <result>, etc.)',
    '- Never use emojis in your responses',
    '- Format your responses in Markdown',
    '- For SQL queries, use code blocks with the appropriate language tag',
    '- Be concise and direct; avoid filler language',
    '- If you encounter an error, explain what went wrong and suggest a fix',
    '- Always verify the database type before generating SQL (syntax differs)',
    '- Do not assume table or column names exist - use schema tools to discover them',
    '- When writing SQL, use proper quoting for the target database type',
    '- Explain your reasoning before executing complex operations',
    '- Use LIMIT or TOP when querying tables to avoid large result sets',
  )

  return parts.join('\n')
}

// ─── Composable ──────────────────────────────────────────────────────────────

export function useChatAgent(config: UseChatAgentConfig) {
  const localError = ref<string | undefined>()
  const dataStudioStore = useDataStudioStore()

  const activeSession = computed(() => config.sessionStore.activeSession.value)

  const isLoading = computed(
    () =>
      activeSession.value?.status === 'running'
      || activeSession.value?.status === 'waiting_confirmation',
  )
  const error = computed(
    () =>
      localError.value
      ?? (activeSession.value ? dataStudioStore.getSessionError(activeSession.value.id) : undefined),
  )
  const lastSettings = ref<Record<string, unknown> | null>(null)

  // ── Event Listener Management ────────────────────────────────────────────

  // Event listeners are managed globally by agentRuntime.ts.
  // This composable relies on the global runtime for all event processing.
  const setupLoopEventListeners = async (_sessionId: string): Promise<() => void> => {
    return () => {}
  }

  // ── Core Methods ──────────────────────────────────────────────────────────

  const runAgentLoop = async (sessionId: string, userMessageContent: string, schemaContext?: string) => {
    config.sessionStore.setSessionStatus(sessionId, 'running')

    try {
      // Build system prompt from context
      const context = config.contextProvider?.()
      const sources = context?.connections
        ? Object.entries(context.connections).map(([alias, conn]) => ({
            connectionId: alias,
            databaseType: conn.dbType,
            permissions: conn.permissions,
          }))
        : []

      // Determine mode from autoMode config or permissions mode
      const autoMode = config.autoMode?.value ?? false
      const permissionsMode = autoMode ? 'AUTO' : 'ASK'

      const systemPrompt = buildSystemPrompt({
        schema: schemaContext || context?.schema,
        sources,
        permissionsMode,
      })

      // Get provider+model config from app store (handles dynamic import inline)
      const { useAppStore } = await import('@/store/appStore')
      const appStore = useAppStore()
      const modelConfig = await appStore.getFeatureModelConfig(config.feature)

      // Get runtime tools
      const runtime = getSessionRuntime(sessionId)

      // Build settings payload for the agent loop
      // Include connectionConfig so the Rust loop_runner can resolve DB connections
      const connectionConfig = context?.connections
        ? Object.fromEntries(
            Object.entries(context.connections).map(([alias, conn]) => [
              alias,
              { connectionId: String(conn.connectionId), dbType: conn.dbType, permissions: conn.permissions },
            ]),
          )
        : {}

      const settings: Record<string, unknown> = {
        systemPrompt,
        provider: modelConfig.provider.name,
        model: modelConfig.model.label,
        baseUrl: modelConfig.provider.baseUrl,
        apiKey: modelConfig.provider.apiKey,
        httpProxy: modelConfig.provider.proxy,
        proxyMode: modelConfig.provider.proxyMode ?? 'none',
        tools: runtime?.tools ?? [],
        toolMetadata: runtime?.toolMetadata ?? {},
        maxIterations: activeSession.value?.maxIterations ?? 25,
        attachedSources: sources,
        connectionConfig,
        apiCompatibility: modelConfig.provider.apiCompatibility,
      }

      lastSettings.value = settings

      // Set up event listeners before starting the loop
      const cleanup = await setupLoopEventListeners(sessionId)

      try {
        // Create assistant placeholder message
        const assistantMsg: ChatMessage = {
          id: ulid(),
          role: 'assistant',
          content: '',
          status: 'streaming',
          timestamp: Date.now(),
        }

        config.sessionStore.addMessage(sessionId, assistantMsg)

        // Track the assistant message ID in runtime
        sessionRuntimes.value = {
          ...sessionRuntimes.value,
          [sessionId]: {
            ...runtime,
            assistantMessageId: assistantMsg.id,
            tools: runtime?.tools ?? [],
            toolMetadata: runtime?.toolMetadata ?? {},
          },
        }

        // Start the agent loop
        await agentApi.runAgentLoop(sessionId, userMessageContent, settings)
      }
      finally {
        cleanup()
      }
    }
    catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      localError.value = message
      config.sessionStore.setSessionStatus(sessionId, 'error')

      const errorMsg: ChatMessage = {
        id: ulid(),
        role: 'system',
        content: `Error: ${message}`,
        status: 'error',
        timestamp: Date.now(),
      }
      config.sessionStore.addMessage(sessionId, errorMsg)
    }
  }

  const sendMessage = async (options: SendMessageOptions) => {
    localError.value = undefined

    try {
      const sessionId = await config.sessionStore.getOrCreateSession()
      dataStudioStore.clearSessionError(sessionId)

      if (config.sessionStore.clearSessionStop) {
        config.sessionStore.clearSessionStop(sessionId)
      }

      const userMessage: ChatMessage = {
        id: ulid(),
        role: 'user',
        content: options.content,
        status: 'done',
        timestamp: Date.now(),
      }
      config.sessionStore.addMessage(sessionId, userMessage)

      dataStudioStore.insertPreparingPlaceholder()
      dataStudioStore.setSessionProgress(sessionId, { phase: 'preparing' })

      const context = options.context ?? config.contextProvider?.()
      const connections = context?.connections ?? {}

      const dbTypes = [...new Set(
        Object.values(connections).map(c => c.dbType),
      )].filter(Boolean)

      let toolsResponse
      try {
        toolsResponse = await agentApi.getAvailableTools(
          dbTypes.length > 0 ? dbTypes : undefined,
        )
      }
      catch (err) {
        console.warn('[useChatAgent] Failed to get available tools, continuing without:', err)
        toolsResponse = { tools: [], metadata: {} }
      }

      sessionRuntimes.value = {
        ...sessionRuntimes.value,
        [sessionId]: {
          tools: toolsResponse.tools,
          toolMetadata: toolsResponse.metadata,
        },
      }

      const { useAppStore } = await import('@/store/appStore')
      const appStore = useAppStore()
      try {
        await appStore.getFeatureModelConfig(config.feature)
      }
      catch (err) {
        localError.value = err instanceof Error ? err.message : String(err)
        return
      }

      let schemaContext = ''
      if (connections && Object.keys(connections).length > 0) {
        const schemaParts: string[] = []
        for (const [alias, conn] of Object.entries(connections)) {
          try {
            const schemaResult = await agentApi.invokeCapability('sqlkit__get_schema', {}, String(conn.connectionId))
            if (schemaResult && schemaResult !== '[]' && !schemaResult.startsWith('[')) {
              schemaParts.push(`-- Source: ${alias} (${conn.dbType})`)
              schemaParts.push(schemaResult)
            }
          }
          catch {
            // Schema fetching is best-effort; continue without it
          }
        }
        if (schemaParts.length > 0) {
          schemaContext = schemaParts.join('\n')
        }
      }

      await runAgentLoop(sessionId, options.content, schemaContext)
    }
    catch (err) {
      localError.value = err instanceof Error ? err.message : String(err)
      console.error('[useChatAgent] sendMessage error:', err)
    }
  }

  const cancelSessionHandler = async () => {
    const sessionId = config.sessionStore.activeSessionId.value
    if (!sessionId)
      return
    try {
      await agentApi.cancelAgentLoop(sessionId)
    }
    catch (err) {
      console.error('[useChatAgent] cancelSession error:', err)
    }
  }

  const cancelSession = cancelSessionHandler

  const handleConfirmation = async (
    assistantMsgId: string,
    toolCallId: string,
    action: 'allow_once' | 'allow_always' | 'deny' | 'deny_always' | 'cancel',
  ) => {
    const sessionId = config.sessionStore.activeSessionId.value
    if (!sessionId)
      return

    // Resolve toolName from the tool call in the session
    const resolveToolName = (): string => {
      const session = config.sessionStore.activeSession.value
      if (!session)
        return ''
      const msg = session.messages.find(m => m.id === assistantMsgId)
      if (!msg || !msg.toolCalls)
        return ''
      const tc = msg.toolCalls.find(t => t.id === toolCallId)
      return tc?.toolName ?? ''
    }

    switch (action) {
      case 'allow_once': {
        config.sessionStore.setSessionStatus(sessionId, 'running')
        config.sessionStore.updateToolCallStatus(assistantMsgId, toolCallId, 'confirmed', undefined, undefined, sessionId)
        await agentApi.confirmToolCall(toolCallId, true)
        break
      }

      case 'allow_always': {
        const toolName = resolveToolName()
        const rule: ConfirmationRule = {
          id: ulid(),
          sessionId,
          toolName,
          action: 'allow',
        }
        config.addConfirmationRule?.(rule)
        config.sessionStore.setSessionStatus(sessionId, 'running')
        config.sessionStore.updateToolCallStatus(assistantMsgId, toolCallId, 'confirmed', undefined, undefined, sessionId)
        await agentApi.confirmToolCall(toolCallId, true)
        break
      }

      case 'deny': {
        config.sessionStore.setSessionStatus(sessionId, 'running')
        config.sessionStore.updateToolCallStatus(assistantMsgId, toolCallId, 'denied', undefined, undefined, sessionId)
        await agentApi.confirmToolCall(toolCallId, false)
        break
      }

      case 'deny_always': {
        const toolName = resolveToolName()
        const denyRule: ConfirmationRule = {
          id: ulid(),
          sessionId,
          toolName,
          action: 'deny',
        }
        config.addConfirmationRule?.(denyRule)
        config.sessionStore.setSessionStatus(sessionId, 'running')
        config.sessionStore.updateToolCallStatus(assistantMsgId, toolCallId, 'denied', undefined, undefined, sessionId)
        await agentApi.confirmToolCall(toolCallId, false)
        break
      }

      case 'cancel': {
        // cancelSession is defined below, but the function is hoisted in runtime
        cancelSessionHandler()
        break
      }
    }
  }

  const clearChat = async () => {
    const sessionId = config.sessionStore.activeSessionId.value
    if (!sessionId)
      return

    config.sessionStore.clearSession(sessionId)
    clearSessionRuntime(sessionId)
  }

  const initContextSettings = async () => {
    if (lastSettings.value)
      return
    try {
      const { useAppStore } = await import('@/store/appStore')
      const appStore = useAppStore()
      const { provider, model } = await appStore.getFeatureModelConfig(config.feature)
      lastSettings.value = {
        provider: provider.name,
        apiCompatibility: provider.apiCompatibility,
        model: model.label,
        apiKey: provider.apiKey ?? '',
        baseUrl: provider.baseUrl,
        httpProxy: provider.proxy || undefined,
        proxyMode: provider.proxyMode ?? 'none',
        contextWindowOverride: provider.contextWindowOverride,
      }
    }
    catch {
      lastSettings.value = null
    }
  }

  // ── Return ────────────────────────────────────────────────────────────────

  return {
    isLoading,
    error,
    activeSession,
    lastSettings,
    initContextSettings,
    sendMessage,
    handleConfirmation,
    cancelSession,
    clearChat,
  }
}

export {
  buildSystemPrompt,
  clearSessionRuntime,
  getSessionRuntime,
}
