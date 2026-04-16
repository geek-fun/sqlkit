<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { Card, CardContent } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'

import GenerateDdl from './GenerateDdl.vue'
import RunSqlFile from './RunSqlFile.vue'

const { t } = useI18n()

const activeSubTab = ref('generate')
</script>

<template>
  <Card>
    <CardContent class="pt-6">
      <Tabs v-model="activeSubTab" class="w-full">
        <TabsList class="grid grid-cols-2 max-w-md w-full">
          <TabsTrigger value="generate">
            {{ t('pages.transfer.structure.generateDdl') }}
          </TabsTrigger>
          <TabsTrigger value="runSql">
            {{ t('pages.transfer.structure.runSqlFile') }}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="generate" class="mt-6">
          <GenerateDdl />
        </TabsContent>

        <TabsContent value="runSql" class="mt-6">
          <RunSqlFile />
        </TabsContent>
      </Tabs>
    </CardContent>
  </Card>
</template>
