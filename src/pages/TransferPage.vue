<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import AppLayout from '@/components/layout/AppLayout.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useTransferStore } from '@/store/transferStore'

const { t } = useI18n()
const transferStore = useTransferStore()

const activeTab = ref('active')

onMounted(async () => {
  try {
    await transferStore.loadProfiles()
  }
  catch (error) {
    console.error('Failed to load transfer profiles', error)
  }
})

const activeJobs = computed(() => {
  return transferStore.jobs.filter(j => j.status === 'queued' || j.status === 'running')
})

const historyJobs = computed(() => {
  return transferStore.jobs.filter(j => j.status === 'completed' || j.status === 'failed' || j.status === 'cancelled')
})

const groupedHistoryJobs = computed(() => {
  const groups: Record<string, typeof historyJobs.value> = {}

  for (const job of historyJobs.value) {
    const d = new Date(job.startedAt)
    const dateStr = d.toLocaleDateString()
    if (!groups[dateStr]) {
      groups[dateStr] = []
    }
    groups[dateStr].push(job)
  }

  return Object.entries(groups)
    .sort((a, b) => new Date(b[0]).getTime() - new Date(a[0]).getTime())
    .map(([date, jobs]) => {
      return {
        date,
        jobs: jobs.sort((a, b) => b.startedAt - a.startedAt),
      }
    })
})

const savedProfiles = computed(() => transferStore.savedProfiles)

function handleCancelJob(id: string) {
  transferStore.cancelJob(id)
}

function handleDismissJob(id: string) {
  transferStore.dismissJob(id)
}

async function handleRunProfile(id: string) {
  try {
    await transferStore.runProfile(id)
    activeTab.value = 'active'
  }
  catch (err) {
    console.error('Failed to run profile', err)
  }
}

function handleEditProfile(id: string) {
  console.warn('TODO: edit profile', id)
}

function handleDeleteProfile(id: string) {
  // eslint-disable-next-line no-alert
  if (confirm('Delete this profile?')) {
    console.warn('TODO: delete profile', id)
  }
}

function formatDuration(start: number, end?: number) {
  if (!end)
    return '-'
  const ms = end - start
  if (ms < 1000)
    return `${ms}ms`
  const s = Math.floor(ms / 1000)
  if (s < 60)
    return `${s}s`
  const m = Math.floor(s / 60)
  return `${m}m ${s % 60}s`
}

function formatTime(ts?: number) {
  if (!ts)
    return '-'
  return new Date(ts).toLocaleTimeString()
}

function formatDate(ts?: number) {
  if (!ts)
    return t('transfer.activityCenter.profile.never')
  return new Date(ts).toLocaleString()
}
</script>

