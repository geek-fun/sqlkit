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

export const agentApi = {
  // Tool discovery
  getAvailableTools: async (sourceKinds?: string[]): Promise<ToolsResponse> => {
    return jsonify.parse(await invoke<string>('get_available_tools', {
      ...(sourceKinds !== undefined ? { sourceKinds } : {}),
    }))
  },

  getAllTools: async (): Promise<ToolsResponse> => {
    return jsonify.parse(await invoke<string>('get_all_tools'))
  },

  // Session CRUD
  loadAgentSessions: async (): Promise<AgentSession[]> => {
    const raw = await invoke<string>('load_agent_sessions')
    return raw ? jsonify.parse(raw) : []
  },

  createAgentSession: async (title: string, sources?: string, permissionsMode?: string, modelId?: string | null): Promise<AgentSession> => {
    return jsonify.parse(await invoke<string>('create_agent_session', {
      title,
      ...(sources !== undefined ? { sources } : {}),
      ...(permissionsMode !== undefined ? { permissionsMode } : {}),
      ...(modelId !== undefined ? { modelId } : {}),
    }))
  },

  updateSessionStatus: async (sessionId: string, status: string): Promise<void> => {
    await invoke<string>('update_session_status', { sessionId, status })
  },

  updateSessionMeta: async (sessionId: string, sources?: string, permissionsMode?: string, modelId?: string | null): Promise<void> => {
    await invoke<string>('update_session_meta', {
      sessionId,
      ...(sources !== undefined ? { sources } : {}),
      ...(permissionsMode !== undefined ? { permissionsMode } : {}),
      ...(modelId !== undefined ? { modelId } : {}),
    })
  },

  deleteAgentSession: async (sessionId: string): Promise<void> => {
    await invoke<string>('delete_agent_session', { sessionId })
  },

  clearAgentSessionMessages: async (sessionId: string): Promise<void> => {
    await invoke<string>('clear_agent_session_messages', { sessionId })
  },

  loadSessionMessages: async (sessionId: string): Promise<AgentMessage[]> => {
    return jsonify.parse(await invoke<string>('load_session_messages', { sessionId }))
  },

  // Agent loop
  runAgentLoop: async (sessionId: string, userMessage: string, settings: unknown): Promise<void> => {
    await invoke<string>('run_agent_loop', { sessionId, userMessage, settings })
  },

  cancelAgentLoop: async (sessionId: string): Promise<void> => {
    await invoke<string>('cancel_agent_loop', { sessionId })
  },

  confirmToolCall: async (toolCallId: string, allowed: boolean): Promise<void> => {
    await invoke<string>('confirm_tool_call', { toolCallId, allowed })
  },

  getToolFullResult: async (toolCallId: string): Promise<string> => {
    return jsonify.parse(await invoke<string>('get_tool_full_result', { toolCallId }))
  },

  validateLlmConfig: async (params: {
    provider: string
    apiKey: string
    model: string
    httpProxy?: string
    proxyMode?: string
    baseUrl?: string
  }): Promise<boolean> => {
    return jsonify.parse(await invoke<string>('validate_llm_config', {
      provider: params.provider,
      apiKey: params.apiKey,
      model: params.model,
      ...(params.httpProxy !== undefined ? { httpProxy: params.httpProxy } : {}),
      ...(params.proxyMode !== undefined ? { proxyMode: params.proxyMode } : {}),
      ...(params.baseUrl !== undefined ? { baseUrl: params.baseUrl } : {}),
    }))
  },

  // Confirmation rules
  loadConfirmationRules: async (sessionId: string): Promise<ConfirmationRuleRow[]> => {
    return jsonify.parse(await invoke<string>('load_confirmation_rules', { sessionId }))
  },

  saveConfirmationRule: async (sessionId: string, toolName: string, action: string): Promise<ConfirmationRuleRow> => {
    return jsonify.parse(await invoke<string>('save_confirmation_rule', { sessionId, toolName, action }))
  },

  deleteConfirmationRule: async (id: string): Promise<void> => {
    await invoke<string>('delete_confirmation_rule', { id })
  },

  clearSessionConfirmationRules: async (sessionId: string): Promise<void> => {
    await invoke<string>('clear_session_confirmation_rules', { sessionId })
  },

  // Attached sources
  loadAttachedSources: async (): Promise<AttachedSourceRow[]> => {
    const raw = await invoke<string>('load_attached_sources')
    return raw ? jsonify.parse(raw) : []
  },

  saveAttachedSource: async (id: string, kind: string, alias?: string, name?: string, databaseType?: string, fileType?: string, filePath?: string, connectionId?: number | null): Promise<AttachedSourceRow> => {
    return jsonify.parse(await invoke<string>('save_attached_source', {
      id,
      kind,
      alias,
      name,
      databaseType,
      fileType,
      filePath,
      connectionId,
    }))
  },

  deleteAttachedSource: async (id: string): Promise<void> => {
    await invoke<string>('delete_attached_source', { id })
  },

  // Events (return unsubscribe functions)
  onAgentLoopDelta: (handler: (payload: { session_id: string, content: string }) => void): Promise<() => void> =>
    listen('agent-loop-delta', e => handler(e.payload as any)),

  onAgentLoopThinkingDelta: (handler: (payload: { session_id: string, content: string }) => void): Promise<() => void> =>
    listen('agent-loop-thinking-delta', e => handler(e.payload as any)),

  onAgentLoopToolCall: (handler: (payload: { session_id: string, tool_call_id: string, tool_name: string, arguments: unknown }) => void): Promise<() => void> =>
    listen('agent-loop-tool-call', e => handler(e.payload as any)),

  onAgentLoopToolResult: (handler: (payload: { session_id: string, tool_call_id: string, envelope: ToolEnvelope, error?: boolean }) => void): Promise<() => void> =>
    listen('agent-loop-tool-result', e => handler(e.payload as any)),

  onAgentLoopStepDone: (handler: (payload: { session_id: string }) => void): Promise<() => void> =>
    listen('agent-loop-step-done', e => handler(e.payload as any)),

  onAgentLoopDone: (handler: (payload: { session_id: string }) => void): Promise<() => void> =>
    listen('agent-loop-done', e => handler(e.payload as any)),

  onAgentLoopStopped: (handler: (payload: { session_id: string, reason: string, message: string }) => void): Promise<() => void> =>
    listen('agent-loop-stopped', e => handler(e.payload as any)),

  onAgentLoopError: (handler: (payload: { session_id: string, error: string }) => void): Promise<() => void> =>
    listen('agent-loop-error', e => handler(e.payload as any)),

  onAgentLoopSummaryInjected: (handler: (payload: { session_id: string, trigger: string, pre_tokens: number, post_tokens: number, removed_count: number, fallback_keep_pairs?: number }) => void): Promise<() => void> =>
    listen('agent-loop-summary-injected', e => handler(e.payload as any)),

  onAgentLoopIteration: (handler: (payload: { session_id: string, iter_count: number, max_iterations: number }) => void): Promise<() => void> =>
    listen('agent-loop-iteration', e => handler(e.payload as any)),

  onAgentLoopWaitingLlm: (handler: (payload: { session_id: string, iter_count: number }) => void): Promise<() => void> =>
    listen('agent-loop-waiting-llm', e => handler(e.payload as any)),

  onAgentLoopCompacting: (handler: (payload: { session_id: string, phase: 'start' | 'end' }) => void): Promise<() => void> =>
    listen('agent-loop-compacting', e => handler(e.payload as any)),

  onAgentLoopWarning: (handler: (payload: { session_id: string, warning: string }) => void): Promise<() => void> =>
    listen('agent-loop-warning', e => handler(e.payload as any)),

  onAgentContextUsage: (handler: (payload: { session_id: string, used_tokens: number, capacity: number, context_window: number, output_reserve: number, trigger_at: number, should_compact: boolean, model: string }) => void): Promise<() => void> =>
    listen('agent-context-usage', e => handler(e.payload as any)),

  // Compact
  compactAgentSession: async (sessionId: string, settings: unknown): Promise<any> => {
    return jsonify.parse(await invoke<string>('compact_agent_session', { sessionId, settings }))
  },

  getAgentContextUsage: async (sessionId: string, settings: unknown): Promise<any> => {
    return jsonify.parse(await invoke<string>('get_agent_context_usage', { sessionId, settings }))
  },

  // Capability invocation
  invokeCapability: async (name: string, args: Record<string, unknown>, connectionId?: string): Promise<string> => {
    return await invoke<string>('invoke_capability', { name, args, connectionId })
  },
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

export default agentApi
