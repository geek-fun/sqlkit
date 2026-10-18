export type ConnectionPrompt = {
  alias: string
  prompt: string
}

export type SystemPromptLayers = {
  base: string
  userGlobal?: string
  connectionPrompts?: ConnectionPrompt[]
  sidebarContext?: string
}

const isNonEmpty = (value: string | undefined | null): value is string => Boolean(value?.trim())

export const assembleSystemPrompt = (layers: SystemPromptLayers): string => {
  const parts = isNonEmpty(layers.base) ? [layers.base.trim()] : []
  if (isNonEmpty(layers.userGlobal))
    parts.push(layers.userGlobal.trim())

  const prompts = (layers.connectionPrompts ?? []).filter(({ prompt }) => isNonEmpty(prompt))
  prompts.forEach(({ alias, prompt }) => {
    parts.push(prompts.length > 1 ? `## Connection Context: ${alias}\n\n${prompt.trim()}` : prompt.trim())
  })

  if (isNonEmpty(layers.sidebarContext))
    parts.push(layers.sidebarContext.trim())

  return parts.join('\n\n')
}