<template>
  <AppLayout>
    <div class="bg-background flex flex-col h-full">
      <header class="px-6 py-4 border-b border-border/40 bg-card flex shrink-0 items-center justify-between">
        <div>
          <h1 class="text-xl tracking-tight font-semibold">
            {{ t('transfer.activityCenter.title') }}
          </h1>
        </div>
      </header>

      <div class="p-6 flex flex-1 flex-col min-h-0 overflow-hidden">
        <Tabs v-model="activeTab" class="mx-auto flex flex-1 flex-col max-w-5xl min-h-0 w-full">
          <TabsList class="grid grid-cols-3 max-w-[400px]">
            <TabsTrigger value="active">
              {{ t('transfer.activityCenter.tabs.active') }}
              <Badge v-if="activeJobs.length" variant="secondary" class="ml-2 px-1.5 py-0 flex h-5 min-w-[20px] items-center justify-center">
                {{ activeJobs.length }}
              </Badge>
            </TabsTrigger>
            <TabsTrigger value="history">
              {{ t('transfer.activityCenter.tabs.history') }}
              <Badge v-if="historyJobs.length" variant="secondary" class="ml-2 px-1.5 py-0 flex h-5 min-w-[20px] items-center justify-center">
                {{ historyJobs.length }}
              </Badge>
            </TabsTrigger>
            <TabsTrigger value="profiles">
              {{ t('transfer.activityCenter.tabs.savedProfiles') }}
              <Badge v-if="savedProfiles.length" variant="secondary" class="ml-2 px-1.5 py-0 flex h-5 min-w-[20px] items-center justify-center">
                {{ savedProfiles.length }}
              </Badge>
            </TabsTrigger>
          </TabsList>

          <div class="mt-6 pr-2 flex-1 overflow-y-auto">
            <!-- Active Jobs Tab -->
            <TabsContent value="active" class="m-0 h-full focus-visible:outline-none">
              <div v-if="activeJobs.length === 0" class="text-muted-foreground border rounded-lg border-dashed bg-muted/20 flex flex-col h-40 items-center justify-center">
                {{ t('transfer.activityCenter.empty.active') }}
              </div>
              <div v-else class="space-y-4">
                <Card v-for="job in activeJobs" :key="job.id" class="overflow-hidden">
                  <CardContent class="p-4 flex gap-6 items-center">
                    <div class="flex-1 min-w-0 space-y-2">
                      <div class="flex items-center justify-between">
                        <div class="flex gap-2 items-center overflow-hidden">
                          <span class="text-sm font-medium truncate" :title="job.name">{{ job.name }}</span>
                          <Badge variant="outline" class="text-[10px] shrink-0 uppercase">
                            {{ job.scope }}
                          </Badge>
                        </div>
                        <span class="text-xs text-muted-foreground shrink-0">{{ job.progress.stage }}</span>
                      </div>
                      <div class="rounded-full bg-muted h-2 w-full relative overflow-hidden">
                        <div
                          class="bg-primary h-full transition-all duration-300 left-0 top-0 absolute"
                          :style="{ width: `${job.progress.total > 0 ? (job.progress.current / job.progress.total) * 100 : 0}%` }"
                        />
                      </div>
                      <div class="text-xs text-muted-foreground flex justify-between">
                        <span>{{ job.progress.current }} / {{ job.progress.total }}</span>
                        <span v-if="job.progress.etaMs">{{ Math.ceil(job.progress.etaMs / 1000) }}s remaining</span>
                      </div>
                    </div>
                    <Button variant="secondary" size="sm" class="shrink-0" @click="handleCancelJob(job.id)">
                      Cancel
                    </Button>
                  </CardContent>
                </Card>
              </div>
            </TabsContent>

            <!-- History Tab -->
            <TabsContent value="history" class="m-0 h-full focus-visible:outline-none">
              <div v-if="historyJobs.length === 0" class="text-muted-foreground border rounded-lg border-dashed bg-muted/20 flex flex-col h-40 items-center justify-center">
                {{ t('transfer.activityCenter.empty.history') }}
              </div>
              <div v-else class="pb-4 space-y-8">
                <div v-for="group in groupedHistoryJobs" :key="group.date" class="space-y-3">
                  <h3 class="text-sm text-muted-foreground font-medium py-1 bg-background/95 top-0 sticky z-10 backdrop-blur-sm">
                    {{ group.date }}
                  </h3>
                  <div class="space-y-2">
                    <Card v-for="job in group.jobs" :key="job.id" class="shadow-sm">
                      <CardContent class="p-4 flex gap-4 items-center">
                        <div
                          class="rounded-full flex shrink-0 h-8 w-8 items-center justify-center"
                          :class="{
                            'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400': job.status === 'completed',
                            'bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400': job.status === 'failed',
                            'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400': job.status === 'cancelled',
                          }"
                        >
                          <span v-if="job.status === 'completed'" class="i-carbon-checkmark text-lg" />
                          <span v-else-if="job.status === 'failed'" class="i-carbon-close text-lg" />
                          <span v-else class="i-carbon-stop-sign text-lg" />
                        </div>
                        <div class="flex-1 min-w-0">
                          <div class="flex gap-2 items-center">
                            <h4 class="text-sm font-medium truncate">
                              {{ job.name }}
                            </h4>
                            <Badge variant="outline" class="text-[10px] shrink-0 uppercase">
                              {{ job.scope }}
                            </Badge>
                          </div>
                          <div class="text-xs text-muted-foreground mt-0.5 flex gap-3 items-center">
                            <span class="flex gap-1 items-center">
                              <span class="i-carbon-time" />
                              {{ formatTime(job.startedAt) }}
                            </span>
                            <span class="flex gap-1 items-center">
                              <span class="i-carbon-timer" />
                              {{ formatDuration(job.startedAt, job.finishedAt) }}
                            </span>
                          </div>
                          <div v-if="job.error" class="text-xs text-red-500 mt-1 truncate" :title="job.error">
                            {{ t('transfer.activityCenter.job.error') }}: {{ job.error }}
                          </div>
                        </div>
                        <div class="flex shrink-0 gap-2 items-center">
                          <Button variant="ghost" size="sm" class="text-xs">
                            {{ t('transfer.activityCenter.job.viewLogs') }}
                          </Button>
                          <Button variant="ghost" size="icon" class="h-8 w-8" @click="handleDismissJob(job.id)">
                            <span class="i-carbon-close" />
                          </Button>
                        </div>
                      </CardContent>
                    </Card>
                  </div>
                </div>
              </div>
            </TabsContent>

            <!-- Saved Profiles Tab -->
            <TabsContent value="profiles" class="m-0 h-full focus-visible:outline-none">
              <div v-if="savedProfiles.length === 0" class="text-muted-foreground border rounded-lg border-dashed bg-muted/20 flex flex-col h-40 items-center justify-center">
                {{ t('transfer.activityCenter.empty.profiles') }}
              </div>
              <div v-else class="pb-4 gap-4 grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3">
                <Card v-for="profile in savedProfiles" :key="profile.id" class="flex flex-col">
                  <CardHeader class="pb-2">
                    <div class="flex gap-2 items-start justify-between">
                      <CardTitle class="text-base flex-1 truncate" :title="profile.name">
                        {{ profile.name }}
                      </CardTitle>
                      <Badge variant="secondary" class="text-[10px] shrink-0 uppercase">
                        {{ profile.kind }}
                      </Badge>
                    </div>
                    <CardDescription class="text-xs mt-1">
                      {{ t('transfer.activityCenter.profile.lastRun') }}: {{ formatDate(profile.lastRunAt) }}
                    </CardDescription>
                  </CardHeader>
                  <CardContent class="pb-4 flex-1">
                    <div class="text-xs text-muted-foreground flex gap-1.5 items-center">
                      <span class="i-carbon-network-4" />
                      <span class="truncate">{{ profile.connectionId }}</span>
                    </div>
                  </CardContent>
                  <div class="p-4 pt-0 flex shrink-0 gap-2 justify-end">
                    <Button variant="ghost" size="sm" @click="handleEditProfile(profile.id)">
                      {{ t('transfer.activityCenter.profile.edit') }}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      class="text-destructive hover:text-destructive hover:bg-destructive/10"
                      @click="handleDeleteProfile(profile.id)"
                    >
                      {{ t('transfer.activityCenter.profile.delete') }}
                    </Button>
                    <Button size="sm" @click="handleRunProfile(profile.id)">
                      <span class="i-carbon-play filled mr-1" />
                      {{ t('transfer.activityCenter.profile.run') }}
                    </Button>
                  </div>
                </Card>
              </div>
            </TabsContent>
          </div>
        </Tabs>
      </div>
    </div>
  </AppLayout>
</template>
