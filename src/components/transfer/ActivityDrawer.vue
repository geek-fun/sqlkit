<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { useTransferStore } from '@/store/transferStore'

const { t } = useI18n()
const router = useRouter()
const transferStore = useTransferStore()
const { jobs } = storeToRefs(transferStore)

const isExpanded = ref(false)
const pinnedJobIds = ref<Set<string>>(new Set())

const runningJobs = computed(() => jobs.value.filter(j => ['queued', 'running', 'paused'].includes(j.status)))
const runningJobCount = computed(() => runningJobs.value.length)
const isHidden = computed(() => jobs.value.length === 0)

const aggregatePercent = computed(() => {
  if (runningJobs.value.length === 0)
    return 100
  let total = 0
  let current = 0
  runningJobs.value.forEach((job) => {
    total += job.progress.total || 1
    current += job.progress.current || 0
  })
  return total > 0 ? Math.round((current / total) * 100) : 0
})

function getStatusVariant(status: string) {
  switch (status) {
    case 'running': return 'default'
    case 'completed': return 'success'
    case 'failed': return 'destructive'
    default: return 'secondary'
  }
}

function formatEta(etaMs?: number) {
  if (etaMs === undefined || etaMs < 0)
    return ''
  const secs = Math.round(etaMs / 1000)
  if (secs < 60)
    return t('transfer.drawer.aboutNSeconds', { n: secs })
  return t('transfer.drawer.aboutNMinutes', { n: Math.round(secs / 60) })
}

function toggleExpand() {
  isExpanded.value = !isExpanded.value
}

function handleCancel(id: string) {
  transferStore.cancelJob(id)
}

function handleShow() {
  isExpanded.value = false
  router.push('/transfer')
}

function togglePin(id: string) {
  const newSet = new Set(pinnedJobIds.value)
  if (newSet.has(id))
    newSet.delete(id)
  else newSet.add(id)
  pinnedJobIds.value = newSet
}

function handleDismiss(id: string) {
  transferStore.dismissJob(id)
}

watch(jobs, (newJobs, oldJobs) => {
  newJobs.forEach((job) => {
    const oldJob = oldJobs?.find(j => j.id === job.id)
    if ((!oldJob || oldJob.status !== 'completed') && job.status === 'completed') {
      // TODO: notification plugin - trigger OS notification if app unfocused
      setTimeout(() => {
        if (!pinnedJobIds.value.has(job.id)) {
          transferStore.dismissJob(job.id)
        }
      }, 5000)
    }
  })
}, { deep: true })
</script>

