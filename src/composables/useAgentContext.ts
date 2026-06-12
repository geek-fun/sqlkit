export function useAgentContext() {
  const getContextString = () => {
    return ''
  }

  const buildPromptWithContext = (content: string, defaultPrompt: string) => {
    return `${defaultPrompt}\n\n${content}`
  }

  return { getContextString, buildPromptWithContext }
}
