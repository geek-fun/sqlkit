<script setup lang="ts">
import AppLayout from '@/components/layout/AppLayout.vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div>
        <h1 class="text-3xl tracking-tight font-bold">
          Query History
        </h1>
        <p class="text-muted-foreground mt-2">
          View and manage your query history
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Recent Queries</CardTitle>
          <CardDescription>
            Your recently executed queries
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p class="text-muted-foreground">
            No query history available yet.
          </p>
        </CardContent>
      </Card>
    </div>
  </AppLayout>
</template>
