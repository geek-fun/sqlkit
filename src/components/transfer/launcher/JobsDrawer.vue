<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'

import { Button } from '@/components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useTransferStore } from '@/store/transferStore'

const { t } = useI18n()
const store = useTransferStore()

const isExpanded = ref(false)
const activeTab = ref('running')

const activeJobs = computed(() => store.jobs.filter(j => j.status === 'queued' || j.status === 'running'))
const historyJobs = computed(() => store.jobs.filter(j => j.status === 'completed' || j.status === 'failed' || j.status === 'cancelled').sort((a, b) => (b.startedAt || 0) - (a.startedAt || 0)))

watch(() => activeJobs.value.length, (newLen, oldLen) => {
  if (newLen > oldLen && newLen > 0) {
    isExpanded.value = true
    activeTab.value = 'running'
  }
})

const overallProgress = computed(() => {
  if (!activeJobs.value.length)
    return 0
  const jobsWithProgress = activeJobs.value.filter(j => j.progress && j.progress.total > 0)
  if (!jobsWithProgress.length)
    return 0

  const totalPercent = jobsWithProgress.reduce((acc, job) => {
    return acc + (job.progress.current / job.progress.total) * 100
  }, 0)
  return Math.round(totalPercent / jobsWithProgress.length)
})

const handleCancel = (id: string) => store.cancelJob(id)

function formatDuration(start: number, end?: number) {
  if (!end)
    return '-'
  const ms = end - start
  if (ms < 1000)
    return `${ms}ms`
  const s = Math.floor(ms / 1000)
  if (s < 60)
    return `${s}s`
  return `${Math.floor(s / 60)}m ${s % 60}s`
}

function toggleExpanded() {
  isExpanded.value = !isExpanded.value
}
</script>

