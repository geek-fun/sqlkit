<script setup lang="ts">
import type { BridgeStatus, DriverInfo, JreStatus, JreUpdateStatus } from '@/datasources'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { useDatabaseIcon } from '@/composables/useDatabaseIcon'
import { useDownloadEvents } from '@/composables/useDownloadEvents'
import { toast } from '@/composables/useNotifications'
import { jdbcApi } from '@/datasources'
import { dbTypeFromBackend } from '@/store'

const { t } = useI18n()
const { getDatabaseIcon } = useDatabaseIcon()

const dl = useDownloadEvents()

function getDriverIcon(dbType: string): string {
  const dbTypeEnum = dbTypeFromBackend[dbType]
  return dbTypeEnum ? getDatabaseIcon(dbTypeEnum) : ''
}

// --- State ---
const jreStatus = ref<JreStatus | null>(null)

const jreUpdate = ref<JreUpdateStatus | null>(null)
const jreLoading = ref(false)
const jreChecking = ref(false)

const jreDownloading = computed(() => dl.isDownloading('jre'))
const jreDownloadProgress = computed(() => dl.getProgress('jre'))

const systemJreValid = computed(() => {
  const status = jreStatus.value
  if (!status || status.source !== 'system' || !status.version)
    return false
  const major = Number.parseInt(status.version, 10)
  return !Number.isNaN(major) && major >= 25
})

const bridgeStatus = ref<BridgeStatus | null>(null)
const bridgeLoading = ref(false)

const bridgeDownloading = computed(() => dl.isDownloading('bridge'))
const bridgeDownloadProgress = computed(() => dl.getProgress('bridge'))

const drivers = ref<DriverInfo[]>([])
const driversLoading = ref(false)
const removingDriver = ref<string | null>(null)
const checkingDriver = ref<string | null>(null)

// --- Computed sort: installed first, then alphabetical by name ---
const sortedDrivers = ref<DriverInfo[]>([])

