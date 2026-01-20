<script setup lang="ts">
import AppLayout from '@/components/layout/AppLayout.vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div>
        <h1 class="text-3xl tracking-tight font-bold">
          Settings
        </h1>
        <p class="text-muted-foreground mt-2">
          Configure your application preferences
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Preferences</CardTitle>
          <CardDescription>
            Application settings
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p class="text-muted-foreground">
            Settings will be available in a future update.
          </p>
        </CardContent>
      </Card>
    </div>
  </AppLayout>
</template>
