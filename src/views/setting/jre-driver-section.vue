<script setup lang="ts">
import type { BridgeStatus, DriverInfo, JreStatus, JreUpdateStatus } from '@/datasources'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Switch } from '@/components/ui/switch'
import { jdbcApi } from '@/datasources'

const { t } = useI18n()

// --- State ---
const jdbcNeeded = ref(true)

const jreStatus = ref<JreStatus | null>(null)
const jreUpdate = ref<JreUpdateStatus | null>(null)
const jreLoading = ref(false)

const bridgeStatus = ref<BridgeStatus | null>(null)
const bridgeLoading = ref(false)

const drivers = ref<DriverInfo[]>([])
const driversLoading = ref(false)
const downloadingDriver = ref<string | null>(null)
const removingDriver = ref<string | null>(null)

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
    loadJdbcNeeded(),
    loadJreStatus(),
    loadBridgeStatus(),
    loadDrivers(),
  ])
}

// --- JDBC Toggle ---
async function loadJdbcNeeded() {
  try {
    jdbcNeeded.value = await jdbcApi.getJdbcNeeded()
  }
  catch {
    jdbcNeeded.value = true
  }
}

async function handleToggleJdbc() {
  const next = !jdbcNeeded.value
  try {
    await jdbcApi.setJdbcNeeded(next)
    jdbcNeeded.value = next
  }
  catch {
    // revert on failure
    jdbcNeeded.value = !next
  }
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
  jreLoading.value = true
  try {
    jreUpdate.value = await jdbcApi.checkJreUpdate()
  }
  finally {
    jreLoading.value = false
  }
}

async function handleDownloadJre() {
  jreLoading.value = true
  try {
    await jdbcApi.downloadJre()
    await loadJreStatus()
  }
  finally {
    jreLoading.value = false
  }
}

