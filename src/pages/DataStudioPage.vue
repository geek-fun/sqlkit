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
            Data Studio
          </h1>
          <span class="text-muted-foreground">|</span>
          <span class="text-sm text-muted-foreground">AI-powered data analysis</span>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>AI Data Studio</CardTitle>
            <CardDescription>
              Use AI to analyze and query your data
            </CardDescription>
          </CardHeader>
          <CardContent>
            <p class="text-muted-foreground">
              AI-powered data studio will be available in a future update.
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  </AppLayout>
</template>
