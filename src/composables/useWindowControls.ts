import { getCurrentWindow } from '@tauri-apps/api/window'
import { onMounted, onUnmounted, ref } from 'vue'

export function shouldReserveMacTrafficLightInset(
  isMac: boolean,
  isFullscreen: boolean,
  isDesktop = true,
): boolean {
  return isDesktop && isMac && !isFullscreen
}

export function useWindowControls() {
  const isMaximized = ref(false)
  const isFullscreen = ref(false)

  let unlistenResize: (() => void) | null = null

  async function refreshState() {
    try {
      const window = getCurrentWindow()
      const [maximized, fullscreen] = await Promise.all([
        window.isMaximized(),
        window.isFullscreen(),
      ])
      isMaximized.value = maximized
      isFullscreen.value = fullscreen
    }
    catch {
      // Not in Tauri environment — leave defaults
    }
  }

  const minimize = () => getCurrentWindow().minimize()
  const toggleMaximize = () => getCurrentWindow().toggleMaximize()
  const close = () => getCurrentWindow().close()

  onMounted(async () => {
    await refreshState()
    try {
      const unlisten = await getCurrentWindow().onResized(() => {
        refreshState()
      })
      unlistenResize = unlisten
    }
    catch {
      // Not in Tauri environment
    }
  })

  onUnmounted(() => {
    unlistenResize?.()
  })

  return { isMaximized, isFullscreen, minimize, toggleMaximize, close }
}
