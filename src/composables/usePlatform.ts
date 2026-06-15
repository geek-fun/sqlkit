import { platform } from '@tauri-apps/plugin-os'
import { computed, ref } from 'vue'

const platformCache = ref<string | null>(null)

async function getPlatform(): Promise<string> {
  if (platformCache.value) {
    return platformCache.value
  }
  try {
    const p = await platform()
    platformCache.value = p
    return p
  }
  catch {
    return 'unknown'
  }
}

// Initialize platform detection on module load
getPlatform()

function usePlatform() {
  const isMac = computed(() => platformCache.value === 'macos')
  const isWindows = computed(() => platformCache.value === 'windows')
  const isLinux = computed(() => platformCache.value === 'linux')

  // Aligned with dockit: ⌘ on Mac, Ctrl on other platforms (no + suffix)
  const cmdKey = computed(() => isMac.value ? '⌘' : 'Ctrl')

  // Full modifier string for display: ⌘+ on Mac, Ctrl+ on others
  const modifierKey = computed(() => isMac.value ? '⌘' : 'Ctrl+')

  // Alt modifier for display: ⌥ on Mac, Alt+ on others
  const altKey = computed(() => isMac.value ? '⌥' : 'Alt+')

  return {
    isMac,
    isWindows,
    isLinux,
    cmdKey,
    modifierKey,
    altKey,
    platform: platformCache,
  }
}

export { getPlatform, usePlatform }
