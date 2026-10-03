import type { Component } from 'vue'
import type { PaidFeature } from '../../../common'
import AiPoster from './AiPoster.vue'
import McpPoster from './McpPoster.vue'
import TransferPoster from './TransferPoster.vue'

const posters: Partial<Record<PaidFeature, Component>> = {
  ai: AiPoster,
  transfer: TransferPoster,
  mcp_bridge: McpPoster,
}

export function posterFor(feature: PaidFeature): Component | null {
  return posters[feature] ?? null
}