function formatFileSize(bytes: number): string {
  if (bytes < 1024)
    return `${bytes} B`
  if (bytes < 1024 * 1024)
    return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

function updateSortedDrivers() {
  sortedDrivers.value = [...drivers.value].sort((a, b) => {
    if (a.installed !== b.installed)
      return a.installed ? -1 : 1
    return a.name.localeCompare(b.name)
  })
}

// --- Load all data in parallel ---
async function loadAll() {
  await Promise.all([
    loadJreStatus(),
    loadBridgeStatus(),
    loadDrivers(),
  ])
}

// --- JRE ---
async function loadJreStatus() {
  jreLoading.value = true
  try {
    const [status, update] = await Promise.all([
      jdbcApi.checkJreStatus(),
      jdbcApi.checkJreUpdate(),
    ])
    jreStatus.value = status
    jreUpdate.value = update
  }
  catch {
    // silently fail
  }
  finally {
    jreLoading.value = false
  }
}

async function handleCheckJreUpdates() {
  jreChecking.value = true
  jreLoading.value = true
  try {
    const update = await jdbcApi.checkJreUpdate()
    jreUpdate.value = update

    if (update.update_available) {
      toast.info(t('pages.settings.jre.jreCard.notifications.updateAvailable'), {
        description: t('pages.settings.jre.jreCard.notifications.newVersion', { version: update.latest_version ?? '' }),
      })
      // Auto-start download with progress (like uninstall → install flow)
      await handleDownloadJre()
    }
    else {
      toast.success(t('pages.settings.jre.jreCard.notifications.upToDate'))
    }
  }
  catch (error) {
    toast.error(t('pages.settings.jre.jreCard.notifications.checkUpdatesFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
  finally {
    jreChecking.value = false
    jreLoading.value = false
  }
}

async function handleDownloadJre() {
  jreLoading.value = true
  dl.reset('jre')
  const ok = await dl.startDownload('jre', 'jre', () => jdbcApi.downloadJre())
  if (!ok) {
    toast.error(t('pages.settings.jre.jreCard.notifications.downloadFailed'), { description: dl.getError('jre') ?? '' })
  }
  else {
    toast.success(t('pages.settings.jre.jreCard.notifications.downloadSuccess'))
    await loadJreStatus()
  }
  jreLoading.value = false
  dl.reset('jre')
}

async function handleRemoveJre() {
  jreLoading.value = true
  try {
    await jdbcApi.removeJre()
    jreStatus.value = null
    jreUpdate.value = null
    await loadJreStatus()
  }
  catch (error) {
    toast.error(t('pages.settings.jre.jreCard.notifications.removeFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
  finally {
    jreLoading.value = false
  }
}

// --- Bridge JAR ---
async function loadBridgeStatus() {
  bridgeLoading.value = true
  try {
    bridgeStatus.value = await jdbcApi.checkBridgeStatus()
  }
  catch {
    // silently fail
  }
  finally {
    bridgeLoading.value = false
  }
}

async function handleDownloadBridge() {
  bridgeLoading.value = true
  dl.reset('bridge')
  const ok = await dl.startDownload('bridge', 'bridge', () => jdbcApi.downloadBridgeJar())
  if (!ok) {
    toast.error(t('pages.settings.jre.bridgeCard.notifications.downloadFailed'), { description: dl.getError('bridge') ?? '' })
  }
  else {
    toast.success(t('pages.settings.jre.bridgeCard.notifications.downloadSuccess'))
    await loadBridgeStatus()
  }
  bridgeLoading.value = false
  dl.reset('bridge')
}

async function handleRemoveBridge() {
  bridgeLoading.value = true
  try {
    await jdbcApi.removeBridgeJar()
    bridgeStatus.value = null
    await loadBridgeStatus()
  }
  catch (error) {
    toast.error(t('pages.settings.jre.bridgeCard.notifications.removeFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
  finally {
    bridgeLoading.value = false
  }
}

// --- Drivers ---
async function loadDrivers() {
  driversLoading.value = true
  try {
    drivers.value = await jdbcApi.listDrivers()
    updateSortedDrivers()
  }
  catch {
    // silently fail
  }
  finally {
    driversLoading.value = false
  }
}

async function handleDownloadDriver(dbType: string) {
  if (!bridgeStatus.value?.installed) {
    toast.error(t('pages.settings.jre.driversCard.notifications.installBridgeFirst'))
    return
  }
  dl.reset(dbType)
  const ok = await dl.startDownload('driver', dbType, () => jdbcApi.downloadDriver(dbType))
  if (!ok) {
    toast.error(t('pages.settings.jre.driversCard.notifications.downloadFailed'), { description: dl.getError(dbType) ?? '' })
  }
  else {
    toast.success(t('pages.settings.jre.driversCard.notifications.downloadSuccess'))
    await loadDrivers()
  }
  dl.reset(dbType)
}

async function handleRemoveDriver(dbType: string) {
  removingDriver.value = dbType
  try {
    await jdbcApi.removeDriver(dbType)
    await loadDrivers()
  }
  catch (error) {
    toast.error(t('pages.settings.jre.driversCard.notifications.removeFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
  finally {
    removingDriver.value = null
  }
}

async function handleCheckDriverUpdates(dbType: string) {
  checkingDriver.value = dbType
  try {
    const update = await jdbcApi.checkDriverUpdate(dbType)
    if (update.update_available) {
      toast.info(t('pages.settings.jre.driversCard.notifications.updateAvailable'), {
        description: t('pages.settings.jre.driversCard.notifications.newVersion', { version: update.latest_version ?? '' }),
      })
      await handleDownloadDriver(dbType)
    }
    else {
      toast.success(t('pages.settings.jre.driversCard.notifications.upToDate'))
    }
  }
  catch (error) {
    toast.error(t('pages.settings.jre.driversCard.notifications.checkUpdatesFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
  finally {
    checkingDriver.value = null
  }
}

// --- Refresh ---
const allLoading = ref(false)

async function handleRefresh() {
  allLoading.value = true
  try {
    await loadAll()
  }
  finally {
    allLoading.value = false
  }
}

onMounted(() => {
  loadAll()
})
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>{{ t('pages.settings.jre.title') }}</CardTitle>
      <CardDescription>{{ t('pages.settings.jre.description') }}</CardDescription>
    </CardHeader>
    <CardContent class="space-y-6">
      <!-- Java Runtime Card (single card, two rows: Managed + System) -->
      <div class="p-4 border rounded-lg space-y-3">
        <div class="space-y-1">
          <p class="text-sm font-medium">
            {{ t('pages.settings.jre.jreCard.title') }}
          </p>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.jre.jreCard.description') }}
          </p>
        </div>

        <div class="border rounded-md divide-y">
          <!-- Managed JRE row -->
          <div class="px-4 py-3 flex gap-3 items-center">
            <span class="i-carbon-wireless-payment text-muted-foreground flex-shrink-0 h-5 w-5" />
            <div class="flex-1 min-w-0">
              <p class="text-sm font-medium truncate">
                {{ t('pages.settings.jre.jreCard.status.managed') }}
              </p>
              <p v-if="jreStatus?.source === 'managed'" class="text-xs text-muted-foreground mt-0.5 truncate">
                {{ jreStatus.version }}
                <template v-if="jreStatus.path">
                  · {{ jreStatus.path }}
                </template>
              </p>
              <p v-else class="text-xs text-muted-foreground mt-0.5">
                {{ t('pages.settings.jre.jreCard.status.notInstalled') }}
              </p>
            </div>
            <!-- Download progress indicator (before badge) -->
            <div v-if="jreDownloadProgress" class="flex flex-shrink-0 items-center">
              <svg class="h-5 w-5 -rotate-90" viewBox="0 0 16 16">
                <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="2" opacity="0.15" />
                <circle
                  cx="8" cy="8" r="6"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-dasharray="37.6991"
                  :stroke-dashoffset="37.6991 * (1 - (jreDownloadProgress?.downloaded ?? 0) / (jreDownloadProgress?.total ?? 1))"
                  stroke-linecap="round"
                />
              </svg>
            </div>
            <div class="flex-shrink-0">
              <Badge :variant="jreStatus?.source === 'managed' ? 'success' : 'secondary'">
                {{ jreStatus?.source === 'managed' ? t('pages.settings.jre.jreCard.status.installed') : t('pages.settings.jre.jreCard.status.notInstalled') }}
              </Badge>
            </div>
            <div class="flex flex-shrink-0 gap-1 items-center">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button size="icon" variant="ghost" class="h-7 w-7" :disabled="jreLoading" @click="handleCheckJreUpdates">
                      <span class="i-carbon-update-now h-3.5 w-3.5" :class="{ 'animate-spin': jreChecking }" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent><p>{{ t('pages.settings.jre.jreCard.actions.checkUpdates') }}</p></TooltipContent>
                </Tooltip>
              </TooltipProvider>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button v-if="jreStatus?.source !== 'managed' || jreUpdate?.update_available" size="icon" variant="ghost" class="h-7 w-7" :disabled="jreDownloading || jreLoading" @click="handleDownloadJre">
                      <span v-if="jreDownloading" class="i-carbon-circle-dash h-3.5 w-3.5 animate-spin" />
                      <span v-else class="i-carbon-download h-3.5 w-3.5" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent><p>{{ jreStatus?.source !== 'managed' ? t('pages.settings.jre.jreCard.actions.download') : t('pages.settings.jre.jreCard.actions.redownload') }}</p></TooltipContent>
                </Tooltip>
              </TooltipProvider>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button v-if="jreStatus?.source === 'managed'" size="icon" variant="ghost" class="h-7 w-7" :disabled="jreLoading" @click="handleRemoveJre">
                      <span class="i-carbon-trash-can h-3.5 w-3.5" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent><p>{{ t('pages.settings.jre.jreCard.actions.remove') }}</p></TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>

          <!-- System JRE row -->
          <div class="px-4 py-3 flex gap-3 items-center">
            <span class="i-carbon-laptop text-muted-foreground flex-shrink-0 h-5 w-5" />
            <template v-if="systemJreValid">
              <div class="flex-1 min-w-0">
                <p class="text-sm font-medium truncate">
                  {{ t('pages.settings.jre.jreCard.status.system') }}
                </p>
                <p class="text-xs text-muted-foreground mt-0.5 truncate">
                  {{ jreStatus?.path || t('pages.settings.jre.jreCard.status.notInstalled') }}
                  · v{{ jreStatus!.version }}
                </p>
              </div>
              <div class="flex-shrink-0">
                <Badge variant="success">
                  {{ t('pages.settings.jre.jreCard.status.installed') }}
                </Badge>
              </div>
            </template>
            <template v-else>
              <div class="flex-1 min-w-0">
                <p class="text-sm font-medium truncate">
                  {{ t('pages.settings.jre.jreCard.status.system') }}
                </p>
                <p class="text-xs text-muted-foreground mt-0.5">
                  <template v-if="jreStatus?.path">
                    {{ jreStatus.path }} ·
                  </template>{{ t('pages.settings.jre.jreCard.status.systemInvalid') }}
                </p>
              </div>
              <div class="flex-shrink-0">
                <Badge variant="secondary">
                  {{ t('pages.settings.jre.jreCard.status.notInstalled') }}
                </Badge>
              </div>
            </template>
          </div>
        </div>
      </div>

      <!-- Bridge JAR Card -->
      <div class="p-4 border rounded-lg space-y-2">
        <div class="space-y-1">
          <p class="text-sm font-medium">
            {{ t('pages.settings.jre.bridgeCard.title') }}
          </p>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.jre.bridgeCard.description') }}
          </p>
        </div>

        <div class="flex flex-wrap gap-2 min-h-8 items-center">
          <!-- Loading state -->
          <span v-if="bridgeLoading && !bridgeStatus && !bridgeDownloading" class="text-sm text-muted-foreground">
            <span class="i-carbon-circle-dash align-middle h-3.5 w-3.5 inline-block animate-spin" />
            {{ t('pages.settings.jre.bridgeCard.status.checking') }}
          </span>

          <!-- Bridge status loaded -->
          <template v-else-if="bridgeStatus">
            <!-- Download progress indicator (before badge) -->
            <template v-if="bridgeDownloadProgress">
              <svg class="h-5 w-5 -rotate-90" viewBox="0 0 16 16">
                <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="2" opacity="0.15" />
                <circle
                  cx="8"
                  cy="8"
                  r="6"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-dasharray="37.6991"
                  :stroke-dashoffset="37.6991 * (1 - (bridgeDownloadProgress?.downloaded ?? 0) / (bridgeDownloadProgress?.total ?? 1))"
                  stroke-linecap="round"
                />
              </svg>
            </template>
            <Badge
              :variant="bridgeStatus.installed ? 'success' : 'secondary'"
            >
              {{ bridgeStatus.installed
                ? t('pages.settings.jre.bridgeCard.status.installed')
                : t('pages.settings.jre.bridgeCard.status.notInstalled') }}
            </Badge>
            <span
              v-if="bridgeStatus.installed"
              class="text-sm text-muted-foreground"
            >
              {{ bridgeStatus.current_version }}
            </span>
          </template>

          <!-- Fallback -->
          <span v-else class="text-sm text-muted-foreground">
            {{ t('pages.settings.jre.bridgeCard.status.notInstalled') }}
          </span>

          <div class="flex-1" />

          <!-- Toolbar -->
          <div class="flex gap-1 items-center">
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger as-child>
                  <Button
                    v-if="!bridgeStatus?.installed"
                    size="icon"
                    variant="ghost"
                    class="h-7 w-7"
                    :disabled="bridgeDownloading || bridgeLoading"
                    @click="handleDownloadBridge"
                  >
                    <span v-if="bridgeDownloading" class="i-carbon-circle-dash h-3.5 w-3.5 animate-spin" />
                    <span v-else class="i-carbon-download h-3.5 w-3.5" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <p>{{ t('pages.settings.jre.bridgeCard.actions.download') }}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger as-child>
                  <Button
                    v-if="bridgeStatus?.installed"
                    size="icon"
                    variant="ghost"
                    class="h-7 w-7"
                    :disabled="bridgeLoading"
                    @click="handleRemoveBridge"
                  >
                    <span class="i-carbon-trash-can h-3.5 w-3.5" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <p>{{ t('pages.settings.jre.bridgeCard.actions.remove') }}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>
        </div>
      </div>

      <!-- JDBC Drivers Card -->
      <div class="p-4 border rounded-lg space-y-3">
        <div class="flex items-start justify-between">
          <div class="space-y-1">
            <p class="text-sm font-medium">
              {{ t('pages.settings.jre.driversCard.title') }}
            </p>
            <p class="text-xs text-muted-foreground">
              {{ t('pages.settings.jre.driversCard.description') }}
            </p>
          </div>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger as-child>
                <Button
                  size="icon"
                  variant="ghost"
                  class="h-7 w-7"
                  :disabled="allLoading"
                  @click="handleRefresh"
                >
                  <span class="i-carbon-refresh h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>{{ t('pages.settings.jre.actions.refresh') }}</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>

        <!-- Loading state -->
        <div
          v-if="driversLoading && drivers.length === 0"
          class="text-sm text-muted-foreground py-4 text-center"
        >
          {{ t('pages.settings.jre.driversCard.status.loading') }}
        </div>

        <!-- Empty state -->
        <div
          v-else-if="drivers.length === 0 && !driversLoading"
          class="py-6 text-center border rounded-md"
        >
          <p class="text-sm text-muted-foreground">
            {{ t('pages.settings.jre.driversCard.empty.title') }}
          </p>
          <p class="text-xs text-muted-foreground mt-1">
            {{ t('pages.settings.jre.driversCard.empty.message') }}
          </p>
        </div>

        <!-- Driver list -->
        <div v-else class="border rounded-md divide-y">
          <div
            v-for="driver in sortedDrivers"
            :key="driver.db_type"
            class="px-4 py-3 flex gap-3 items-center"
          >
            <img
              :src="getDriverIcon(driver.db_type)"
              alt=""
              class="flex-shrink-0 h-5 w-5 object-contain"
            >
            <div class="flex-1 min-w-0">
              <p class="text-sm font-medium truncate">
                {{ driver.name }}
              </p>
              <p
                v-if="driver.installed && driver.filename"
                class="text-xs text-muted-foreground mt-0.5 truncate"
              >
                {{ driver.filename }}
                <template v-if="driver.file_size != null">
                  · {{ formatFileSize(driver.file_size) }}
                </template>
                <template v-if="driver.resolved_version">
                  · v{{ driver.resolved_version }}
                </template>
              </p>
            </div>
            <!-- Download progress indicator (before badge) -->
            <div v-if="dl.isDownloading(driver.db_type) || dl.getProgress(driver.db_type)" class="flex flex-shrink-0 items-center">
              <svg class="h-5 w-5 -rotate-90" viewBox="0 0 16 16">
                <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="2" opacity="0.15" />
                <circle
                  cx="8"
                  cy="8"
                  r="6"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-dasharray="37.6991"
                  :stroke-dashoffset="37.6991 * (1 - (dl.getProgress(driver.db_type)?.downloaded ?? 0) / (dl.getProgress(driver.db_type)?.total ?? 1))"
                  stroke-linecap="round"
                />
              </svg>
            </div>
            <div class="flex-shrink-0">
              <Badge
                :variant="driver.installed ? 'success' : 'secondary'"
              >
                {{ driver.installed
                  ? t('pages.settings.jre.driversCard.status.installed')
                  : t('pages.settings.jre.driversCard.status.notInstalled') }}
              </Badge>
            </div>
            <div class="flex flex-shrink-0 gap-1 items-center">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      size="icon"
                      variant="ghost"
                      class="h-7 w-7"
                      :disabled="!driver.installed || checkingDriver === driver.db_type || dl.isDownloading(driver.db_type)"
                      @click="handleCheckDriverUpdates(driver.db_type)"
                    >
                      <span
                        class="i-carbon-update-now h-3.5 w-3.5"
                        :class="{ 'animate-spin': checkingDriver === driver.db_type }"
                      />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.settings.jre.driversCard.actions.checkUpdates') }}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      v-if="!driver.installed"
                      size="icon"
                      variant="ghost"
                      class="h-7 w-7"
                      :disabled="dl.isDownloading(driver.db_type)"
                      @click="handleDownloadDriver(driver.db_type)"
                    >
                      <span v-if="dl.isDownloading(driver.db_type)" class="i-carbon-circle-dash h-3.5 w-3.5 animate-spin" />
                      <span v-else class="i-carbon-download h-3.5 w-3.5" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.settings.jre.driversCard.actions.download') }}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      v-if="driver.installed"
                      size="icon"
                      variant="ghost"
                      class="h-7 w-7"
                      :disabled="removingDriver === driver.db_type"
                      @click="handleRemoveDriver(driver.db_type)"
                    >
                      <span class="i-carbon-trash-can h-3.5 w-3.5" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{{ t('pages.settings.jre.driversCard.actions.remove') }}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
