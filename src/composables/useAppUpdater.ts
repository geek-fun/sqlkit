import type { DownloadEvent, Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { check } from '@tauri-apps/plugin-updater'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from './useNotifications'

export function useAppUpdater() {
  const { t } = useI18n()
  const updateAvailable = ref(false)
  const updateInfo = ref<Update | null>(null)
  const isChecking = ref(false)
  const isDownloading = ref(false)
  const isInstalling = ref(false)

  /** Accumulated downloaded bytes for the current update download */
  const downloadedBytes = ref(0)
  /** Total content length (from Started event) */
  const contentLength = ref(0)
  /** Download progress as percentage 0–100, null when not downloading */
  const downloadProgress = computed(() => {
    if (!isDownloading.value || contentLength.value <= 0)
      return null
    return Math.round((downloadedBytes.value / contentLength.value) * 100)
  })

  const checkForUpdates = async (showToast = false): Promise<Update | null> => {
    if (isChecking.value)
      return null

    try {
      isChecking.value = true
      const update = await check()

      if (update) {
        updateAvailable.value = true
        updateInfo.value = update

        if (showToast) {
          toast.info(t('updater.updateAvailable'), {
            description: t('updater.newVersion', { version: update.version }),
          })
        }

        return update
      }
      else {
        if (showToast) {
          toast.success(t('updater.upToDate'))
        }
        return null
      }
    }
    catch (error) {
      console.error('Failed to check for updates:', error)
      if (showToast) {
        toast.error(t('updater.checkFailed'), {
          description: error instanceof Error ? error.message : String(error),
        })
      }
      return null
    }
    finally {
      isChecking.value = false
    }
  }

  const downloadAndInstall = async () => {
    if (!updateInfo.value)
      return

    try {
      isDownloading.value = true
      downloadedBytes.value = 0
      contentLength.value = 0
      toast.info(t('updater.downloading'))

      await updateInfo.value.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === 'Started') {
          contentLength.value = event.data.contentLength ?? 0
        }
        else if (event.event === 'Progress') {
          downloadedBytes.value += event.data.chunkLength
        }
        else if (event.event === 'Finished') {
          isDownloading.value = false
          isInstalling.value = true
          toast.info(t('updater.installing'))
        }
      })

      toast.success(t('updater.installed'))

      setTimeout(async () => {
        await relaunch()
      }, 1500)
    }
    catch (error) {
      console.error('Failed to install update:', error)
      toast.error(t('updater.installFailed'), {
        description: error instanceof Error ? error.message : String(error),
      })
    }
    finally {
      isDownloading.value = false
      isInstalling.value = false
      updateAvailable.value = false
      updateInfo.value = null
      downloadedBytes.value = 0
      contentLength.value = 0
    }
  }

  const dismissUpdate = () => {
    updateAvailable.value = false
    updateInfo.value = null
    downloadedBytes.value = 0
    contentLength.value = 0
  }

  return {
    updateAvailable,
    updateInfo,
    isChecking,
    isDownloading,
    isInstalling,
    downloadProgress,
    checkForUpdates,
    downloadAndInstall,
    dismissUpdate,
  }
}
