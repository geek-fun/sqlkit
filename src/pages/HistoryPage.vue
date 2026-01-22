<script setup lang="ts">
import AppLayout from '@/components/layout/AppLayout.vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
</script>

<template>
  <AppLayout>
    <div class="p-6 h-full">
      <div class="space-y-6">
        <!-- Page Header -->
        <div class="flex gap-3 items-center">
          <h1 class="text-xl font-semibold">
            History
          </h1>
          <span class="text-muted-foreground">|</span>
          <span class="text-sm text-muted-foreground">View and manage your query history</span>
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
    </div>
  </AppLayout>
</template>
