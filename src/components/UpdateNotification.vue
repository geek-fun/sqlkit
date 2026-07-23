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
  isInstalling,
  isRestarting,
  downloadProgress,
  updateError,
  downloadAndInstall,
  skipUpdate,
  dismissUpdate,
  clearError,
} = useAppUpdater()

const hasError = computed(() => !!updateError.value)

const installButtonLabel = computed(() => {
  if (hasError.value)
    return t('updater.retry')
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
  <div v-if="updateAvailable" class="bottom-6 right-6 fixed z-50">
    <div
      class="border rounded-2xl bg-background w-[420px] shadow-xl overflow-hidden"
    >
      <!-- Header -->
      <div class="p-5 pb-4 flex gap-3 items-start">
        <div class="border border-success-border rounded-xl bg-success-muted inline-flex shrink-0 h-[42px] w-[42px] items-center justify-center">
          <svg
            class="text-success h-5 w-5"
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
          <div class="text-[16px] text-foreground font-bold mb-0.5">
            {{ t('updater.updateAvailable') }}
          </div>
          <div class="text-sm text-muted-foreground">
            {{ t('updater.newVersion', { version: updateInfo?.version }) }}
          </div>
        </div>

        <button
          class="text-muted-foreground rounded-lg border-none bg-transparent inline-flex shrink-0 h-7 w-7 transition-colors items-center justify-center hover:text-foreground hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed"
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
        <div class="flex items-center justify-between">
          <span class="text-xs text-muted-foreground font-medium">{{ progressLabel }}</span>
          <span
            v-if="isDownloading && downloadProgress !== null"
            class="text-xs text-success font-semibold"
          >
            {{ downloadProgress }}%
          </span>
        </div>
        <div class="rounded-full bg-secondary h-[5px] w-full overflow-hidden">
          <div
            class="rounded-full bg-success h-full transition-all duration-300"
            :class="{
              'w-[40%] animate-progress-slide': isInstalling || isRestarting || (isDownloading && downloadProgress === null),
            }"
            :style="isDownloading && downloadProgress !== null ? { width: `${downloadProgress}%` } : {}"
          />
        </div>
      </div>

      <!-- Error state -->
      <div v-if="hasError" class="px-5 pb-3">
        <div class="px-3 py-2.5 border border-destructive/30 rounded-lg bg-destructive/10 space-y-1.5">
          <div class="flex gap-2 items-start">
            <span class="i-carbon-warning text-destructive mt-0.5 shrink-0 h-4 w-4" />
            <div class="flex-1 min-w-0">
              <p class="text-xs text-destructive font-semibold">
                {{ t('updater.installFailed') }}
              </p>
              <p class="text-xs text-destructive/80 leading-relaxed mt-0.5 break-all">
                {{ updateError }}
              </p>
            </div>
            <button
              class="text-destructive/60 shrink-0 transition-colors hover:text-destructive"
              :aria-label="t('common.buttons.dismiss')"
              @click="clearError"
            >
              <span class="i-carbon-close h-3.5 w-3.5" />
            </button>
          </div>
          <p class="text-xs text-destructive/70 pl-6">
            {{ t('updater.installFailedHelp') }}
          </p>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-4 py-3 border-t flex items-center justify-between">
        <button
          class="text-sm text-muted-foreground font-medium border-none bg-transparent transition-colors hover:text-foreground disabled:opacity-50 disabled:cursor-not-allowed"
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
            class="text-white font-semibold min-w-[96px]"
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
