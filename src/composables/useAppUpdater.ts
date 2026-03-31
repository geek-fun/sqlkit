import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { toast } from './useNotifications'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

export const useAppUpdater = () => {
  const { t } = useI18n()
  const updateAvailable = ref(false)
  const updateInfo = ref<Update | null>(null)
  const isChecking = ref(false)
  const isDownloading = ref(false)
  const isInstalling = ref(false)

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
      toast.info(t('updater.downloading'))

      await updateInfo.value.downloadAndInstall(() => {
        isDownloading.value = false
        isInstalling.value = true
        toast.info(t('updater.installing'))
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
    }
  }

  const dismissUpdate = () => {
    updateAvailable.value = false
    updateInfo.value = null
  }

  return {
    updateAvailable,
    updateInfo,
    isChecking,
    isDownloading,
    isInstalling,
    checkForUpdates,
    downloadAndInstall,
    dismissUpdate,
  }
}