<template>
  <div
    v-if="!isHidden"
    class="flex w-full pointer-events-none transition-all duration-300 ease-out bottom-0 left-0 right-0 justify-center absolute z-[100]"
    :class="isExpanded ? 'h-[240px]' : 'h-8'"
  >
    <!-- Drawer Container -->
    <div
      class="text-card-foreground border-t bg-card flex flex-col h-full w-full pointer-events-auto shadow-[0_-4px_12px_rgba(0,0,0,0.05)] transition-all duration-300 ease-out relative overflow-hidden"
    >
      <!-- Collapsed Bar -->
      <div
        v-if="!isExpanded"
        class="group px-4 flex h-8 w-full cursor-pointer transition-colors items-center justify-center relative hover:bg-muted/50"
        @click="toggleExpand"
      >
        <!-- Tiny Progress Bar along top edge -->
        <div class="bg-primary h-[2px] transition-all duration-300 ease-out left-0 top-0 absolute" :style="{ width: `${aggregatePercent}%` }" />

        <span class="text-xs font-medium transition-colors group-hover:text-foreground/80">
          {{ runningJobCount > 0
            ? t('transfer.drawer.collapsed', { n: runningJobCount, percent: aggregatePercent })
            : `${t('transfer.drawer.completed')} · ${t('transfer.drawer.dismiss')}` }}
        </span>
      </div>

      <!-- Expanded Panel -->
      <div v-else class="bg-background/50 flex flex-col h-full w-full backdrop-blur-sm">
        <!-- Header -->
        <div class="px-4 py-2 border-b bg-card flex items-center justify-between">
          <span class="text-sm font-semibold">{{ t('transfer.drawer.jobsRunning', { n: runningJobCount }) }}</span>
          <Button variant="ghost" size="icon" class="h-6 w-6" @click="toggleExpand">
            <span class="i-carbon-chevron-down" />
          </Button>
        </div>

        <!-- Job List -->
        <div class="p-3 bg-muted/10 flex-1 overflow-y-auto space-y-2">
          <div
            v-for="job in jobs"
            :key="job.id"
            class="p-3 border rounded-lg flex flex-col gap-2 shadow-sm transition-colors duration-500"
            :class="job.status === 'completed' ? 'border-green-500/30 bg-green-500/5' : 'bg-card'"
          >
            <div class="flex items-center justify-between">
              <div class="flex gap-2 items-center">
                <span class="text-sm font-medium">{{ job.name }}</span>
                <Badge variant="outline" class="text-[10px] font-semibold px-1.5 h-5 uppercase">
                  {{ t(`transfer.scope.${job.scope}`) }}
                </Badge>
              </div>
              <div class="flex gap-2 items-center">
                <Badge :variant="getStatusVariant(job.status)" class="text-[10px] font-semibold px-1.5 h-5 uppercase shadow-none">
                  {{ t(`transfer.drawer.${job.status}`) }}
                </Badge>

                <!-- Actions -->
                <div class="flex gap-0.5 items-center -mr-1">
                  <Button
                    v-if="['queued', 'running'].includes(job.status)"
                    variant="ghost" size="sm" class="text-xs px-2 h-6"
                    @click="handleCancel(job.id)"
                  >
                    {{ t('transfer.drawer.cancel') }}
                  </Button>
                  <Button
                    variant="ghost" size="icon" class="rounded-md h-6 w-6"
                    :class="pinnedJobIds.has(job.id) ? 'text-primary' : 'text-muted-foreground'"
                    :title="t('transfer.drawer.pin')"
                    @click="togglePin(job.id)"
                  >
                    <span class="i-carbon-pin" />
                  </Button>
                  <Button
                    variant="ghost" size="icon" class="rounded-md h-6 w-6"
                    :title="t('transfer.drawer.dismiss')"
                    @click="handleDismiss(job.id)"
                  >
                    <span class="i-carbon-close" />
                  </Button>
                </div>
              </div>
            </div>

            <!-- Progress -->
            <div class="flex gap-3 items-center">
              <div class="rounded-full bg-secondary flex-1 h-1.5 overflow-hidden">
                <div
                  class="h-full transition-all duration-300 ease-out"
                  :class="[
                    job.status === 'completed' ? 'bg-green-500'
                    : job.status === 'failed' ? 'bg-destructive'
                      : job.status === 'cancelled' ? 'bg-muted-foreground' : 'bg-primary',
                  ]"
                  :style="{ width: `${job.progress.total > 0 ? (job.progress.current / job.progress.total) * 100 : 0}%` }"
                />
              </div>
              <span class="text-xs text-muted-foreground font-medium text-right w-10 tabular-nums">
                {{ job.progress.total > 0 ? Math.round((job.progress.current / job.progress.total) * 100) : 0 }}%
              </span>
            </div>

            <!-- Footer (ETA & Show button) -->
            <div class="mt-0.5 flex items-center justify-between">
              <span class="text-xs text-muted-foreground font-medium h-4">
                {{ formatEta(job.progress.etaMs) }}
              </span>
              <Button variant="link" size="sm" class="text-xs font-medium p-0 h-4" @click="handleShow">
                {{ t('transfer.drawer.show') }}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@media (prefers-reduced-motion: reduce) {
  * {
    transition: none !important;
  }
}
</style>
