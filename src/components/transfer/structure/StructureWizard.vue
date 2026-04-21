<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'

import GenerateDdl from './GenerateDdl.vue'
import RunSqlFile from './RunSqlFile.vue'

const { t } = useI18n()

const activeSubTab = ref('generate')
</script>

<template>
  <div class="pb-8 flex flex-col gap-8">
    <!-- Mode Switch -->
    <div class="flex shrink-0 justify-start">
      <Tabs v-model="activeSubTab" class="w-auto">
        <TabsList class="p-1 border border-border rounded-md bg-muted flex gap-1 h-8 w-auto items-center justify-center">
          <TabsTrigger
            value="generate"
            class="text-[13px] text-muted-foreground font-medium px-3 rounded-sm border-none bg-transparent flex gap-1.5 h-full cursor-pointer shadow-none transition-all duration-200 items-center justify-center data-[state=active]:text-primary hover:text-foreground data-[state=active]:font-semibold data-[state=active]:bg-card data-[state=active]:shadow-sm"
          >
            <span class="i-carbon-data-base h-4 w-4" />
            <span>{{ t('pages.transfer.structure.generateDdl') }}</span>
          </TabsTrigger>
          <TabsTrigger
            value="runSql"
            class="text-[13px] text-muted-foreground font-medium px-3 rounded-sm border-none bg-transparent flex gap-1.5 h-full cursor-pointer shadow-none transition-all duration-200 items-center justify-center data-[state=active]:text-primary hover:text-foreground data-[state=active]:font-semibold data-[state=active]:bg-card data-[state=active]:shadow-sm"
          >
            <span class="i-carbon-document h-4 w-4" />
            <span>{{ t('pages.transfer.structure.runSqlFile') }}</span>
          </TabsTrigger>
        </TabsList>
      </Tabs>
    </div>

    <!-- Content -->
    <div class="flex flex-col gap-3">
      <GenerateDdl v-if="activeSubTab === 'generate'" />
      <RunSqlFile v-if="activeSubTab === 'runSql'" />
    </div>
  </div>
</template>
