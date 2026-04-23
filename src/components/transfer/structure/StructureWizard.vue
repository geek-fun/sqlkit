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
  <div class="flex flex-1 flex-col gap-4 min-h-0">
    <!-- Mode Switch -->
    <div class="flex shrink-0 justify-start">
      <Tabs v-model="activeSubTab" class="w-auto">
        <TabsList class="p-1 border border-border/40 rounded-lg bg-muted/50 flex gap-1 h-9 w-auto items-center justify-center">
          <TabsTrigger
            value="generate"
            class="text-xs text-muted-foreground font-medium px-3 rounded-md border-none bg-transparent flex gap-1.5 h-full cursor-pointer shadow-none transition-all duration-200 items-center justify-center data-[state=active]:text-primary hover:text-foreground data-[state=active]:font-semibold data-[state=active]:bg-background data-[state=active]:shadow-sm"
          >
            <span class="i-carbon-data-base h-3.5 w-3.5" />
            <span>{{ t('transfer.structure.generateDdl') }}</span>
          </TabsTrigger>
          <TabsTrigger
            value="runSql"
            class="text-xs text-muted-foreground font-medium px-3 rounded-md border-none bg-transparent flex gap-1.5 h-full cursor-pointer shadow-none transition-all duration-200 items-center justify-center data-[state=active]:text-primary hover:text-foreground data-[state=active]:font-semibold data-[state=active]:bg-background data-[state=active]:shadow-sm"
          >
            <span class="i-carbon-document h-3.5 w-3.5" />
            <span>{{ t('transfer.structure.runSqlFile') }}</span>
          </TabsTrigger>
        </TabsList>
      </Tabs>
    </div>

    <!-- Content -->
    <div class="flex flex-1 flex-col min-h-0">
      <GenerateDdl v-if="activeSubTab === 'generate'" />
      <RunSqlFile v-if="activeSubTab === 'runSql'" />
    </div>
  </div>
</template>
