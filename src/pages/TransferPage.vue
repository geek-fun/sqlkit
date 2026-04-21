<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import AppLayout from '@/components/layout/AppLayout.vue'
import ExportWizard from '@/components/transfer/export/ExportWizard.vue'

import ImportWizard from '@/components/transfer/import/ImportWizard.vue'

import MigrationWizard from '@/components/transfer/migration/MigrationWizard.vue'
import StructureWizard from '@/components/transfer/structure/StructureWizard.vue'
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
  { value: 'export', label: t('pages.transfer.tabs.export'), icon: 'i-carbon-document-export' },
  { value: 'import', label: t('pages.transfer.tabs.import'), icon: 'i-carbon-document-import' },
  { value: 'structure', label: t('pages.transfer.tabs.structure'), icon: 'i-carbon-data-base' },
  { value: 'migration', label: t('pages.transfer.tabs.migration'), icon: 'i-carbon-data-refinery' },
])
</script>

<template>
  <AppLayout>
    <div class="bg-background flex flex-col h-full">
      <!-- Header -->
      <header class="px-6 py-4 border-b flex shrink-0 items-center justify-between">
        <div class="flex gap-3 items-center">
          <h1 class="text-lg tracking-tight font-semibold">
            {{ t('pages.transfer.title') }}
          </h1>
          <p class="text-sm text-muted-foreground">
            {{ t('pages.transfer.subtitle') }}
          </p>
        </div>
        <TaskManagerButton @click="showTaskManager = true" />
      </header>

      <!-- Content Layout -->
      <div class="px-4 py-3 flex flex-1 min-h-0">
        <!-- Left: Steps Column -->
        <div class="flex flex-1 flex-col gap-3 min-h-0 min-w-0">
          <!-- Segmented Control for Mode Switch -->
          <div class="flex shrink-0 justify-start">
            <Tabs v-model="activeTab" class="w-auto">
              <TabsList class="p-1 border rounded-lg bg-muted/50 gap-1 h-9">
                <TabsTrigger
                  v-for="tab in tabConfig"
                  :key="tab.value"
                  :value="tab.value"
                  class="text-sm font-medium px-4 rounded-md flex gap-2 h-full transition-colors items-center justify-center data-[state=active]:text-primary data-[state=active]:bg-background data-[state=active]:shadow-sm"
                >
                  <span :class="tab.icon" class="h-4 w-4" />
                  <span class="truncate">{{ tab.label }}</span>
                </TabsTrigger>
              </TabsList>
            </Tabs>
          </div>

          <!-- Steps Content (scrollable) -->
          <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border pr-2 flex flex-1 flex-col gap-3 overflow-y-auto">
            <ExportWizard v-if="activeTab === 'export'" />
            <ImportWizard v-if="activeTab === 'import'" />
            <StructureWizard v-if="activeTab === 'structure'" />
            <MigrationWizard v-if="activeTab === 'migration'" />
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
