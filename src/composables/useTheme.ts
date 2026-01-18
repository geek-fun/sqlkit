import { onBeforeUnmount, onMounted, ref } from 'vue'

export type Theme = 'dark' | 'light' | 'system'

const THEME_STORAGE_KEY = 'sqlkit-theme'

export function useTheme() {
  const theme = ref<Theme>('system')
  const isDark = ref(false)
  let mediaQuery: MediaQueryList | null = null
  let handleChange: (() => void) | null = null

  const getSystemTheme = (): 'dark' | 'light' => {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }

  const applyTheme = (newTheme: Theme) => {
    const root = document.documentElement

    let effectiveTheme: 'dark' | 'light'
    if (newTheme === 'system') {
      effectiveTheme = getSystemTheme()
    }
    else {
      effectiveTheme = newTheme
    }

    if (effectiveTheme === 'dark') {
      root.classList.add('dark')
      isDark.value = true
    }
    else {
      root.classList.remove('dark')
      isDark.value = false
    }
  }

  const setTheme = (newTheme: Theme) => {
    theme.value = newTheme
    try {
      localStorage.setItem(THEME_STORAGE_KEY, newTheme)
    }
    catch (error) {
      console.error('Failed to save theme preference:', error)
    }
    applyTheme(newTheme)
  }

  const toggleTheme = () => {
    const newTheme = isDark.value ? 'light' : 'dark'
    setTheme(newTheme)
  }

  onMounted(() => {
    // Load theme from localStorage or default to system
    let savedTheme: Theme | null = null
    try {
      savedTheme = localStorage.getItem(THEME_STORAGE_KEY) as Theme | null
    }
    catch (error) {
      console.error('Failed to load theme preference:', error)
    }
    theme.value = savedTheme || 'system'
    applyTheme(theme.value)

    // Listen for system theme changes
    mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    handleChange = () => {
      if (theme.value === 'system') {
        applyTheme('system')
      }
    }
    mediaQuery.addEventListener('change', handleChange)
  })

  onBeforeUnmount(() => {
    // Cleanup event listener
    if (mediaQuery && handleChange) {
      mediaQuery.removeEventListener('change', handleChange)
    }
  })

  return {
    theme,
    isDark,
    setTheme,
    toggleTheme,
  }
}
