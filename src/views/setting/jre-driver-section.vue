<script setup lang="ts">
import type { BridgeStatus, DriverInfo, JreStatus, JreUpdateStatus } from '@/datasources'
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import javaLogo from '@/assets/images/java-logo.svg'
import { SettingsFeedback } from '@/components/settings'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { useDownloadEvents } from '@/composables'
import { useDatabaseIcon } from '@/composables/useDatabaseIcon'
import { jdbcApi } from '@/datasources'
import { dbTypeFromBackend } from '@/store'

type Task = { state: 'idle' | 'pending' | 'success' | 'error', label: string, detail?: string }
const { t } = useI18n()
const { getDatabaseIcon } = useDatabaseIcon()
const dl = useDownloadEvents()
const jreStatus = ref<JreStatus | null>(null)
const jreUpdate = ref<JreUpdateStatus | null>(null)
const bridgeStatus = ref<BridgeStatus | null>(null)
const drivers = ref<DriverInfo[]>([])
const loading = reactive({ jre: true, bridge: true, drivers: true })
const loadErrors = reactive({ jre: '', bridge: '', drivers: '' })
const tasks = reactive<Record<string, Task>>({})
const idle: Task = { state: 'idle', label: '' }
const feedback = (key: string) => t(`pages.settings.feedback.${key}`)
const message = (error: unknown) => error instanceof Error ? error.message : String(error)
const task = (id: string): Task => tasks[id] ?? idle
const busy = (id: string) => task(id).state === 'pending' || dl.isDownloading(id)
const anyBusy = computed(() => Object.values(tasks).some(task => task.state === 'pending') || Object.values(loading).some(Boolean) || Object.values(dl.states).some(state => ['preparing', 'downloading', 'installing'].includes(state)))
const systemJreValid = computed(() => jreStatus.value?.source === 'system' && Number.parseInt(jreStatus.value.version ?? '', 10) >= 25)
const sortedDrivers = computed(() => [...drivers.value].sort((a, b) => Number(b.installed) - Number(a.installed) || a.name.localeCompare(b.name)))
const getDriverIcon = (type: string) => dbTypeFromBackend[type] ? getDatabaseIcon(dbTypeFromBackend[type]) : ''
const formatFileSize = (bytes: number) => bytes < 1024 ? `${bytes} B` : bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / (1024 * 1024)).toFixed(1)} MB`

const runTask = async (id: string, label: string, operation: () => Promise<void | string>, done = 'saved') => {
  if (busy(id))
    return
  tasks[id] = { state: 'pending', label }
  try {
    const result = await operation()
    tasks[id] = { state: 'success', label: result ?? feedback(done) }
  }
  catch (error) {
    tasks[id] = { state: 'error', label: feedback('failed'), detail: message(error) }
  }
}

const load = async (id: keyof typeof loading, operation: () => Promise<void>) => {
  loading[id] = true
  loadErrors[id] = ''
  try {
    await operation()
    return true
  }
  catch (error) {
    loadErrors[id] = message(error)
    return false
  }
  finally {
    loading[id] = false
  }
}
const loadJreStatus = () => load('jre', async () => {
  jreStatus.value = await jdbcApi.checkJreStatus()
})
const loadBridgeStatus = () => load('bridge', async () => {
  bridgeStatus.value = await jdbcApi.checkBridgeStatus()
})
const loadDrivers = () => load('drivers', async () => {
  drivers.value = await jdbcApi.listDrivers()
})
const loadAll = () => Promise.all([loadJreStatus(), loadBridgeStatus(), loadDrivers()])
const handleRefresh = () => runTask('refresh', feedback('refreshing'), async () => {
  const results = await loadAll()
  if (results.some(result => !result))
    throw new Error(feedback('loadFailed'))
}, 'refreshed')

const download = async (id: string, kind: 'jre' | 'bridge' | 'driver', invoke: () => Promise<void>, reload: () => Promise<boolean>) => {
  dl.reset(id)
  const ok = await dl.startDownload(kind, id, invoke)
  if (!ok)
    throw new Error(dl.getError(id) ?? feedback('failed'))
  tasks[id] = { state: 'pending', label: feedback('installing') }
  if (!await reload())
    throw new Error(feedback('loadFailed'))
}
const handleDownloadJre = () => runTask('jre', feedback('preparing'), () => download('jre', 'jre', () => jdbcApi.downloadJre(), loadJreStatus), 'complete')
const handleDownloadBridge = () => runTask('bridge', feedback('preparing'), () => download('bridge', 'bridge', () => jdbcApi.downloadBridgeJar(), loadBridgeStatus), 'complete')
const handleDownloadDriver = (id: string) => runTask(id, feedback('preparing'), async () => {
  if (!bridgeStatus.value?.installed)
    throw new Error(t('pages.settings.jre.driversCard.notifications.installBridgeFirst'))
  await download(id, 'driver', () => jdbcApi.downloadDriver(id), loadDrivers)
}, 'complete')
const handleRemoveJre = () => runTask('jre', feedback('removing'), async () => {
  await jdbcApi.removeJre()
  jreUpdate.value = null
  if (!await loadJreStatus())
    throw new Error(feedback('loadFailed'))
}, 'removed')
const handleRemoveBridge = () => runTask('bridge', feedback('removing'), async () => {
  await jdbcApi.removeBridgeJar()
  if (!await loadBridgeStatus())
    throw new Error(feedback('loadFailed'))
}, 'removed')
const handleRemoveDriver = (id: string) => runTask(id, feedback('removing'), async () => {
  await jdbcApi.removeDriver(id)
  if (!await loadDrivers())
    throw new Error(feedback('loadFailed'))
}, 'removed')
const handleCheckJreUpdates = () => runTask('jre', feedback('checking'), async () => {
  const update = await jdbcApi.checkJreUpdate()
  jreUpdate.value = update
  dl.reset('jre')
  if (update.update_available)
    await download('jre', 'jre', () => jdbcApi.downloadJre(), loadJreStatus)
  return t(`pages.settings.jre.jreCard.notifications.${update.update_available ? 'downloadSuccess' : 'upToDate'}`)
}, 'refreshed')
const handleCheckDriverUpdates = (id: string) => runTask(id, feedback('checking'), async () => {
  const update = await jdbcApi.checkDriverUpdate(id)
  dl.reset(id)
  if (update.update_available)
    await download(id, 'driver', () => jdbcApi.downloadDriver(id), loadDrivers)
  return t(`pages.settings.jre.driversCard.notifications.${update.update_available ? 'downloadSuccess' : 'upToDate'}`)
}, 'refreshed')

const downloadFeedback = (id: string): Task & { progress: number | null } => {
  const state = dl.getState(id)
  const current = task(id)
  const progress = dl.getProgress(id)
  if (!dl.isDownloading(id)) {
    if (current.state === 'idle' && state === 'error')
      return { state: 'error', label: feedback('failed'), detail: dl.getError(id) ?? undefined, progress: null }
    if (current.state === 'idle' && state === 'complete')
      return { state: 'success', label: feedback('complete'), progress: null }
    return { ...current, progress: null }
  }
  const total = progress?.total ?? 0
  const downloaded = progress?.downloaded ?? 0
  return {
    state: 'pending',
    label: feedback(state === 'installing' ? 'installing' : state === 'downloading' ? 'downloading' : 'preparing'),
    detail: progress ? `${formatFileSize(downloaded)}${total > 0 ? ` / ${formatFileSize(total)}` : ''}` : undefined,
    progress: state === 'downloading' && total > 0 && downloaded < total ? downloaded / total * 100 : null,
  }
}

watch(() => ({ ...dl.states }), (states, previous) => {
  Object.entries(states).filter(([id, state]) => state === 'complete' && previous[id] !== 'complete' && task(id).state === 'idle').forEach(([id]) => {
    void (id === 'jre' ? loadJreStatus() : id === 'bridge' ? loadBridgeStatus() : loadDrivers())
  })
})

onMounted(() => {
  void loadAll()
})
</script>

<template>
  <Card>
    <CardHeader class="flex-row gap-4 items-start justify-between">
      <div class="space-y-1.5">
        <CardTitle>{{ t('pages.settings.jre.title') }}</CardTitle>
        <CardDescription>{{ t('pages.settings.jre.description') }}</CardDescription>
      </div>
      <Button size="sm" variant="outline" :disabled="anyBusy" :aria-busy="task('refresh').state === 'pending'" @click="handleRefresh">
        <span class="i-lucide-refresh-cw mr-2 h-3.5 w-3.5" :class="{ 'settings-action-spin': task('refresh').state === 'pending' }" aria-hidden="true" />
        {{ t('pages.settings.jre.actions.refresh') }}
      </Button>
    </CardHeader>
    <CardContent class="space-y-6">
      <SettingsFeedback v-bind="task('refresh')" />
      <section class="space-y-3" :aria-label="t('pages.settings.jre.jreCard.title')">
        <div class="space-y-1">
          <h3 class="text-sm font-medium">
            {{ t('pages.settings.jre.jreCard.title') }}
          </h3>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.jre.jreCard.description') }}
          </p>
        </div>
        <SettingsFeedback v-if="loading.jre && !jreStatus" state="pending" :label="t('pages.settings.jre.jreCard.status.checking')" />
        <div v-else-if="jreStatus" class="border rounded-md divide-y">
          <div class="px-4 py-3 space-y-3">
            <div class="flex flex-wrap gap-3 items-center">
              <img :src="javaLogo" alt="" class="shrink-0 h-5 w-5 object-contain">
              <div class="flex-1 min-w-0">
                <p class="text-sm font-medium">
                  {{ t('pages.settings.jre.jreCard.status.managed') }}
                </p>
                <p v-if="jreStatus.source === 'managed'" class="text-xs text-muted-foreground mt-0.5 break-all">
                  {{ jreStatus.version }} · {{ jreStatus.path }}
                </p>
              </div>
              <Badge variant="secondary" class="runtime-badge" :class="{ 'is-installed': jreStatus.source === 'managed' }">
                {{ t(`pages.settings.jre.jreCard.status.${jreStatus.source === 'managed' ? 'installed' : 'notInstalled'}`) }}
              </Badge>
              <div class="flex gap-1 items-center">
                <Button size="sm" variant="ghost" :disabled="busy('jre') || loading.jre" :aria-label="t('pages.settings.jre.jreCard.actions.checkUpdates')" :title="t('pages.settings.jre.jreCard.actions.checkUpdates')" @click="handleCheckJreUpdates">
                  <span class="i-lucide-refresh-cw h-4 w-4" aria-hidden="true" />
                </Button>
                <Button v-if="jreStatus.source !== 'managed' || jreUpdate?.update_available" size="sm" variant="outline" :disabled="busy('jre') || loading.jre" @click="handleDownloadJre">
                  <span class="i-lucide-download mr-2 h-3.5 w-3.5" aria-hidden="true" />{{ t('pages.settings.jre.jreCard.actions.download') }}
                </Button>
                <Button v-if="jreStatus.source === 'managed'" size="sm" variant="ghost" :disabled="anyBusy" :aria-label="t('pages.settings.jre.jreCard.actions.remove')" :title="t('pages.settings.jre.jreCard.actions.remove')" @click="handleRemoveJre">
                  <span class="i-lucide-trash-2 h-4 w-4" aria-hidden="true" />
                </Button>
              </div>
            </div>
            <SettingsFeedback v-bind="downloadFeedback('jre')" />
          </div>
          <div class="px-4 py-3 flex gap-3 items-center">
            <span class="i-lucide-laptop text-muted-foreground shrink-0 h-5 w-5" aria-hidden="true" />
            <div class="flex-1 min-w-0">
              <p class="text-sm font-medium">
                {{ t('pages.settings.jre.jreCard.status.system') }}
              </p>
              <p class="text-xs text-muted-foreground mt-0.5 break-all">
                {{ systemJreValid ? `${jreStatus.version} · ${jreStatus.path}` : t('pages.settings.jre.jreCard.status.systemInvalid') }}
              </p>
            </div>
            <Badge variant="secondary" class="runtime-badge" :class="{ 'is-installed': systemJreValid }">
              {{ t(`pages.settings.jre.jreCard.status.${systemJreValid ? 'installed' : 'notInstalled'}`) }}
            </Badge>
          </div>
        </div>
        <div v-if="loadErrors.jre" class="space-y-2">
          <SettingsFeedback state="error" :label="feedback('loadFailed')" :detail="loadErrors.jre" /><Button size="sm" variant="outline" :disabled="loading.jre" @click="loadJreStatus">
            {{ feedback('retry') }}
          </Button>
        </div>
      </section>

      <section class="pt-5 border-t space-y-3" :aria-label="t('pages.settings.jre.bridgeCard.title')">
        <div class="space-y-1">
          <h3 class="text-sm font-medium">
            {{ t('pages.settings.jre.bridgeCard.title') }}
          </h3><p class="text-xs text-muted-foreground">
            {{ t('pages.settings.jre.bridgeCard.description') }}
          </p>
        </div>
        <SettingsFeedback v-if="loading.bridge && !bridgeStatus" state="pending" :label="t('pages.settings.jre.bridgeCard.status.checking')" />
        <div v-else-if="bridgeStatus" class="flex flex-wrap gap-3 items-center">
          <Badge variant="secondary" class="runtime-badge" :class="{ 'is-installed': bridgeStatus.installed }">
            {{ t(`pages.settings.jre.bridgeCard.status.${bridgeStatus.installed ? 'installed' : 'notInstalled'}`) }}
          </Badge>
          <span v-if="bridgeStatus.installed" class="text-xs text-muted-foreground flex-1">{{ bridgeStatus.current_version }}</span>
          <Button v-if="!bridgeStatus.installed" size="sm" variant="outline" :disabled="busy('bridge') || loading.bridge" @click="handleDownloadBridge">
            <span class="i-lucide-download mr-2 h-3.5 w-3.5" aria-hidden="true" />{{ t('pages.settings.jre.bridgeCard.actions.download') }}
          </Button>
          <Button v-else size="sm" variant="ghost" :disabled="anyBusy" :aria-label="t('pages.settings.jre.bridgeCard.actions.remove')" :title="t('pages.settings.jre.bridgeCard.actions.remove')" @click="handleRemoveBridge">
            <span class="i-lucide-trash-2 h-4 w-4" aria-hidden="true" />
          </Button>
        </div>
        <SettingsFeedback v-bind="downloadFeedback('bridge')" />
        <div v-if="loadErrors.bridge" class="space-y-2">
          <SettingsFeedback state="error" :label="feedback('loadFailed')" :detail="loadErrors.bridge" /><Button size="sm" variant="outline" :disabled="loading.bridge" @click="loadBridgeStatus">
            {{ feedback('retry') }}
          </Button>
        </div>
      </section>

      <section class="pt-5 border-t space-y-3" :aria-label="t('pages.settings.jre.driversCard.title')">
        <div class="space-y-1">
          <h3 class="text-sm font-medium">
            {{ t('pages.settings.jre.driversCard.title') }}
          </h3><p class="text-xs text-muted-foreground">
            {{ t('pages.settings.jre.driversCard.description') }}
          </p>
        </div>
        <SettingsFeedback v-if="loading.drivers" state="pending" :label="t('pages.settings.jre.driversCard.status.loading')" />
        <div v-if="loadErrors.drivers" class="space-y-2">
          <SettingsFeedback state="error" :label="feedback('loadFailed')" :detail="loadErrors.drivers" /><Button size="sm" variant="outline" :disabled="loading.drivers" @click="loadDrivers">
            {{ feedback('retry') }}
          </Button>
        </div>
        <div v-if="drivers.length" class="border rounded-md divide-y">
          <div v-for="driver in sortedDrivers" :key="driver.db_type" class="px-4 py-3 space-y-3">
            <div class="flex flex-wrap gap-3 items-center">
              <img :src="getDriverIcon(driver.db_type)" alt="" class="shrink-0 h-5 w-5 object-contain">
              <div class="flex-1 min-w-0">
                <p class="text-sm font-medium">
                  {{ driver.name }}
                </p><p v-if="driver.installed && driver.filename" class="text-xs text-muted-foreground mt-0.5 break-all">
                  {{ driver.filename }}<template v-if="driver.file_size != null">
                    · {{ formatFileSize(driver.file_size) }}
                  </template><template v-if="driver.resolved_version">
                    · v{{ driver.resolved_version }}
                  </template>
                </p>
              </div>
              <Badge variant="secondary" class="runtime-badge" :class="{ 'is-installed': driver.installed }">
                {{ t(`pages.settings.jre.driversCard.status.${driver.installed ? 'installed' : 'notInstalled'}`) }}
              </Badge>
              <div class="flex gap-1 items-center">
                <Button v-if="driver.installed" size="sm" variant="ghost" :disabled="busy(driver.db_type) || loading.drivers" :aria-label="`${driver.name}: ${t('pages.settings.jre.driversCard.actions.checkUpdates')}`" :title="t('pages.settings.jre.driversCard.actions.checkUpdates')" @click="handleCheckDriverUpdates(driver.db_type)">
                  <span class="i-lucide-refresh-cw h-4 w-4" aria-hidden="true" />
                </Button>
                <Button v-if="!driver.installed" size="sm" variant="ghost" :disabled="busy(driver.db_type) || loading.drivers || !bridgeStatus?.installed || busy('bridge')" :title="!bridgeStatus?.installed ? t('pages.settings.jre.driversCard.notifications.installBridgeFirst') : undefined" :aria-label="`${driver.name}: ${t('pages.settings.jre.driversCard.actions.download')}`" @click="handleDownloadDriver(driver.db_type)">
                  <span class="i-lucide-download mr-2 h-3.5 w-3.5" aria-hidden="true" />{{ t('pages.settings.jre.driversCard.actions.download') }}
                </Button>
                <Button v-else size="sm" variant="ghost" :disabled="busy(driver.db_type) || loading.drivers" :aria-label="`${driver.name}: ${t('pages.settings.jre.driversCard.actions.remove')}`" :title="t('pages.settings.jre.driversCard.actions.remove')" @click="handleRemoveDriver(driver.db_type)">
                  <span class="i-lucide-trash-2 h-4 w-4" aria-hidden="true" />
                </Button>
              </div>
            </div>
            <SettingsFeedback v-bind="downloadFeedback(driver.db_type)" />
          </div>
        </div>
        <p v-else-if="!loading.drivers && !loadErrors.drivers" class="text-sm text-muted-foreground py-4">
          {{ t('pages.settings.jre.driversCard.empty.title') }} · {{ t('pages.settings.jre.driversCard.empty.message') }}
        </p>
      </section>
    </CardContent>
  </Card>
</template>

<style scoped>
.runtime-badge.is-installed { background: hsl(var(--success-muted)); color: hsl(var(--foreground)); }
.settings-action-spin { animation: settings-action-spin 900ms linear infinite; }
@keyframes settings-action-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) { .settings-action-spin { animation: none; } }
</style>