async function handleRemoveJre() {
  jreLoading.value = true
  try {
    await jdbcApi.removeJre()
    jreStatus.value = null
    jreUpdate.value = null
    await loadJreStatus()
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
  try {
    await jdbcApi.downloadBridgeJar()
    await loadBridgeStatus()
  }
  finally {
    bridgeLoading.value = false
  }
}

async function handleRemoveBridge() {
  bridgeLoading.value = true
  try {
    await jdbcApi.removeBridgeJar()
    bridgeStatus.value = null
    await loadBridgeStatus()
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
  downloadingDriver.value = dbType
  try {
    await jdbcApi.downloadDriver(dbType)
    await loadDrivers()
  }
  finally {
    downloadingDriver.value = null
  }
}

async function handleRemoveDriver(dbType: string) {
  removingDriver.value = dbType
  try {
    await jdbcApi.removeDriver(dbType)
    await loadDrivers()
  }
  finally {
    removingDriver.value = null
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
      <!-- JDBC Toggle -->
      <div class="p-4 border rounded-lg flex items-center justify-between">
        <div class="space-y-0.5">
          <p class="text-sm font-medium">
            {{ t('pages.settings.jre.jdbcToggle.title') }}
          </p>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.jre.jdbcToggle.description') }}
          </p>
        </div>
        <div class="flex gap-2 items-center">
          <label
            for="jdbc-toggle"
            class="text-sm text-muted-foreground cursor-pointer select-none"
          >
            {{ t('pages.settings.jre.jdbcToggle.label') }}
          </label>
          <Switch
            id="jdbc-toggle"
            :checked="!jdbcNeeded"
            @update:checked="handleToggleJdbc"
          />
        </div>
      </div>

      <!-- JDBC Disabled Message -->
      <div
        v-if="!jdbcNeeded"
        class="py-8 text-center border rounded-lg border-dashed"
      >
        <p class="text-sm text-muted-foreground">
          {{ t('pages.settings.jre.jdbcToggle.title') }}
        </p>
        <p class="text-xs text-muted-foreground mt-1">
          {{ t('pages.settings.jre.jdbcToggle.description') }}
        </p>
      </div>

      <!-- JDBC Sections (hidden when not needed) -->
      <template v-if="jdbcNeeded">
        <!-- JRE Card -->
        <div class="p-4 border rounded-lg space-y-3">
          <div class="space-y-1">
            <p class="text-sm font-medium">
              {{ t('pages.settings.jre.jreCard.title') }}
            </p>
            <p class="text-xs text-muted-foreground">
              {{ t('pages.settings.jre.jreCard.description') }}
            </p>
          </div>

          <div class="space-y-2">
            <!-- Loading state -->
            <div v-if="jreLoading && !jreStatus" class="flex gap-2 items-center">
              <span class="text-sm text-muted-foreground">{{ t('pages.settings.jre.jreCard.status.checking') }}</span>
            </div>

            <!-- JRE status loaded -->
            <template v-else-if="jreStatus">
              <div class="flex flex-wrap gap-x-4 gap-y-1 items-center">
                <span class="text-sm">
                  {{ t('pages.settings.jre.jreCard.status.installed') }}
                </span>
                <span
                  v-if="jreStatus.version"
                  class="text-sm text-muted-foreground"
                >
                  {{ jreStatus.version }}
                </span>
                <Badge
                  :variant="jreStatus.source === 'managed' ? 'default' : 'secondary'"
                >
                  {{ jreStatus.source === 'managed' ? t('pages.settings.jre.jreCard.status.managed') : t('pages.settings.jre.jreCard.status.system') }}
                </Badge>
                <Badge
                  v-if="jreUpdate?.update_available"
                  variant="warning"
                >
                  {{ t('pages.settings.jre.jreCard.actions.redownload') }}
                </Badge>
              </div>
            </template>

            <!-- Not installed -->
            <div v-else class="text-sm text-muted-foreground">
              {{ t('pages.settings.jre.jreCard.status.notInstalled') }}
            </div>

            <!-- Actions -->
            <div class="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="outline"
                :disabled="jreLoading"
                @click="handleCheckJreUpdates"
              >
                {{ t('pages.settings.jre.jreCard.actions.checkUpdates') }}
              </Button>
              <Button
                v-if="!jreStatus?.installed || jreUpdate?.update_available"
                size="sm"
                variant="outline"
                :disabled="jreLoading"
                @click="handleDownloadJre"
              >
                {{ t('pages.settings.jre.jreCard.actions.download') }}
              </Button>
              <Button
                v-if="jreStatus?.installed && jreStatus?.source === 'managed'"
                size="sm"
                variant="outline"
                :disabled="jreLoading"
                @click="handleRemoveJre"
              >
                {{ t('pages.settings.jre.jreCard.actions.remove') }}
              </Button>
            </div>
          </div>
        </div>

        <!-- Bridge JAR Card -->
        <div class="p-4 border rounded-lg space-y-3">
          <div class="space-y-1">
            <p class="text-sm font-medium">
              {{ t('pages.settings.jre.bridgeCard.title') }}
            </p>
            <p class="text-xs text-muted-foreground">
              {{ t('pages.settings.jre.bridgeCard.description') }}
            </p>
          </div>

          <div class="space-y-2">
            <!-- Loading state -->
            <div v-if="bridgeLoading && !bridgeStatus" class="flex gap-2 items-center">
              <span class="text-sm text-muted-foreground">{{ t('pages.settings.jre.bridgeCard.status.checking') }}</span>
            </div>

            <!-- Bridge status loaded -->
            <template v-else-if="bridgeStatus">
              <div class="flex flex-wrap gap-x-4 gap-y-1 items-center">
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
              </div>
            </template>

            <!-- Fallback -->
            <div v-else class="text-sm text-muted-foreground">
              {{ t('pages.settings.jre.bridgeCard.status.notInstalled') }}
            </div>

            <!-- Actions -->
            <div class="flex flex-wrap gap-2">
              <Button
                v-if="!bridgeStatus?.installed"
                size="sm"
                variant="outline"
                :disabled="bridgeLoading"
                @click="handleDownloadBridge"
              >
                {{ t('pages.settings.jre.bridgeCard.actions.download') }}
              </Button>
              <Button
                v-if="bridgeStatus?.installed"
                size="sm"
                variant="outline"
                :disabled="bridgeLoading"
                @click="handleRemoveBridge"
              >
                {{ t('pages.settings.jre.bridgeCard.actions.remove') }}
              </Button>
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
            <Button
              size="sm"
              variant="outline"
              :disabled="allLoading"
              @click="handleRefresh"
            >
              {{ t('pages.settings.jre.actions.refresh') }}
            </Button>
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
              <div class="flex-shrink-0">
                <Badge
                  :variant="driver.installed ? 'success' : 'secondary'"
                >
                  {{ driver.installed
                    ? t('pages.settings.jre.driversCard.status.installed')
                    : t('pages.settings.jre.driversCard.status.notInstalled') }}
                </Badge>
              </div>
              <div class="flex-shrink-0">
                <Button
                  v-if="!driver.installed"
                  size="sm"
                  variant="outline"
                  :disabled="downloadingDriver === driver.db_type"
                  @click="handleDownloadDriver(driver.db_type)"
                >
                  {{ t('pages.settings.jre.driversCard.actions.download') }}
                </Button>
                <Button
                  v-if="driver.installed"
                  size="sm"
                  variant="outline"
                  :disabled="removingDriver === driver.db_type"
                  @click="handleRemoveDriver(driver.db_type)"
                >
                  {{ t('pages.settings.jre.driversCard.actions.remove') }}
                </Button>
              </div>
            </div>
          </div>
        </div>
      </template>
    </CardContent>
  </Card>
</template>
