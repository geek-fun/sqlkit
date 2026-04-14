import { computed, ref } from 'vue'
import { platform } from '@tauri-apps/plugin-os'

const platformCache = ref<string | null>(null)

const getPlatform = async (): Promise<string> => {
  if (platformCache.value) {
    return platformCache.value
  }
  try {
    const p = await platform()
    platformCache.value = p
    return p
  } catch {
    return 'unknown'
  }
}

// Initialize platform detection on module load
getPlatform()

const usePlatform = () => {
  const isMac = computed(() => platformCache.value === 'macos')
  const isWindows = computed(() => platformCache.value === 'windows')
  const isLinux = computed(() => platformCache.value === 'linux')

  // Aligned with dockit: ⌘ on Mac, Ctrl on other platforms (no + suffix)
  const cmdKey = computed(() => isMac.value ? '⌘' : 'Ctrl')

  // Full modifier string for display: ⌘+ on Mac, Ctrl+ on others
  const modifierKey = computed(() => isMac.value ? '⌘' : 'Ctrl+')

  return {
    isMac,
    isWindows,
    isLinux,
    cmdKey,
    modifierKey,
    platform: platformCache,
  }
}

export { usePlatform, getPlatform }