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
            Import / Export
          </h1>
          <span class="text-muted-foreground">|</span>
          <span class="text-sm text-muted-foreground">Import and export your data</span>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>Data Import / Export</CardTitle>
            <CardDescription>
              Import and export database schemas and data
            </CardDescription>
          </CardHeader>
          <CardContent>
            <p class="text-muted-foreground">
              Import and export functionality will be available in a future update.
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  </AppLayout>
</template>
