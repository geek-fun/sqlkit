<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import AppLayout from '@/components/layout/AppLayout.vue'
import SQLEditor from '@/components/SQLEditor.vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

const { t } = useI18n()

const sqlQuery = `-- Sample SQL Query
SELECT 
  u.id,
  u.name,
  u.email,
  COUNT(o.id) as order_count
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
WHERE u.created_at > '2024-01-01'
GROUP BY u.id, u.name, u.email
ORDER BY order_count DESC
LIMIT 10;`

function handleExecuteQuery() {
  // TODO: Implement query execution logic
}
</script>

<template>
  <AppLayout>
    <div class="p-6 h-full">
      <div class="space-y-6">
        <!-- Page Header -->
        <div class="flex gap-3 items-center">
          <h1 class="text-xl font-semibold">
            {{ t('pages.queries.title') }}
          </h1>
          <span class="text-muted-foreground">|</span>
          <span class="text-sm text-muted-foreground">{{ t('pages.queries.subtitle') }}</span>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>{{ t('pages.queries.editor.title') }}</CardTitle>
            <CardDescription>
              {{ t('pages.queries.editor.description') }}
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <SQLEditor
              :model-value="sqlQuery"
              height="400px"
              dialect="sql"
              @execute="handleExecuteQuery"
            />
            <div class="flex gap-2">
              <Button @click="handleExecuteQuery">
                {{ t('pages.queries.editor.execute') }}
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  </AppLayout>
</template>
