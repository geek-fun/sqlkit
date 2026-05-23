<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import AppLayout from '@/components/layout/AppLayout.vue'
import { JobsDrawer, TransferLauncher } from '@/components/transfer/launcher'

const { t } = useI18n()
</script>

<template>
  <AppLayout>
    <div class="bg-background flex flex-col h-full relative overflow-hidden">
      <header class="px-6 py-4 border-b border-border/40 bg-card flex shrink-0 items-center justify-between">
        <div>
          <h1 class="text-xl tracking-tight font-semibold">
            {{ t('transfer.launcher.title') }}
          </h1>
        </div>
      </header>

      <div class="p-6 flex flex-1 flex-col min-h-0 overflow-y-auto">
        <TransferLauncher />
      </div>

      <JobsDrawer />
    </div>
  </AppLayout>
</template>
