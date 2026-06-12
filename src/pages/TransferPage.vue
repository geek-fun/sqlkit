<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import AppLayout from '@/components/layout/AppLayout.vue'
import ExportWizard from '@/components/transfer/export/ExportWizard.vue'
import ImportWizard from '@/components/transfer/import/ImportWizard.vue'
import TaskManagerButton from '@/components/transfer/tasks/TaskManagerButton.vue'
import TaskManagerPanel from '@/components/transfer/tasks/TaskManagerPanel.vue'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useTransferStore } from '@/store/transferStore'

const { t } = useI18n()
const transferStore = useTransferStore()

const showTaskManager = ref(false)

const activeTab = computed({
  get: () => transferStore.activeTab,
  set: val => transferStore.setActiveTab(val),
})

const tabConfig = computed(() => [
  { value: 'export', label: t('transfer.tabs.export'), icon: 'i-carbon-document-export' },
  { value: 'import', label: t('transfer.tabs.import'), icon: 'i-carbon-document-import' },
])
</script>

<template>
  <AppLayout>
    <div class="bg-background flex flex-col h-full">
      <!-- Header -->
      <header class="px-6 py-3 border-b border-border/40 flex shrink-0 items-center justify-between">
        <div class="flex gap-3 items-center">
          <h1 class="text-base tracking-tight font-semibold">
            {{ t('transfer.title') }}
          </h1>
          <p class="text-[11px] text-muted-foreground">
            {{ t('transfer.subtitle') }}
          </p>
        </div>
        <TaskManagerButton @click="showTaskManager = true" />
      </header>

      <!-- Content Layout -->
      <div class="px-3 py-2.5 flex flex-1 min-h-0">
        <!-- Left: Steps Column -->
        <div class="flex flex-1 flex-col gap-2.5 min-h-0 min-w-0">
          <!-- Segmented Control for Mode Switch -->
          <div class="flex shrink-0 justify-start">
            <Tabs v-model="activeTab" class="w-auto">
              <TabsList class="p-1 border border-border/40 rounded-lg bg-muted/50 gap-1 h-9">
                <TabsTrigger
                  v-for="tab in tabConfig"
                  :key="tab.value"
                  :value="tab.value"
                  class="text-xs font-medium px-4 rounded-md flex gap-2 h-full transition-colors items-center justify-center data-[state=active]:text-primary data-[state=active]:bg-background data-[state=active]:shadow-sm"
                >
                  <span :class="tab.icon" class="h-3.5 w-3.5" />
                  <span class="truncate">{{ tab.label }}</span>
                </TabsTrigger>
              </TabsList>
            </Tabs>
          </div>

          <!-- Steps Content (scrollable) -->
          <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border pr-2 flex flex-1 flex-col gap-3 overflow-y-auto">
            <ExportWizard v-if="activeTab === 'export'" />
            <ImportWizard v-else-if="activeTab === 'import'" />
          </div>
        </div>
      </div>

      <!-- Task Manager Panel -->
      <TaskManagerPanel v-model:open="showTaskManager" />
    </div>
  </AppLayout>
</template>

<style scoped>
/* Custom scrollbar fallback if tailwind-scrollbar is not available */
.scrollbar-thin {
  scrollbar-width: thin;
}
.scrollbar-track-transparent::-webkit-scrollbar-track {
  background: transparent;
}
.scrollbar-thumb-border::-webkit-scrollbar-thumb {
  background: hsl(var(--border));
  border-radius: 9999px;
}
.scrollbar-thin::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
</style>
