import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { jsonify } from '@/common/jsonify'

export type AgentSession = {
  id: string
  title: string
  status: string
  sources: string
  permissions_mode: string
  model_id: string
  created_at: string
  updated_at: string
}

export type AgentMessage = {
  id: string
  session_id: string
  role: string
  content: string
  created_at: string
}

export type ToolEnvelope = {
  summary: string
  full_result?: string | null
  metadata?: {
    tool_name: string
    duration_ms: number
    truncated: boolean
  }
}

export type ToolDefinition = {
  type: 'function'
  function: {
    name: string
    description: string
    parameters: Record<string, unknown>
  }
}

export type ToolMetadata = {
  riskLevel: string
  requiredPermission: string
}

export type ToolsResponse = {
  tools: ToolDefinition[]
  metadata: Record<string, ToolMetadata>
}

export type AttachedSourceRow = {
  id: string
  kind: string
  alias: string
  name: string
  database_type: string | null
  file_type: string | null
  file_path: string | null
  connection_id: string | null
  created_at: string
  updated_at: string
}

export type ConfirmationRuleRow = {
  id: string
  session_id: string
  tool_name: string
  action: string
  created_at: string
}

export type ContextUsage = {
  session_id: string
  used_tokens: number
  capacity: number
  context_window: number
  output_reserve: number
  trigger_at: number
  should_compact: boolean
  model: string
}

// Commands returning JSON strings from the Rust side (Result<String, String>)
// — these need jsonify.parse after invoke<string>.
const parseStringResult = <T>(raw: string): T => jsonify.parse(raw) as T

