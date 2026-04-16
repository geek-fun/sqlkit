<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import ExportWizard from '@/components/transfer/export/ExportWizard.vue'
import ImportWizard from '@/components/transfer/import/ImportWizard.vue'
import MigrationWizard from '@/components/transfer/migration/MigrationWizard.vue'
import StructureWizard from '@/components/transfer/structure/StructureWizard.vue'
import TaskManagerButton from '@/components/transfer/tasks/TaskManagerButton.vue'
import TaskManagerPanel from '@/components/transfer/tasks/TaskManagerPanel.vue'

import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'

import { useTransferStore } from '@/store/transferStore'

const { t } = useI18n()
const transferStore = useTransferStore()

const showTaskManager = ref(false)

const activeTab = computed({
  get: () => transferStore.activeTab,
  set: val => transferStore.setActiveTab(val),
})
</script>

<template>
  <div class="flex flex-col h-full">
    <header class="px-6 py-4 border-b flex items-center justify-between">
      <div>
        <h1 class="text-xl font-semibold">
          {{ t('pages.transfer.title') }}
        </h1>
        <p class="text-sm text-muted-foreground">
          {{ t('pages.transfer.subtitle') }}
        </p>
      </div>
      <TaskManagerButton @click="showTaskManager = true" />
    </header>

    <div class="p-6 flex-1 overflow-auto">
      <Tabs v-model="activeTab" class="w-full">
        <TabsList class="grid grid-cols-4 w-full">
          <TabsTrigger value="export">
            {{ t('pages.transfer.tabs.export') }}
          </TabsTrigger>
          <TabsTrigger value="import">
            {{ t('pages.transfer.tabs.import') }}
          </TabsTrigger>
          <TabsTrigger value="structure">
            {{ t('pages.transfer.tabs.structure') }}
          </TabsTrigger>
          <TabsTrigger value="migration">
            {{ t('pages.transfer.tabs.migration') }}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="export" class="mt-6">
          <ExportWizard />
        </TabsContent>

        <TabsContent value="import" class="mt-6">
          <ImportWizard />
        </TabsContent>

        <TabsContent value="structure" class="mt-6">
          <StructureWizard />
        </TabsContent>

        <TabsContent value="migration" class="mt-6">
          <MigrationWizard />
        </TabsContent>
      </Tabs>
    </div>

    <TaskManagerPanel
      v-model:open="showTaskManager"
    />
  </div>
</template>
