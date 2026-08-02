<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'

const { t } = useI18n()

const status = ref<{ running: boolean, port: number | null }>({ running: false, port: null })
const portValue = ref<number | undefined>(undefined)
const autoStart = ref(true)
const loading = ref(false)

async function refreshStatus() {
  const raw = await invoke<string>('get_mcp_status')
  const data = JSON.parse(raw)
  status.value = { running: data.running, port: data.port ?? null }
  portValue.value = data.configuredPort ?? undefined
  autoStart.value = data.autoStart
}

onMounted(async () => {
  try {
    await refreshStatus()
  }
  catch (e) {
    console.error('Failed to get MCP status:', e)
  }
})

function onPortChange(val: string | number) {
  const num = Number(val)
  portValue.value = Number.isInteger(num) && num > 0 ? num : undefined
}

async function onAutoStartChange(val: boolean) {
  autoStart.value = val
  try {
    await invoke('save_mcp_config', { port: portValue.value ?? null, autoStart: val })
  }
  catch (e) {
    console.error('Failed to save MCP config:', e)
  }
}

async function restartBridge() {
  loading.value = true
  try {
    await invoke('save_mcp_config', { port: portValue.value ?? null, autoStart: autoStart.value })
    await refreshStatus()
  }
  catch (e) {
    console.error('Failed to restart MCP bridge:', e)
  }
  finally {
    loading.value = false
  }
}
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>{{ t('pages.settings.mcp.title') }}</CardTitle>
      <CardDescription>{{ t('pages.settings.mcp.portDesc') }}</CardDescription>
    </CardHeader>
    <CardContent class="space-y-5">
      <!-- Status -->
      <div class="flex items-center justify-between">
        <Label>{{ t('pages.settings.mcp.status') }}</Label>
        <div class="flex gap-2 items-center">
          <span
            class="rounded-full h-2.5 w-2.5"
            :class="status.running ? 'bg-green-500' : 'bg-red-500'"
          />
          <span class="text-sm text-muted-foreground">
            {{
              status.running
                ? t('pages.settings.mcp.running', { port: status.port })
                : t('pages.settings.mcp.stopped')
            }}
          </span>
        </div>
      </div>

      <!-- Port + Restart -->
      <div class="flex items-center justify-between">
        <div class="space-y-1">
          <Label>{{ t('pages.settings.mcp.port') }}</Label>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.mcp.portDesc') }}
          </p>
        </div>
        <div class="flex gap-3 items-center">
          <Input
            type="number"
            :model-value="portValue"
            min="1024"
            max="65535"
            class="w-28"
            @update:model-value="onPortChange($event)"
          />
          <Button
            variant="outline"
            size="sm"
            :disabled="loading"
            @click="restartBridge"
          >
            {{ t('pages.settings.mcp.restart') }}
          </Button>
        </div>
      </div>

      <!-- Auto-start -->
      <div class="flex items-center justify-between">
        <div class="space-y-1">
          <Label>{{ t('pages.settings.mcp.autoStart') }}</Label>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.mcp.autoStartDesc') }}
          </p>
        </div>
        <Switch
          :checked="autoStart"
          @update:checked="onAutoStartChange"
        />
      </div>
    </CardContent>
  </Card>
</template>
