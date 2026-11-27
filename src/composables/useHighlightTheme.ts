import { watch } from 'vue'
import { ThemeType, useAppStore } from '@/store/appStore'

let highlightLink: HTMLLinkElement | null = null
let activeTheme: Exclude<ThemeType, ThemeType.AUTO> | null = null

const applyHighlightTheme = async (uiThemeType: Exclude<ThemeType, ThemeType.AUTO>) => {
  if (typeof document === 'undefined' || activeTheme === uiThemeType)
    return
  activeTheme = uiThemeType
  try {
    const mod = uiThemeType === ThemeType.DARK
      ? await import('highlight.js/styles/atom-one-dark.css?url')
      : await import('highlight.js/styles/atom-one-light.css?url')
    if (activeTheme !== uiThemeType)
      return
    if (!highlightLink) {
      highlightLink = document.createElement('link')
      highlightLink.rel = 'stylesheet'
      document.head.appendChild(highlightLink)
    }
    highlightLink.href = mod.default
  }
  catch {
    // Non-browser / test environments without CSS ?url loader
  }
}

export const useHighlightTheme = () => {
  const appStore = useAppStore()
  watch(() => appStore.uiThemeType, (theme) => {
    void applyHighlightTheme(theme)
  }, { immediate: true })
}
