<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { DefaultPermissionEditor } from '@/components/ai-permissions'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'

const { t } = useI18n()
const status = ref<{ running: boolean, port: number | null }>({ running: false, port: null })
const port = ref<number | undefined>()
const autoStart = ref(true)
const loading = ref(true)
const saving = ref(false)
const loaded = ref(false)
const error = ref('')
const feedback = ref('')
const load = async () => {
  loading.value = true
  error.value = ''
  try {
    const data = JSON.parse(await invoke<string>('get_mcp_status'))
    status.value = { running: data.running, port: data.port ?? null }
    port.value = data.configuredPort ?? undefined
    autoStart.value = data.autoStart
    loaded.value = true
  }
  catch (e) { error.value = String(e) }
  finally { loading.value = false }
}
onMounted(load)
const saveConfig = async (restart = false, start = autoStart.value) => {
  if (saving.value || !loaded.value)
    return
  saving.value = true
  error.value = ''
  feedback.value = ''
  const previous = autoStart.value
  try {
    await invoke('save_mcp_config', { port: port.value ?? null, autoStart: start, restart })
    autoStart.value = start
    feedback.value = t(restart ? 'pages.settings.mcp.restartSuccess' : 'pages.settings.feedback.saved')
    await load()
  }
  catch (e) {
    autoStart.value = previous
    error.value = String(e)
  }
  finally { saving.value = false }
}
</script>

<template>
  <div class="space-y-6">
    <Card>
      <CardHeader>
        <div class="flex flex-wrap gap-3 items-center justify-between">
          <CardTitle>{{ t('pages.settings.mcp.title') }}</CardTitle>
          <span role="status" class="text-xs text-muted-foreground flex gap-2 items-center">
            <span class="sr-only">{{ t('pages.settings.mcp.status') }}:</span>
            <span aria-hidden="true" class="rounded-full shrink-0 h-2 w-2" :class="status.running && !loading && loaded ? 'bg-success' : 'bg-muted-foreground'" />
            {{ loading ? t('aiPermissions.loading') : !loaded ? t('aiPermissions.unavailable') : status.running ? t('pages.settings.mcp.running', { port: status.port }) : t('pages.settings.mcp.stopped') }}
          </span>
        </div>
      </CardHeader>
      <CardContent class="space-y-6">
        <div v-if="error" role="alert" class="space-y-2">
          <p class="text-sm text-destructive-text break-words">
            {{ t('pages.settings.mcp.saveFailed') }}: {{ error }}
          </p>
          <Button type="button" variant="outline" size="sm" :disabled="loading || saving" @click="load">
            {{ t('aiPermissions.retry') }}
          </Button>
        </div>
        <p v-if="feedback" role="status" class="text-xs text-muted-foreground">
          {{ feedback }}
        </p>
        <fieldset class="p-0 border-0 max-w-lg min-w-0 space-y-5" :disabled="loading || saving || !loaded" :aria-busy="loading || saving">
          <div class="space-y-2">
            <Label for="mcp-port">{{ t('pages.settings.mcp.port') }}</Label>
            <p id="mcp-port-help" class="text-xs text-muted-foreground leading-relaxed">
              {{ t('pages.settings.mcp.portDesc') }}
            </p>
            <div class="flex flex-wrap gap-2 items-center">
              <Input id="mcp-port" type="number" :model-value="port" placeholder="9121" min="1024" max="65535" class="w-28" aria-describedby="mcp-port-help" @update:model-value="port = $event === '' ? undefined : Number($event)" />
              <Button type="button" variant="outline" size="sm" :disabled="loading || saving || !loaded" @click="saveConfig(true)">
                {{ t('pages.settings.mcp.restart') }}
              </Button>
            </div>
          </div>
          <div class="flex gap-4 items-center justify-between">
            <div class="space-y-1">
              <Label for="mcp-autostart">{{ t('pages.settings.mcp.autoStart') }}</Label>
              <p id="mcp-autostart-help" class="text-xs text-muted-foreground leading-relaxed">
                {{ t('pages.settings.mcp.autoStartDesc') }}
              </p>
            </div>
            <Switch id="mcp-autostart" class="shrink-0" :checked="autoStart" :disabled="loading || saving || !loaded" aria-describedby="mcp-autostart-help" @update:checked="saveConfig(false, $event)" />
          </div>
        </fieldset>
      </CardContent>
    </Card>
    <Card>
      <CardContent class="p-6">
        <DefaultPermissionEditor />
      </CardContent>
    </Card>
  </div>
</template>
