<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useDataStudioStore } from '@/store/dataStudioStore'

const emit = defineEmits<{
  select: [sessionId: string]
  delete: [sessionId: string]
  newSession: []
  close: []
}>()

const { t } = useI18n()
const dataStudioStore = useDataStudioStore()

function formatDate(timestamp: number): string {
  const d = new Date(timestamp)
  const now = new Date()
  const diff = now.getTime() - d.getTime()
  if (diff < 60000)
    return t('dataStudio.history.justNow')
  if (diff < 3600000)
    return t('dataStudio.history.minutesAgo', { count: Math.floor(diff / 60000) })
  if (diff < 86400000)
    return t('dataStudio.history.hoursAgo', { count: Math.floor(diff / 3600000) })
  return d.toLocaleDateString()
}
</script>

<template>
  <div class="bg-background flex flex-col h-full">
    <div class="px-4 py-3 border-b border-border flex items-center justify-between">
      <span class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">
        {{ t('dataStudio.history.title') }}
      </span>
      <div class="flex gap-1 items-center">
        <button
          class="i-carbon-add text-muted-foreground rounded inline-flex h-6 w-6 transition-colors items-center justify-center hover:text-foreground hover:bg-muted"
          :title="t('dataStudio.history.newSession')"
          @click="emit('newSession')"
        />
        <button
          class="i-carbon-close text-muted-foreground rounded inline-flex h-6 w-6 transition-colors items-center justify-center hover:text-foreground hover:bg-muted"
          :title="t('common.buttons.close')"
          @click="emit('close')"
        />
      </div>
    </div>
    <div class="flex-1 overflow-y-auto">
      <div v-if="dataStudioStore.sessions.length === 0" class="text-xs text-muted-foreground p-4 text-center">
        {{ t('dataStudio.history.noSessions') }}
      </div>
      <button
        v-for="session in dataStudioStore.sessions"
        :key="session.id"
        class="px-4 py-3 text-left border-b border-border/50 w-full transition-colors hover:bg-muted/50"
        :class="{ 'bg-muted/30': session.id === dataStudioStore.activeSessionId || session.id === dataStudioStore.sidebarSessionId }"
        @click="emit('select', session.id)"
      >
        <div class="text-sm text-foreground font-medium truncate">
          {{ session.title }}
        </div>
        <div class="text-xs text-muted-foreground mt-0.5">
          {{ session.messages.length }} {{ t('dataStudio.history.messages') }}
          · {{ formatDate(session.updated_at) }}
        </div>
        <div class="mt-1 flex gap-1 items-center">
          <span
            class="text-[10px] px-1.5 py-0.5 rounded-full"
            :class="session.status === 'running' ? 'bg-green-500/10 text-green-600' : 'bg-muted text-muted-foreground'"
          >
            {{ session.status }}
          </span>
          <span v-if="session.sources.length > 0" class="text-[10px] text-muted-foreground">
            {{ session.sources.length }} {{ t('dataStudio.history.sources') }}
          </span>
        </div>
      </button>
    </div>
  </div>
</template>
