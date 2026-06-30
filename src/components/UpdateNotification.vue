<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { useAppUpdater } from '@/composables/useAppUpdater'

const { t } = useI18n()

const {
  updateAvailable,
  updateInfo,
  isDownloading,
  downloadProgress,
  isInstalling,
  isRestarting,
  downloadAndInstall,
  skipUpdate,
  dismissUpdate,
} = useAppUpdater()

const installButtonLabel = computed(() => {
  if (isRestarting.value)
    return t('updater.restarting')
  if (isDownloading.value && downloadProgress.value !== null)
    return t('updater.downloadingPercent', { percent: downloadProgress.value })
  if (isDownloading.value)
    return t('updater.downloading')
  if (isInstalling.value)
    return t('updater.installing')
  return t('updater.updateNow')
})

const progressLabel = computed(() => {
  if (isRestarting.value)
    return t('updater.restarting')
  if (isInstalling.value)
    return t('updater.installing')
  return t('updater.downloading')
})

const isProgressActive = computed(() =>
  isDownloading.value || isInstalling.value || isRestarting.value,
)

const buttonsDisabled = computed(() =>
  isDownloading.value || isInstalling.value || isRestarting.value,
)
</script>

<template>
  <div v-if="updateAvailable" class="fixed bottom-6 right-6 z-50">
    <div
      class="w-[420px] rounded-2xl border bg-background shadow-lg overflow-hidden"
      style="box-shadow: 0 10px 30px rgba(0, 0, 0, 0.12)"
    >
      <!-- Header -->
      <div class="flex items-start gap-3 p-5 pb-4">
        <div class="shrink-0 inline-flex items-center justify-center w-[42px] h-[42px] rounded-xl"
          style="background: #eaf7ee; border: 1px solid #cfead7"
        >
          <svg
            class="h-5 w-5"
            style="color: #22a559"
            viewBox="0 0 24 24"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M12 4v13m0 0l-5-5m5 5l5-5M5 21h14"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </div>

        <div class="flex-1 min-w-0">
          <div class="font-bold text-[16px] text-foreground mb-0.5">
            {{ t('updater.updateAvailable') }}
          </div>
          <div class="text-sm text-muted-foreground">
            {{ t('updater.newVersion', { version: updateInfo?.version }) }}
          </div>
        </div>

        <button
          class="shrink-0 w-7 h-7 inline-flex items-center justify-center rounded-lg border-none bg-transparent text-muted-foreground hover:bg-accent hover:text-foreground transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="buttonsDisabled"
          @click="dismissUpdate"
        >
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M9 3L3 9M3 3l6 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
        </button>
      </div>

      <!-- Progress bar -->
      <div v-if="isProgressActive" class="px-5 pb-3 space-y-1.5">
        <div class="flex justify-between items-center">
          <span class="text-xs text-muted-foreground font-medium">{{ progressLabel }}</span>
          <span
            v-if="isDownloading && downloadProgress !== null"
            class="text-xs font-semibold"
            style="color: #27ae60"
          >
            {{ downloadProgress }}%
          </span>
        </div>
        <div class="w-full h-[5px] rounded-full bg-secondary overflow-hidden">
          <div
            class="h-full rounded-full transition-all duration-300"
            :class="{
              'w-[40%] animate-progress-slide': isInstalling || isRestarting || (isDownloading && downloadProgress === null),
            }"
            :style="isDownloading && downloadProgress !== null ? { width: `${downloadProgress}%`, background: '#27ae60' } : { background: '#27ae60' }"
          />
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between px-4 py-3 border-t">
        <button
          class="text-sm font-medium bg-transparent border-none text-muted-foreground hover:text-foreground transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="buttonsDisabled"
          @click="skipUpdate"
        >
          {{ t('updater.skip') }}
        </button>

        <div class="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            class="font-semibold min-w-[96px]"
            :disabled="buttonsDisabled"
            @click="dismissUpdate"
          >
            {{ t('updater.later') }}
          </Button>
          <Button
            variant="default"
            size="sm"
            class="font-semibold min-w-[96px] text-white"
            :class="{
              'opacity-50 cursor-not-allowed': buttonsDisabled,
            }"
            :disabled="buttonsDisabled"
            @click="downloadAndInstall"
          >
            {{ installButtonLabel }}
          </Button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Full tailwind-like animation for indeterminate progress bar */
@keyframes progress-slide {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(350%);
  }
}

.animate-progress-slide {
  animation: progress-slide 1.4s ease-in-out infinite;
}
</style>
