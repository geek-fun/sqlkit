<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import AppLayout from '@/components/layout/AppLayout.vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

const { t } = useI18n()
</script>

<template>
  <AppLayout>
    <div class="p-6 h-full">
      <div class="space-y-6">
        <!-- Page Header -->
        <div class="flex gap-3 items-center">
          <h1 class="text-xl font-semibold">
            {{ t('pages.dataStudio.title') }}
          </h1>
          <span class="text-muted-foreground">|</span>
          <span class="text-sm text-muted-foreground">{{ t('pages.dataStudio.subtitle') }}</span>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>{{ t('pages.dataStudio.card.title') }}</CardTitle>
            <CardDescription>
              {{ t('pages.dataStudio.card.description') }}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <p class="text-muted-foreground">
              {{ t('pages.dataStudio.card.comingSoon') }}
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  </AppLayout>
</template>
