<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { openUpgradeDialog } from '@/components/upgrade/upgradeDialogService'
import { useEntitlementStore } from '../../store'

type BridgeSession = {
  client: string | null
  version: string | null
  lastSeenSecs: number
}

type BridgeStatus = {
  running: boolean
  port: number | null
  configuredPort: number | null
  autoStart: boolean
  stopReason: string | null
  sessions: BridgeSession[]
}

const { t } = useI18n()
const entitlementStore = useEntitlementStore()

const status = ref<BridgeStatus | null>(null)
const loading = ref(true)
const saving = ref(false)
const loaded = ref(false)
const copied = ref(false)
const port = ref<number | undefined>()
const autoStart = ref(true)
const saveError = ref('')
const feedback = ref('')

const statusText = computed(() => {
  if (!status.value)
    return loading.value ? t('aiPermissions.loading') : t('aiPermissions.unavailable')
  return status.value.running
    ? t('pages.settings.mcp.running', { port: status.value.port })
    : t('pages.settings.mcp.stopped')
})

const portFallback = computed(() =>
  status.value?.running
  && Boolean(status.value.configuredPort)
  && status.value.port !== status.value.configuredPort)

let inFlight = false
const load = async () => {
  if (inFlight)
    return
  inFlight = true
  try {
    const data = JSON.parse(await invoke<string>('get_mcp_status')) as BridgeStatus & { configuredPort: number | null }
    status.value = data
    // Only sync the editable fields when the user is not mid-edit.
    if (!saving.value) {
      port.value = data.configuredPort ?? undefined
      autoStart.value = data.autoStart
      loaded.value = true
    }
  }
  catch {
    // Keep the last known good status; a transient failure is refreshed on the next tick.
  }
  finally {
    inFlight = false
    loading.value = false
  }
}

const saveConfig = async (restart = false, start = autoStart.value) => {
  if (saving.value || !loaded.value)
    return
  saving.value = true
  saveError.value = ''
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
    saveError.value = String(e)
  }
  finally {
    saving.value = false
  }
}

const copyUrl = async () => {
  if (!status.value?.port)
    return
  const url = `http://127.0.0.1:${status.value.port}`
  try {
    await navigator.clipboard.writeText(url)
  }
  catch {
    const ta = document.createElement('textarea')
    ta.value = url
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
  }
  copied.value = true
  setTimeout(() => (copied.value = false), 2000)
}

let poll: ReturnType<typeof setInterval> | undefined
onMounted(() => {
  load()
  poll = setInterval(load, 5000)
})
onBeforeUnmount(() => clearInterval(poll))
</script>

<template>
  <Card>
    <CardHeader>
      <div class="flex flex-wrap gap-3 items-center justify-between">
        <CardTitle>{{ t('pages.settings.mcp.title') }}</CardTitle>
        <div class="flex gap-2 items-center">
          <span role="status" class="text-xs text-muted-foreground flex gap-2 items-center">
            <span class="sr-only">{{ t('pages.settings.mcp.status') }}:</span>
            <span aria-hidden="true" class="i-lucide-plug shrink-0 h-3.5 w-3.5" />
            <span
              aria-hidden="true"
              class="rounded-full shrink-0 h-2 w-2"
              :class="status?.running ? 'bg-success' : 'bg-muted-foreground'"
            />
            {{ statusText }}
          </span>
          <TooltipProvider>
            <Tooltip v-if="status?.running">
              <TooltipTrigger as-child>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  :aria-label="t('pages.settings.mcp.copyUrl')"
                  @click="copyUrl"
                >
                  <span
                    aria-hidden="true"
                    class="h-3.5 w-3.5"
                    :class="copied ? 'i-lucide-check text-success' : 'i-lucide-copy'"
                  />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                {{ copied ? t('pages.settings.mcp.install.copied') : t('pages.settings.mcp.copyUrl') }}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </div>
    </CardHeader>
    <CardContent class="space-y-4">
      <div class="space-y-3">
        <div
          v-if="portFallback"
          class="text-xs flex gap-2 items-center"
          role="status"
        >
          <span aria-hidden="true" class="i-lucide-triangle-alert text-warning shrink-0 h-3.5 w-3.5" />
          <span class="text-warning">
            {{ t('pages.settings.mcp.portFallback', { configured: status?.configuredPort, actual: status?.port }) }}
          </span>
        </div>
        <div
          v-if="status && !status.running && status.stopReason === 'subscriptionRequired'"
          class="px-3 py-2.5 border border-warning-border rounded-md bg-warning-muted flex gap-2"
          role="status"
        >
          <span aria-hidden="true" class="i-lucide-triangle-alert text-warning mt-0.5 shrink-0 h-3.5 w-3.5" />
          <div class="min-w-0 space-y-1.5">
            <p class="text-xs leading-relaxed">
              {{ t('pages.settings.mcp.stoppedReason') }}
            </p>
            <p class="text-xs text-muted-foreground leading-relaxed">
              {{ entitlementStore.isLocalUltimate ? t('pages.settings.mcp.stoppedReasonTransient') : t('pages.settings.mcp.unlockBelow') }}
            </p>
            <Button
              v-if="entitlementStore.isLocalUltimate"
              type="button"
              variant="outline"
              size="sm"
              @click="openUpgradeDialog('mcp_bridge')"
            >
              {{ t('pages.settings.mcp.unlockAction') }}
            </Button>
          </div>
        </div>
        <div v-if="!status && !loading" class="flex gap-3 items-center">
          <p class="text-xs text-destructive-text" role="alert">
            {{ t('pages.settings.mcp.statusLoadFailed') }}
          </p>
          <Button type="button" variant="outline" size="sm" @click="load">
            {{ t('aiPermissions.retry') }}
          </Button>
        </div>
      </div>

      <div class="border-t" role="presentation" />

      <fieldset class="p-0 border-0 min-w-0 space-y-5" :disabled="loading || saving || !loaded" :aria-busy="loading || saving">
        <div v-if="saveError" role="alert" class="space-y-2">
          <p class="text-sm text-destructive-text break-words">
            {{ t('pages.settings.mcp.saveFailed') }}: {{ saveError }}
          </p>
          <Button type="button" variant="outline" size="sm" :disabled="loading || saving" @click="load">
            {{ t('aiPermissions.retry') }}
          </Button>
        </div>
        <p v-if="feedback" role="status" class="text-xs text-muted-foreground">
          {{ feedback }}
        </p>
        <div class="flex gap-4 items-center justify-between">
          <div class="min-w-0 space-y-1">
            <Label for="mcp-port">{{ t('pages.settings.mcp.port') }}</Label>
            <p id="mcp-port-help" class="text-xs text-muted-foreground leading-relaxed">
              {{ t('pages.settings.mcp.portDesc') }}
            </p>
          </div>
          <div class="flex shrink-0 flex-wrap gap-2 items-center">
            <Input id="mcp-port" type="number" :model-value="port" placeholder="9121" min="1024" max="65535" class="w-28" aria-describedby="mcp-port-help" @update:model-value="port = $event === '' ? undefined : Number($event)" />
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger as-child>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    :aria-label="t('pages.settings.mcp.restart')"
                    :disabled="loading || saving || !loaded"
                    @click="saveConfig(true)"
                  >
                    <span aria-hidden="true" class="i-lucide-refresh-cw h-3.5 w-3.5" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{{ t('pages.settings.mcp.restart') }}</TooltipContent>
              </Tooltip>
            </TooltipProvider>
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
</template>
