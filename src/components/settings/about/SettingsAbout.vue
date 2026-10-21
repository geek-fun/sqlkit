<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { SettingsFeedback } from '@/components/settings'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { version as buildVersion } from '../../../../package.json'

const { t } = useI18n()
const version = ref(buildVersion)
const {
  checkForUpdates,
  downloadAndInstall,
  isChecking,
  isDownloading,
  isInstalling,
  updateAvailable,
  updateInfo,
  downloadProgress,
  updateError,
  isRestarting,
  isUpToDate,
} = useAppUpdater()

const busy = computed(() => isChecking.value || isDownloading.value || isInstalling.value || isRestarting.value)
const updateLabel = computed(() => {
  if (isRestarting.value)
    return t('updater.restarting')
  if (isInstalling.value)
    return t('pages.settings.updates.installing')
  if (isDownloading.value)
    return downloadProgress.value === null ? t('updater.downloading') : t('updater.downloadingPercent', { percent: downloadProgress.value })
  if (isChecking.value)
    return t('pages.settings.updates.checking')
  return updateAvailable.value ? t('updater.updateNow') : t('pages.settings.updates.check')
})
const feedbackLabel = computed(() => {
  if (updateError.value)
    return t('pages.settings.feedback.failed')
  if (busy.value)
    return updateLabel.value
  return updateAvailable.value
    ? t('updater.newVersion', { version: updateInfo.value?.version })
    : t('updater.upToDate')
})

const handleUpdate = async () => {
  if (updateAvailable.value)
    await downloadAndInstall()
  else
    await checkForUpdates(true, { includeSkipped: true })
}

onMounted(() => {
  getVersion().then(value => version.value = value).catch(() => {})
  void checkForUpdates(false, { includeSkipped: true })
})
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>{{ t('pages.settings.about.title') }}</CardTitle>
      <CardDescription>{{ t('pages.settings.about.description') }}</CardDescription>
    </CardHeader>
    <CardContent class="space-y-5">
      <div class="flex flex-wrap gap-3 items-center justify-between">
        <div class="space-y-1">
          <p class="text-sm font-medium flex gap-2 items-center">
            {{ t('pages.settings.about.version') }}
            <span class="text-muted-foreground font-mono">{{ version }}</span>
          </p>
          <p class="text-xs text-muted-foreground max-w-lg">
            {{ t('pages.settings.updates.checkForUpdatesHelp') }}
          </p>
        </div>
        <Button variant="outline" size="sm" :disabled="busy" @click="handleUpdate">
          <span v-if="busy" class="i-carbon-circle-dash mr-2 shrink-0 h-4 w-4 animate-spin" aria-hidden="true" />
          {{ updateLabel }}
        </Button>
      </div>
      <SettingsFeedback
        v-if="busy || updateError || (isUpToDate && !updateAvailable)"
        :state="updateError ? 'error' : busy ? 'pending' : 'success'"
        :label="feedbackLabel"
        :detail="updateError"
        :progress="isDownloading ? downloadProgress : null"
      />
      <p v-if="updateAvailable && !busy && !updateError" class="text-sm text-primary-text" role="status">
        {{ feedbackLabel }}
      </p>
      <div class="text-sm flex gap-3 items-center justify-between">
        <span class="font-medium">{{ t('pages.settings.about.license') }}</span>
        <span class="text-muted-foreground">{{ t('pages.settings.about.licenseName') }}</span>
      </div>
      <div class="border-t border-border/40" />
      <a
        href="https://github.com/geek-fun/sqlkit" target="_blank" rel="noopener noreferrer"
        class="text-sm p-2 rounded-md flex flex-wrap gap-2 transition-colors items-center justify-between -mx-2 hover:bg-muted/50"
      >
        <span class="font-medium">{{ t('pages.settings.about.repository') }}</span>
        <span class="text-muted-foreground flex gap-2 items-center">
          geek-fun/sqlkit <span class="i-carbon-launch h-4 w-4" aria-hidden="true" />
        </span>
      </a>
      <a
        href="https://www.geekfun.club" target="_blank" rel="noopener noreferrer"
        class="text-sm p-2 rounded-md flex flex-wrap gap-2 transition-colors items-center justify-between -mx-2 hover:bg-muted/50"
      >
        <span class="font-medium">{{ t('pages.settings.about.website') }}</span>
        <span class="text-muted-foreground flex gap-2 items-center">
          geekfun.club <span class="i-carbon-launch h-4 w-4" aria-hidden="true" />
        </span>
      </a>
      <p class="text-xs text-muted-foreground pt-2 text-center border-t border-border/40">
        {{ t('pages.settings.about.copyright', { year: new Date().getFullYear() }) }}
      </p>
    </CardContent>
  </Card>
</template>