export const agentApi = {
  // Tool discovery — returns JSON string
  getAvailableTools: async (sourceKinds?: string[]): Promise<ToolsResponse> => {
    const result = await invoke<string>('get_available_tools', {
      sourceKinds: sourceKinds ?? [],
    })
    return parseStringResult<ToolsResponse>(result)
  },

  getAllTools: async (): Promise<ToolsResponse> => {
    const result = await invoke<string>('get_all_tools')
    return parseStringResult<ToolsResponse>(result)
  },

  // Session CRUD — returns typed structs
  loadAgentSessions: (): Promise<AgentSession[]> =>
    invoke<AgentSession[]>('load_agent_sessions'),

  createAgentSession: async (
    title: string,
    sources?: string,
    permissionsMode?: string,
    modelId?: string | null,
  ): Promise<AgentSession> => {
    const args: Record<string, unknown> = { title }
    if (sources !== undefined)
      args.sources = sources
    if (permissionsMode !== undefined)
      args.permissionsMode = permissionsMode
    if (modelId !== undefined)
      args.modelId = modelId
    return invoke<AgentSession>('create_agent_session', args)
  },

  updateSessionStatus: (sessionId: string, status: string): Promise<void> =>
    invoke('update_session_status', { sessionId, status }),

  updateSessionMeta: async (
    sessionId: string,
    sources?: string,
    permissionsMode?: string,
    modelId?: string | null,
  ): Promise<void> => {
    const args: Record<string, unknown> = { sessionId }
    if (sources !== undefined)
      args.sources = sources
    if (permissionsMode !== undefined)
      args.permissionsMode = permissionsMode
    if (modelId !== undefined)
      args.modelId = modelId
    await invoke('update_session_meta', args)
  },

  deleteAgentSession: (sessionId: string): Promise<void> =>
    invoke('delete_agent_session', { sessionId }),

  clearAgentSessionMessages: (sessionId: string): Promise<void> =>
    invoke('clear_agent_session_messages', { sessionId }),

  loadSessionMessages: (sessionId: string): Promise<AgentMessage[]> =>
    invoke<AgentMessage[]>('load_session_messages', { sessionId }),

  // Agent loop
  runAgentLoop: (sessionId: string, userMessage: string, settings: unknown): Promise<void> =>
    invoke('run_agent_loop', { sessionId, userMessage, settings }),

  cancelAgentLoop: (sessionId: string): Promise<void> =>
    invoke('cancel_agent_loop', { sessionId }),

  confirmToolCall: (toolCallId: string, allowed: boolean): Promise<void> =>
    invoke('confirm_tool_call', { toolCallId, allowed }),

  getToolFullResult: async (toolCallId: string): Promise<string> => {
    const result = await invoke<string>('get_tool_full_result', { toolCallId })
    return parseStringResult<string>(result)
  },

  validateLlmConfig: async (params: {
    provider: string
    apiKey: string
    model: string
    httpProxy?: string
    proxyMode?: string
    baseUrl?: string
  }): Promise<boolean> => {
    const args: Record<string, unknown> = {
      provider: params.provider,
      apiKey: params.apiKey,
      model: params.model,
    }
    if (params.httpProxy !== undefined)
      args.httpProxy = params.httpProxy
    if (params.proxyMode !== undefined)
      args.proxyMode = params.proxyMode
    if (params.baseUrl !== undefined)
      args.baseUrl = params.baseUrl
    const result = await invoke<string>('validate_llm_config', args)
    return parseStringResult<boolean>(result)
  },

  // Confirmation rules — typed struct returns
  loadConfirmationRules: (sessionId: string): Promise<ConfirmationRuleRow[]> =>
    invoke<ConfirmationRuleRow[]>('load_confirmation_rules', { sessionId }),

  saveConfirmationRule: (
    sessionId: string,
    toolName: string,
    action: string,
  ): Promise<ConfirmationRuleRow> =>
    invoke<ConfirmationRuleRow>('save_confirmation_rule', { sessionId, toolName, action }),

  deleteConfirmationRule: (id: string): Promise<void> =>
    invoke('delete_confirmation_rule', { id }),

  clearSessionConfirmationRules: (sessionId: string): Promise<void> =>
    invoke('clear_session_confirmation_rules', { sessionId }),

  // Attached sources — typed struct returns
  loadAttachedSources: (): Promise<AttachedSourceRow[]> =>
    invoke<AttachedSourceRow[]>('load_attached_sources'),

  saveAttachedSource: (
    id: string,
    kind: string,
    alias?: string,
    name?: string,
    databaseType?: string,
    fileType?: string,
    filePath?: string,
    connectionId?: number | null,
  ): Promise<AttachedSourceRow> =>
    invoke<AttachedSourceRow>('save_attached_source', {
      id,
      kind,
      alias,
      name,
      databaseType,
      fileType,
      filePath,
      connectionId,
    }),

  deleteAttachedSource: (id: string): Promise<void> =>
    invoke('delete_attached_source', { id }),

  // Events (return unsubscribe functions)
  onAgentLoopDelta: (
    handler: (payload: { session_id: string, content: string }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-delta', e => handler(e.payload as any)),

  onAgentLoopThinkingDelta: (
    handler: (payload: { session_id: string, content: string }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-thinking-delta', e => handler(e.payload as any)),

  onAgentLoopToolCall: (
    handler: (payload: {
      session_id: string
      tool_call_id: string
      tool_name: string
      arguments: unknown
    }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-tool-call', e => handler(e.payload as any)),

  onAgentLoopToolResult: (
    handler: (payload: {
      session_id: string
      tool_call_id: string
      envelope: ToolEnvelope
      error?: boolean
    }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-tool-result', e => handler(e.payload as any)),

  onAgentLoopStepDone: (
    handler: (payload: { session_id: string }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-step-done', e => handler(e.payload as any)),

  onAgentLoopDone: (
    handler: (payload: { session_id: string }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-done', e => handler(e.payload as any)),

  onAgentLoopStopped: (
    handler: (payload: { session_id: string, reason: string, message: string }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-stopped', e => handler(e.payload as any)),

  onAgentLoopError: (
    handler: (payload: { session_id: string, error: string }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-error', e => handler(e.payload as any)),

  onAgentLoopSummaryInjected: (
    handler: (payload: {
      session_id: string
      trigger: string
      pre_tokens: number
      post_tokens: number
      removed_count: number
      fallback_keep_pairs?: number
    }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-summary-injected', e => handler(e.payload as any)),

  onAgentLoopIteration: (
    handler: (payload: { session_id: string, iter_count: number, max_iterations: number }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-iteration', e => handler(e.payload as any)),

  onAgentLoopWaitingLlm: (
    handler: (payload: { session_id: string, iter_count: number }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-waiting-llm', e => handler(e.payload as any)),

  onAgentLoopCompacting: (
    handler: (payload: { session_id: string, phase: 'start' | 'end' }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-compacting', e => handler(e.payload as any)),

  onAgentLoopWarning: (
    handler: (payload: { session_id: string, warning: string }) => void,
  ): Promise<() => void> =>
    listen('agent-loop-warning', e => handler(e.payload as any)),

  onAgentContextUsage: (
    handler: (payload: {
      session_id: string
      used_tokens: number
      capacity: number
      context_window: number
      output_reserve: number
      trigger_at: number
      should_compact: boolean
      model: string
    }) => void,
  ): Promise<() => void> =>
    listen('agent-context-usage', e => handler(e.payload as any)),

  // Compact — returns JSON string
  compactAgentSession: async (sessionId: string, settings: unknown): Promise<any> => {
    const result = await invoke<string>('compact_agent_session', { sessionId, settings })
    return parseStringResult<any>(result)
  },

  getAgentContextUsage: async (sessionId: string, settings: unknown): Promise<any> => {
    const result = await invoke<string>('get_agent_context_usage', { sessionId, settings })
    return parseStringResult<any>(result)
  },

  // Capability invocation — returns raw string
  invokeCapability: (
    name: string,
    args: Record<string, unknown>,
    connectionId?: string,
  ): Promise<string> =>
    invoke<string>('invoke_capability', { name, args, connectionId }),
}

// Standalone function exports
export const loadAgentSessions = agentApi.loadAgentSessions
export const createAgentSession = agentApi.createAgentSession
export const updateSessionStatus = agentApi.updateSessionStatus
export const updateSessionMeta = agentApi.updateSessionMeta
export const deleteAgentSession = agentApi.deleteAgentSession
export const clearAgentSessionMessages = agentApi.clearAgentSessionMessages
export const loadSessionMessages = agentApi.loadSessionMessages
export const runAgentLoop = agentApi.runAgentLoop
export const cancelAgentLoop = agentApi.cancelAgentLoop
export const confirmToolCall = agentApi.confirmToolCall
export const getToolFullResult = agentApi.getToolFullResult
export const getAvailableTools = agentApi.getAvailableTools
export const getAllTools = agentApi.getAllTools
export const validateLlmConfig = agentApi.validateLlmConfig
export const loadConfirmationRules = agentApi.loadConfirmationRules
export const saveConfirmationRule = agentApi.saveConfirmationRule
export const deleteConfirmationRule = agentApi.deleteConfirmationRule
export const clearSessionConfirmationRules = agentApi.clearSessionConfirmationRules
export const loadAttachedSources = agentApi.loadAttachedSources
export const saveAttachedSource = agentApi.saveAttachedSource
export const deleteAttachedSource = agentApi.deleteAttachedSource
export const compactAgentSession = agentApi.compactAgentSession
export const getAgentContextUsage = agentApi.getAgentContextUsage
export const onAgentContextUsage = agentApi.onAgentContextUsage

export default agentApi