<template>
  <div v-if="activeJobs.length > 0 || historyJobs.length > 0" class="flex pointer-events-none bottom-0 left-0 right-0 justify-center fixed z-50">
    <div class="flex flex-col max-w-4xl w-full pointer-events-auto" :class="isExpanded ? 'h-[320px]' : 'h-auto'">
      <!-- Expanded Panel -->
      <div v-if="isExpanded" class="transfer-activity-drawer border border-b-0 rounded-t-lg flex flex-1 flex-col min-h-0 transition-all duration-300 overflow-hidden">
        <Tabs v-model="activeTab" class="flex flex-col h-full">
          <div class="px-3 py-1.5 border-b flex shrink-0 items-center justify-between">
            <TabsList class="grid grid-cols-2 w-[240px]">
              <TabsTrigger value="running">
                {{ t('transfer.launcher.tabs.running') }}
                <Badge v-if="activeJobs.length" variant="secondary" class="text-[10px] ml-2 px-1 py-0 h-4">
                  {{ activeJobs.length }}
                </Badge>
              </TabsTrigger>
              <TabsTrigger value="history">
                {{ t('transfer.launcher.tabs.history') }}
                <Badge v-if="historyJobs.length" variant="secondary" class="text-[10px] ml-2 px-1 py-0 h-4">
                  {{ historyJobs.length }}
                </Badge>
              </TabsTrigger>
            </TabsList>
            <button class="transfer-action-btn p-1 flex h-6 w-6 items-center justify-center" @click="toggleExpanded">
              <span class="i-carbon-chevron-up text-xs" />
            </button>
          </div>

          <div class="transfer-scroll p-3 flex-1 overflow-y-auto">
            <!-- Running Tab -->
            <TabsContent value="running" class="m-0 h-full">
              <div v-if="activeJobs.length === 0" class="text-sm text-muted-foreground flex h-full items-center justify-center">
                {{ t('transfer.launcher.noRunningJobs') }}
              </div>
              <div v-else class="space-y-2">
                <div v-for="job in activeJobs" :key="job.id" class="transfer-card p-3">
                  <div class="mb-2 flex items-center justify-between">
                    <div class="flex gap-2 items-center">
                      <span class="transfer-status-dot running" />
                      <span class="text-sm font-medium max-w-[300px] truncate">{{ job.name }}</span>
                      <Badge variant="outline" class="text-[10px] h-5 uppercase">
                        {{ job.kind }}
                      </Badge>
                    </div>
                    <Button variant="ghost" size="sm" class="text-xs text-muted-foreground h-6 hover:text-foreground" @click="handleCancel(job.id)">
                      {{ t('common.cancel') }}
                    </Button>
                  </div>

                  <div class="text-xs text-muted-foreground mb-1 flex items-center justify-between">
                    <span>{{ job.progress.stage }}</span>
                    <span>{{ job.progress.total > 0 ? Math.round((job.progress.current / job.progress.total) * 100) : 0 }}%</span>
                  </div>
                  <div class="rounded-full bg-muted h-1.5 w-full overflow-hidden">
                    <div
                      class="bg-primary h-full transition-all duration-300"
                      :style="{ width: `${job.progress.total > 0 ? (job.progress.current / job.progress.total) * 100 : 0}%` }"
                    />
                  </div>
                </div>
              </div>
            </TabsContent>

            <!-- History Tab -->
            <TabsContent value="history" class="m-0 h-full">
              <div v-if="historyJobs.length === 0" class="text-sm text-muted-foreground flex h-full items-center justify-center">
                {{ t('transfer.launcher.noHistoryJobs') }}
              </div>
              <div v-else class="space-y-1.5">
                <div v-for="job in historyJobs" :key="job.id" class="transfer-card p-2.5 flex gap-3 items-center">
                  <div class="flex shrink-0 h-6 w-6 items-center justify-center">
                    <span v-if="job.status === 'completed'" class="transfer-status-dot completed text-sm" />
                    <span v-else-if="job.status === 'failed'" class="transfer-status-dot failed text-sm" />
                    <span v-else class="transfer-status-dot text-sm text-muted-foreground" />
                    <span v-if="job.status === 'completed'" class="i-carbon-checkmark absolute" />
                    <span v-else-if="job.status === 'failed'" class="i-carbon-close text-[10px] absolute" />
                    <span v-else class="i-carbon-stop-sign text-[10px] absolute" />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="flex gap-2 items-center">
                      <span class="text-sm font-medium truncate">{{ job.name }}</span>
                      <span class="text-xs text-muted-foreground shrink-0">{{ formatDuration(job.startedAt || 0, job.finishedAt) }}</span>
                    </div>
                    <div v-if="job.error" class="text-xs text-destructive truncate">
                      {{ job.error }}
                    </div>
                  </div>
                </div>
              </div>
            </TabsContent>
          </div>
        </Tabs>
      </div>

      <!-- Activity Bar (always visible handle) -->
      <div class="transfer-activity-bar px-4 rounded-t-lg flex shrink-0 h-9 cursor-pointer transition-colors items-center justify-between" :class="{ 'rounded-t-none rounded-b-lg': isExpanded }" @click="toggleExpanded">
        <div class="flex gap-2 items-center">
          <template v-if="activeJobs.length">
            <span class="transfer-status-dot running animate-pulse" />
            <span class="text-xs font-medium font-mono">
              {{ t('transfer.launcher.jobsRunning', { count: activeJobs.length }) }}
            </span>
            <span class="text-xs text-muted-foreground font-mono">{{ overallProgress }}%</span>
          </template>
          <template v-else>
            <span class="transfer-status-dot completed" />
            <span class="text-xs font-medium font-mono">
              {{ t('transfer.launcher.allJobsCompleted') }}
            </span>
          </template>
        </div>
        <div class="flex gap-2 items-center">
          <span class="text-xs transition-transform duration-200" :class="isExpanded ? 'rotate-180 i-carbon-chevron-down' : 'i-carbon-chevron-up'" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.transfer-status-dot.text-sm {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  position: relative;
}
.transfer-status-dot.text-sm[class*="i-carbon"] {
  display: inline;
}
</style>
