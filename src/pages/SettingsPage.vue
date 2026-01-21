<script setup lang="ts">
import { ref } from 'vue'
import AppLayout from '@/components/layout/AppLayout.vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

const theme = ref('system')
</script>

<template>
  <AppLayout>
    <div class="p-6 h-full">
      <div class="space-y-6">
        <!-- Page Header -->
        <div class="flex gap-3 items-center">
          <h1 class="text-xl font-semibold">
            Settings
          </h1>
          <span class="text-muted-foreground">|</span>
          <span class="text-sm text-muted-foreground">Configure your application preferences</span>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>Appearance</CardTitle>
            <CardDescription>
              Customize the look and feel of the application
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="space-y-2">
              <Label>Theme</Label>
              <Select v-model="theme">
                <SelectTrigger class="w-48">
                  <SelectValue placeholder="Select theme" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="light">
                    Light
                  </SelectItem>
                  <SelectItem value="dark">
                    Dark
                  </SelectItem>
                  <SelectItem value="system">
                    System
                  </SelectItem>
                </SelectContent>
              </Select>
              <p class="text-sm text-muted-foreground">
                Choose between light, dark, or system theme
              </p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Preferences</CardTitle>
            <CardDescription>
              Application settings
            </CardDescription>
          </CardHeader>
          <CardContent>
            <p class="text-muted-foreground">
              Additional settings will be available in a future update.
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  </AppLayout>
</template>
